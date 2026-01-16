use std::path::Path;
use transfer::{endpoint::make_client_endpoint, send::send_file};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let endpoint = make_client_endpoint();
    send_file(&endpoint, "127.0.0.1:5000", Path::new("test.txt")).await?;
    println!("File sent successfully!");
    Ok(())
}

// cd /Users/colin/Desktop/rust/transfer
// cargo run --example send_file -p transfer
