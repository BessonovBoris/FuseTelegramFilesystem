use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use libc::ENOENT;
use fuse::{FileType, Filesystem, Request, ReplyData, ReplyEntry, ReplyAttr, ReplyDirectory, FileAttr};
use time::{Timespec};
use crate::tg_client::TgClient;
use crate::tg_client::Block;

const HELLO_TXT_CONTENT: &str = "Hello World!\n";

struct TgFileSystem {
    tg_client: TgClient,
    files: HashMap<u64, Block>,
    directories: HashMap<u64, Vec<u64>>,
}

impl TgFileSystem {
    pub async fn new(tg_client: TgClient) -> TgFileSystem {
        let files = tg_client.get_files().await;
        let directories = tg_client.get_directories().await;

        TgFileSystem {tg_client, files, directories}
    }
}

impl Filesystem for TgFileSystem {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if !self.directories.contains_key(&parent) {
            reply.error(ENOENT);
            return;
        }

        let entries = self.directories.get(&parent).unwrap();

        let default_block = Block {
            attr: FileAttr {
                ino: 0,
                size: 0,
                blocks: 0,
                atime: Timespec { sec: 0, nsec: 0 },
                mtime: Timespec { sec: 0, nsec: 0 },
                ctime: Timespec { sec: 0, nsec: 0 },
                crtime: Timespec { sec: 0, nsec: 0 },
                kind: FileType::NamedPipe,
                perm: 0,
                nlink: 0,
                uid: 0,
                gid: 0,
                rdev: 0,
                flags: 0,
            },
            message_id: -1,
            name: String::new(),
            data: &[],
        };

        let mut block: &Block = &default_block;

        for (_, entry) in entries.into_iter().enumerate() {
            let this_block = self.files.get(entry).unwrap();

            if this_block.name == name.to_str().unwrap() {
                block = this_block;
            }
        }

        if block.message_id == -1 {
            reply.error(ENOENT);
            return;
        }

        reply.entry(&Timespec::new(1, 0), &block.attr, 0);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        if !self.files.contains_key(&ino) {
            reply.error(ENOENT);
            return;
        }

        reply.attr(&Timespec::new(1702944000, 0), &self.files.get(&ino).unwrap().attr)
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, _offset: i64, _size: u32, reply: ReplyData) {
        if !self.files.contains_key(&ino) {
            reply.error(ENOENT);
            return;
        }

        let block = self.files.get(&ino).unwrap();
        if block.attr.kind != FileType::RegularFile {
            reply.error(ENOENT);
            return;
        }

        reply.data(block.data);
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        if !self.directories.contains_key(&ino) {
            reply.error(ENOENT);
            return;
        }

        let entries = self.directories.get(&ino).unwrap();

        let mut dir = vec![
            (ino, FileType::Directory, "."),
            (ino, FileType::Directory, ".."),
        ];

        for (_, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            if self.files.get(entry).unwrap().attr.ino == ino {
                continue
            }

            let block = self.files.get(entry).unwrap();
            let kind = block.attr.kind;
            let name = block.name.as_str();

            dir.push((*entry, kind, name));
        }

        for (i, entry) in dir.into_iter().enumerate().skip(offset as usize) {
            reply.add(entry.0, (i + 1) as i64, entry.1, entry.2);
        }

        reply.ok();
    }
}

pub async  fn fuse_main(tg_client: TgClient, mountpoint: OsString) {
    let file_system = TgFileSystem::new(tg_client).await;

    fuse::mount(file_system, &mountpoint, &[]).expect("ERROR: FUSE ALREADY ACTIVE");
}