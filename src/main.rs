mod tg_client;
use rust_tdlib::{
    tdjson,
};
use crate::tg_client::TgClient;

#[tokio::main]
async fn main() {
    tdjson::set_log_verbosity_level(1);
    env_logger::init();

    println!("HI");

    let api_id = 29558350;
    let api_hash: String = String::from("47f892e160f00212f898358037a1d9b6");

    log::info!("Start client initialization");

    let tg = TgClient::new(std::env::var("api_id").unwrap().parse::<i32>().unwrap(), std::env::var("api_hash").unwrap(), "group").await;
    // let tg = TgClient::new(api_id, api_hash, "group").await;
    // let v = tg.get_metafiles();

    // for vec in v {
    //     println!("Meta: {}   ID: {}", vec.meta, vec.id);
    // }

    // tg.stop().await;
}