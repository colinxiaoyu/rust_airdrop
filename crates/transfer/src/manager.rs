use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use quinn::Endpoint;
use tokio::sync::mpsc;

use crate::{endpoint, event::TransferEvent, receive::receive_file, send::send_file};
use tracing::{error, info};
pub struct TransferManager {
    endpoint: Endpoint,
    download_dir: Arc<PathBuf>,
    event_tx: mpsc::Sender<TransferEvent>,
}

impl TransferManager {
    pub fn new(
        bind_port: u16,
        download_dir: PathBuf,
        event_tx: mpsc::Sender<TransferEvent>,
    ) -> Result<Self> {
        // 1. 确保下载目录存在
        if !download_dir.exists() {
            std::fs::create_dir_all(&download_dir)?;
        }

        // 2. 创建 Endpoint
        let endpoint =
            endpoint::make_server_endpoint(format!("0.0.0.0:{bind_port}").parse()?)?;

        let download_dir = Arc::new(download_dir);

        // 3. 启动后台接收任务
        tokio::spawn(Self::run_receiver_loop(
            endpoint.clone(),
            download_dir.clone(),
            event_tx.clone(),
        ));
        Ok(Self {
            endpoint,
            download_dir,
            event_tx,
        })
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub async fn send(&self, peer_addr: String, file: PathBuf) -> Result<()> {
        send_file(&self.endpoint, &peer_addr, &file).await
    }

    /// 后台接收循环
    async fn run_receiver_loop(
        endpoint: Endpoint,
        download_dir: Arc<PathBuf>,
        event_tx: mpsc::Sender<TransferEvent>,
    ) {
        info!("Transfer receiver started, listening for incoming files");

        loop {
            // 1. 等待新连接
            let incoming = match endpoint.accept().await {
                Some(inc) => inc,
                None => {
                    error!("Endpoint closed, receiver exiting");
                    break;
                }
            };

            // 2. 为每个连接 spawn 独立任务（支持并发）
            let download_dir = download_dir.clone();
            let event_tx = event_tx.clone();

            tokio::spawn(async move {
                // 3. 建立连接
                let conn = match incoming.await {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Failed to establish connection: {:?}", e);
                        let _ = event_tx
                            .send(TransferEvent::ReceiveFailed {
                                error: format!("Connection failed: {:?}", e),
                                sender_addr: None,
                            })
                            .await;
                        return;
                    }
                };
                let sender_addr = conn.remote_address();

                // 4. 接收文件
                match receive_file(conn, &download_dir).await {
                    Ok(result) => {
                        info!(
                            "File received from {}: {} ({} bytes)",
                            result.sender_addr, result.file_name, result.file_size
                        );

                        // 发送成功事件
                        let _ = event_tx
                            .send(TransferEvent::FileReceived {
                                file_name: result.file_name,
                                file_size: result.file_size,
                                file_path: result.file_path,
                                sender_addr: result.sender_addr,
                            })
                            .await;
                    }
                    Err(e) => {
                        error!("Failed to receive file from {}: {:?}", sender_addr, e);

                        // 发送失败事件
                        let _ = event_tx
                            .send(TransferEvent::ReceiveFailed {
                                error: format!("{:?}", e),
                                sender_addr: Some(sender_addr),
                            })
                            .await;
                    }
                }
            });
        }
    }
}
