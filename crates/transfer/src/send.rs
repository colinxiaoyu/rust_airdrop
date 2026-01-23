use std::path::Path;

use quinn::Endpoint;
use tokio::fs::File;

use crate::protocol::FileHeader;

pub async fn send_file(endpoint: &Endpoint, remote: &str, file_path: &Path) -> anyhow::Result<()> {
    let conn = endpoint.connect(remote.parse()?, "airdrop")?.await?;

    let mut stream = conn.open_uni().await?;

    let mut file = File::open(file_path).await?;
    let metadata = file.metadata().await?;

    let header = FileHeader {
        file_name: file_path.file_name().unwrap().to_string_lossy().into(),
        file_size: metadata.len(),
    };
    let header_bytes = bincode::serialize(&header)?;
    stream
        .write_all(&(header_bytes.len() as u32).to_be_bytes())
        .await?;
    stream.write_all(&header_bytes).await?;

    tokio::io::copy(&mut file, &mut stream).await?;
    stream.finish()?;
    Ok(())
}
