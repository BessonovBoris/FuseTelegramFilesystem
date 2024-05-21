mod tg_client;
use rust_tdlib::{
    tdjson,
};
use crate::tg_client::TgClient;

#[tokio::main]
async fn main() {
    tdjson::set_log_verbosity_level(1);
    env_logger::init();

    let tg = TgClient::new(std::env::var("API_ID").unwrap().parse::<i32>().unwrap(), std::env::var("API_HASH").unwrap(), "group").await;
    let v = tg.get_metafiles();

    for vec in v {
        println!("Meta: {}   ID: {}", vec.meta, vec.id);
    }

    tg.stop().await;
}