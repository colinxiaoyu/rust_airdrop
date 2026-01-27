use transfer::endpoint::make_server_endpoint;
use transfer::receive::run_receiver;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let endpoint = make_server_endpoint("0.0.0.0:5000".parse()?)?;
    println!("listening on 5000");
    run_receiver(endpoint).await
}

// cd /Users/colin/Desktop/rust/transfer
// cargo run --example receive_file -p transfer
