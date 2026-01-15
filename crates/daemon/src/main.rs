use std::time::Duration;

use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("airdropd starting....");

    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        info!("airdropd alive");
    }
}
