use crate::protocol::FileHeader;
use quinn::Endpoint;
use tokio::{fs::File, io::AsyncWriteExt};

pub async fn run_receiver(endpoint: Endpoint) -> anyhow::Result<()> {
    loop {
        let incoming = endpoint.accept().await.unwrap();
        let conn = incoming.await?;

        let mut uni = conn.accept_uni().await?;

        // 1. 读取 header 长度
        let mut len_buf = [0u8; 4];
        uni.read_exact(&mut len_buf).await?;
        let header_len = u32::from_be_bytes(len_buf) as usize;

        // 2. 读取 header
        let mut header_buf = vec![0; header_len];
        uni.read_exact(&mut header_buf).await?;
        let header: FileHeader = bincode::deserialize(&header_buf)?;

        let mut file = File::create(&header.file_name).await?;

        // 3. 写入文件内容
        tokio::io::copy(&mut uni, &mut file).await?;
        file.flush().await?;
    }
}
