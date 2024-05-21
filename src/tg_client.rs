use rust_tdlib::client::{Client, ClientState, ConsoleAuthStateHandler, Worker};
use rust_tdlib::client::tdlib_client::TdJson;
use rust_tdlib::types::{Chat, GetChat, GetChatHistory, GetChats, Message, MessageContent, TdlibParameters, Update};

#[derive(Clone)]
pub struct MyMeta{ pub meta: String, pub id: i64}

pub struct TgClient {
    client : Client<TdJson>,
    worker: Worker<ConsoleAuthStateHandler, TdJson>,
    files_metadata: Vec<MyMeta>,
    chat_id: i64
}

impl TgClient {
    pub async fn new(api_id: i32, api_hash: String, group_name: &str) -> TgClient {
        let (sender, _reciever) = tokio::sync::mpsc::channel::<Box<Update>>(10000);

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
                let meta_data = MyMeta{meta: text.text().text().parse().unwrap(), id: messages[i+1].id()};

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
}