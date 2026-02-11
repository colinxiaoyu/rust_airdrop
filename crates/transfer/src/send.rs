use std::path::Path;

use quinn::Endpoint;
use tokio::fs::File;

use crate::protocol::FileHeader;

pub async fn send_file(
    endpoint: &Endpoint,
    client_config: &quinn::ClientConfig,
    remote: &str,
    file_path: &Path,
) -> anyhow::Result<()> {
    tracing::info!("send_file: Connecting to {}", remote);
    tracing::info!("send_file: Endpoint local address: {:?}", endpoint.local_addr());

    let remote_addr = remote.parse()?;
    tracing::info!("send_file: Parsed remote address: {:?}", remote_addr);

    // 使用 connect_with 显式传入客户端配置
    let connecting = endpoint.connect_with(client_config.clone(), remote_addr, "airdrop")
        .map_err(|e| {
            tracing::error!("send_file: Failed to initiate connection: {:?}", e);
            anyhow::anyhow!("Failed to initiate connection: {:?}", e)
        })?;
    tracing::info!("send_file: Connection initiated, waiting for handshake...");

    let conn = connecting.await
        .map_err(|e| {
            tracing::error!("send_file: Connection failed: {:?}", e);
            anyhow::anyhow!("Connection failed: {:?}", e)
        })?;
    tracing::info!("send_file: Connected to {}", conn.remote_address());

    let mut stream = conn.open_uni().await
        .map_err(|e| {
            tracing::error!("send_file: Failed to open stream: {:?}", e);
            anyhow::anyhow!("Failed to open stream: {:?}", e)
        })?;
    tracing::info!("send_file: Opened unidirectional stream");

    let mut file = File::open(file_path).await?;
    let metadata = file.metadata().await?;

    let header = FileHeader {
        file_name: file_path.file_name().unwrap().to_string_lossy().into(),
        file_size: metadata.len(),
    };
    tracing::info!("send_file: Sending header: {} ({} bytes)", header.file_name, header.file_size);

    let header_bytes = bincode::serialize(&header)?;
    stream
        .write_all(&(header_bytes.len() as u32).to_be_bytes())
        .await?;
    stream.write_all(&header_bytes).await?;
    tracing::info!("send_file: Header sent, sending file content...");

    let bytes_copied = tokio::io::copy(&mut file, &mut stream).await?;
    tracing::info!("send_file: File content sent ({} bytes)", bytes_copied);

    stream.finish()
        .map_err(|e| {
            tracing::error!("send_file: Failed to finish stream: {:?}", e);
            anyhow::anyhow!("Failed to finish stream: {:?}", e)
        })?;
    tracing::info!("send_file: Stream finished, waiting for acknowledgment...");

    // Wait for the stream to be fully acknowledged by the receiver
    // This prevents the connection from being dropped before all data is transmitted
    stream.stopped().await
        .map_err(|e| {
            tracing::error!("send_file: Stream stopped with error: {:?}", e);
            anyhow::anyhow!("Stream stopped with error: {:?}", e)
        })?;
    tracing::info!("send_file: Transfer completed and acknowledged successfully");

    Ok(())
}
