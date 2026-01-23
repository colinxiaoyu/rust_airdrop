use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

use anyhow::{Ok, Result};
use quinn::{Connection, Endpoint};
use tokio::{fs::File, io::AsyncWriteExt};

use crate::protocol::FileHeader;
use tracing::{error, info};
pub struct ReceiveResult {
    pub file_name: String,
    pub file_size: u64,
    pub file_path: PathBuf,
    pub sender_addr: SocketAddr,
}

pub async fn receive_file(conn: Connection, download_dir: &Path) -> Result<ReceiveResult> {
    let sender_addr = conn.remote_address();
    let mut uni = conn.accept_uni().await?;

    // 1. 读取 header 长度
    let mut len_buf = [0u8; 4];
    uni.read_exact(&mut len_buf).await?;
    let header_len = u32::from_be_bytes(len_buf) as usize;

    // 读取 header 内容
    let mut header_buf = vec![0; header_len];
    uni.read_exact(&mut header_buf).await?;
    let header: FileHeader = bincode::deserialize(&header_buf)?;

    info!(
        "Receiving file: {} ({} bytes) from {}",
        header.file_name, header.file_size, sender_addr
    );

    // 3. 安全的文件路径处理（防止路径遍历攻击）
    let safe_file_name = sanitize_filename(&header.file_name);
    let mut file_path = download_dir.join(&safe_file_name);

    // 4. 处理文件重名（添加递增后缀）
    file_path = get_unique_path(file_path).await;

    // 5. 写入文件内容
    let mut file = File::create(&file_path).await?;
    let bytes_written = tokio::io::copy(&mut uni, &mut file).await?;
    file.flush().await?;

    info!(
        "File received successfully: {} ({} bytes) -> {:?}",
        header.file_name, bytes_written, file_path
    );

    Ok(ReceiveResult {
        file_name: header.file_name,
        file_size: bytes_written,
        file_path,
        sender_addr,
    })
}

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

/// 清理文件名，防止路径遍历攻击
fn sanitize_filename(filename: &str) -> String {
    filename
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim_start_matches('.') // 防止隐藏文件
        .to_string()
}

/// 获取唯一文件路径（处理重名）
async fn get_unique_path(mut path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
    let extension = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let parent = path.parent().unwrap().to_path_buf();

    let mut counter = 1;
    loop {
        path = parent.join(format!("{}_{}{}", stem, counter, extension));
        if !path.exists() {
            return path;
        }
        counter += 1;
    }
}
