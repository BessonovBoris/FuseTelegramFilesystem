use std::collections::HashMap;
use std::str::FromStr;
use clap::builder::Str;
use fuse::{FileAttr, FileType};
use rust_tdlib::client::{Client, ClientState, ConsoleAuthStateHandler, Worker};
use rust_tdlib::client::tdlib_client::TdJson;
use rust_tdlib::types::{Chat, DeleteMessages, DownloadFile, FormattedText, GetChat, GetChatHistory, GetChats, InputMessageContent, InputMessageText, Message, MessageContent, SendMessage, TdlibParameters, Update};
use serde::{Serialize, Serializer};
use time::Timespec;
use tokio::io::split;

#[derive(Clone)]
pub struct MyMeta{ pub meta: String, pub id: i32}

pub struct Block {
    pub attr: FileAttr,
    pub message_id: i32,
    pub name: String,
    pub data: &'static[u8]
}

pub struct TgClient {
    client : Client<TdJson>,
    worker: Worker<ConsoleAuthStateHandler, TdJson>,
    files_metadata: Vec<MyMeta>,
    chat_id: i64
}

impl TgClient {
    pub async fn new(api_id: i32, api_hash: String, group_name: &str) -> TgClient {
        let (sender, _receiver) = tokio::sync::mpsc::channel::<Box<Update>>(10000);

        let client = Client::builder()
            .with_tdlib_parameters(
                TdlibParameters::builder()
                    .database_directory("tddb")
                    .use_test_dc(false)
                    .api_id(api_id)
                    .api_hash(api_hash)
                    .system_language_code("en")
                    .device_model("Desktop")
                    .system_version("Unknown")
                    .application_version(env!("CARGO_PKG_VERSION"))
                    .enable_storage_optimizer(true)
                    .build(),
            )
            .with_updates_sender(sender)
            .build()
            .unwrap();

        let mut worker = Worker::builder().build().unwrap();
        worker.start();

        let client = worker.bind_client(client).await.unwrap();

        loop {
            if worker.wait_client_state(&client).await.unwrap() == ClientState::Opened {
                log::info!("authorized");
                break;
            }
        }

        let chats = client.get_chats(GetChats::builder().limit(300).build()).await.unwrap();
        let chats_ids = chats.chat_ids();
        let mut chat: Chat = Chat::default();

        for chat_id in chats_ids {
            chat = client.get_chat(GetChat::builder().chat_id(*chat_id).build()).await.unwrap();
            let name = chat.title();

            if name == group_name {
                break
            }
        }

        let chat_id = chat.id();

        let mut messages: Vec<Message> = Vec::new();
        let mut next_message_id = 0;
        let limit = 1;
        let offset = 0;

        loop {
            let history_r = client.get_chat_history(
                GetChatHistory::builder()
                    .chat_id(chat_id)
                    .only_local(false)
                    .offset(offset)
                    .from_message_id(next_message_id)
                    .limit(limit)
                    .build()).await;

            match history_r {
                Ok(history) => {
                    let mes = history.messages();

                    if mes.is_empty() {
                        break
                    }

                    let opt_mes = mes.last().unwrap();
                    match opt_mes {
                        Some(last_mes) => {
                            next_message_id = last_mes.id();
                        }
                        _ => {}
                    }

                    for me in mes.iter().flatten() {
                        messages.push(me.clone());
                    }
                }
                Err(e) => {
                    println!("Error: {}", e);
                    break
                }
            }
        }

        let mut files_metadata: Vec<MyMeta> = Vec::new();

        for i in 0..messages.len() {
            if let MessageContent::MessageText(text) = messages[i].content() {
                let mut id: i32 = 0;

                if let MessageContent::MessageDocument(doc) = messages[i+1].content() {
                    id = doc.document().document().id();
                }

                let meta_data = MyMeta{meta: text.text().text().parse().unwrap(), id};

                files_metadata.push(meta_data);
            }
        }

        TgClient { client, worker, files_metadata, chat_id }
    }

    pub async fn stop(&self) {
        self.client.stop().await.unwrap();

        loop {
            if self.worker.wait_client_state(&self.client).await.unwrap() == ClientState::Closed {
                log::info!("client closed");
                break;
            }
        }

        self.worker.stop();
        log::info!("worker stopped");

        // if Path::new("/home/kali/RustroverProjects/FuseTelegramFilesystem/src/tddb/").exists() {
        //     fs::remove_dir_all("/home/kali/RustroverProjects/FuseTelegramFilesystem/src/tddb/").expect("can't clear directory");
        // }
    }

    pub fn get_metafiles(&self) -> Vec<MyMeta> {
        self.files_metadata.clone()
    }

    pub async fn send_message(&self) {
        self.client.send_message(
            SendMessage::builder()
                .chat_id(self.chat_id)
                .input_message_content(InputMessageContent::InputMessageText(
                    InputMessageText::builder()
                        .text(FormattedText::builder().text("Message").build())
                        .build(),
                ))
                .build(),
        ).await.unwrap();
    }

