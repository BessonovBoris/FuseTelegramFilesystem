mod tg_client;
mod rust_fuse;

use std::env;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use rust_tdlib::{
    tdjson,
};
use crate::rust_fuse::fuse_main;
use crate::tg_client::TgClient;
// api_id=29558350 api_hash=47f892e160f00212f898358037a1d9b6 cargo run - запуск

#[tokio::main]
async fn main() {
    tdjson::set_log_verbosity_level(1);
    env_logger::init();

    let tg = TgClient::new(env::var("api_id").unwrap().parse::<i32>().unwrap(), std::env::var("api_hash").unwrap(), "group").await;

    // let v = tg.get_files().await;
    // let mut id: i32 = 0;

    // for i in v.into_iter().enumerate() {
    //     println!("{} - {}", i.1.1.message_id, i.1.1.name)
    // }

    // let p = tg.download_file(191165890560i32).await;
    // println!("{}", p);

    fuse_main(tg, env::args_os().nth(1).unwrap()).await;

    // tg.stop().await;
}