    pub async fn delete_message(&self, id: i64) {
        self.client.delete_messages(
            DeleteMessages::builder()
                .chat_id(self.chat_id)
                .message_ids(vec![id])
                .build()
        ).await.unwrap();
    }

    pub async fn get_files(&self) -> HashMap<u64, Block> {
        let mut inode = 1;
        let mut map: HashMap<u64, Block> = Default::default();

        let dir_attr: FileAttr = FileAttr {
            ino: 1,
            size: 0,
            blocks: 0,
            atime: Timespec::new(0, 0),                                  // 1970-01-01 00:00:00
            mtime: Timespec::new(0, 0),
            ctime: Timespec::new(0, 0),
            crtime: Timespec::new(0, 0),
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid: 501,
            gid: 20,
            rdev: 0,
            flags: 0,
        };

        map.insert(inode, Block{attr: dir_attr, message_id: -1, name: String::from_str(".").unwrap(), data: &[]});

        let v = self.get_metafiles();

        for meta in v {
            inode += 1;
            let mut name: String = String::new();
            let mes_id = meta.id;
            let mut size: u64 = 0;
            let mut atime: Timespec = Timespec::new(0, 0);
            let mut mtime: Timespec = Timespec::new(0, 0);
            let mut ctime: Timespec = Timespec::new(0, 0);
            let mut crtime: Timespec = Timespec::new(0, 0);
            let mut perm: u16 = 0;
            let mut uid: u32 = 0;
            let mut gid: u32 = 0;

            let meta_string = meta.meta;

            for parts in meta_string.split("\n") {
                let split_parts = parts.split(":");

                let mut iter = split_parts.into_iter();
                let mut filed = iter.next();

                while filed != None {
                    let s = filed.unwrap();

                    if s == "name" {
                        if let Some(value) = iter.next() {
                            name = value.to_string();
                        }
                    }

                    if s == "size" {
                        if let Some(value) = iter.next() {
                            size = value.parse::<u64>().unwrap();
                        }
                    }

                    if s == "atime" {
                        if let Some(time) = iter.next() {
                            let mut time_iter = time.split(",");
                            if let (Some(secs), Some(nsec)) = (time_iter.next(), time_iter.next()) {
                                atime = Timespec::new(secs.parse::<i64>().unwrap(), nsec.parse::<i32>().unwrap());
                            }
                        }
                    }

                    if s == "mtime" {
                        if let Some(time) = iter.next() {
                            let mut time_iter = time.split(",");
                            if let (Some(secs), Some(nsec)) = (time_iter.next(), time_iter.next()) {
                                mtime = Timespec::new(secs.parse::<i64>().unwrap(), nsec.parse::<i32>().unwrap());
                            }
                        }
                    }

                    if s == "ctime" {
                        if let Some(time) = iter.next() {
                            let mut time_iter = time.split(",");
                            if let (Some(secs), Some(nsec)) = (time_iter.next(), time_iter.next()) {
                                ctime = Timespec::new(secs.parse::<i64>().unwrap(), nsec.parse::<i32>().unwrap());
                            }
                        }
                    }

                    if s == "crtime" {
                        if let Some(time) = iter.next() {
                            let mut time_iter = time.split(",");

                            if let (Some(secs), Some(nsec)) = (time_iter.next(), time_iter.next()) {
                                crtime = Timespec::new(secs.parse::<i64>().unwrap(), nsec.parse::<i32>().unwrap());
                            }
                        }
                    }

                    if s == "perms" {
                        if let Some(value) = iter.next() {
                            perm = value.parse::<u16>().unwrap();
                        }
                    }

                    if s == "uid" {
                        if let Some(value) = iter.next() {
                            uid = value.parse::<u32>().unwrap();
                        }
                    }

                    if s == "gid" {
                        if let Some(value) = iter.next() {
                            gid = value.parse::<u32>().unwrap();
                        }
                    }

                    filed = iter.next();
                }
            }

            let attr = FileAttr {
                ino: inode,
                size,
                blocks: 1,
                atime,
                mtime,
                ctime,
                crtime,
                kind: FileType::NamedPipe,
                perm,
                nlink: 0,
                uid,
                gid,
                rdev: 0,
                flags: 0,
            };

            let block = Block{attr, message_id: mes_id, name, data: &[]};
            map.insert(inode, block);
        }

        map
    }

    pub async fn get_directories(&self) -> HashMap<u64, Vec<u64>> {

        let mut map: HashMap<u64, Vec<u64>> = Default::default();
        map.insert(1u64, Vec::new());

        let v = self.get_files().await;

        for (i, block) in v.into_iter().enumerate() {
            if block.1.attr.ino > 1 {
                map.get_mut(&1u64).unwrap().push(block.1.attr.ino);
            }
        }

        map
    }

    pub async fn download_file(&self, id: i32) -> String {
        let file = self.client.download_file(
            DownloadFile::builder()
                .offset(0)
                .limit(0)
                .file_id(id)
                .priority(1)
                .build()
        ).await.unwrap();

        let l = file.local();

        loop {
            if l.is_downloading_completed() {
                break;
            }
        }

        l.path().to_string()
    }
}