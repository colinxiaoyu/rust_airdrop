use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use quinn::Endpoint;
use tokio::sync::mpsc;

use crate::{endpoint, event::TransferEvent, receive::receive_file, send::send_file};
use tracing::{error, info};
pub struct TransferManager {
    server_endpoint: Endpoint,  // 用于接收文件
    client_endpoint: Endpoint,  // 用于发送文件
    client_config: quinn::ClientConfig,  // 客户端配置
    download_dir: Arc<PathBuf>,
    event_tx: mpsc::Sender<TransferEvent>,
}

impl TransferManager {
    pub fn new(
        bind_port: u16,
        download_dir: PathBuf,
        event_tx: mpsc::Sender<TransferEvent>,
    ) -> Result<Self> {
        info!("Initializing TransferManager...");

        // 1. 确保下载目录存在
        if !download_dir.exists() {
            std::fs::create_dir_all(&download_dir)?;
        }
        info!("Download directory ready: {:?}", download_dir);

        // 2. 创建 Client Config（用于发送）
        info!("Creating client config for sending files...");
        let client_config = endpoint::make_client_config()
            .map_err(|e| {
                error!("Failed to create client config: {:?}", e);
                e
            })?;
        info!("Client config created successfully");

        // 3. 创建 Client Endpoint（用于发送）
        info!("Creating client endpoint for sending files...");
        let client_endpoint = endpoint::make_client_endpoint()
            .map_err(|e| {
                error!("Failed to create client endpoint: {:?}", e);
                e
            })?;
        info!("Client endpoint created successfully, local address: {:?}", client_endpoint.local_addr());

        // 4. 创建 Server Endpoint（用于接收）
        info!("Creating server endpoint for receiving files...");
        let server_endpoint =
            endpoint::make_server_endpoint(format!("0.0.0.0:{bind_port}").parse()?)
                .map_err(|e| {
                    error!("Failed to create server endpoint: {:?}", e);
                    e
                })?;
        info!("Server endpoint created successfully, listening on: {:?}", server_endpoint.local_addr());

        let download_dir = Arc::new(download_dir);

        // 5. 启动后台接收任务
        info!("Starting receiver loop...");
        tokio::spawn(Self::run_receiver_loop(
            server_endpoint.clone(),
            download_dir.clone(),
            event_tx.clone(),
        ));

        info!("TransferManager initialized successfully");
        Ok(Self {
            server_endpoint,
            client_endpoint,
            client_config,
            download_dir,
            event_tx,
        })
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.server_endpoint
    }

    pub async fn send(&self, peer_addr: String, file: PathBuf) -> Result<()> {
        info!("Sending file to {}: {:?}", peer_addr, file);
        // 使用 client_endpoint 和显式的 client_config 发送文件
        let result = send_file(&self.client_endpoint, &self.client_config, &peer_addr, &file).await;
        match &result {
            Ok(_) => info!("File sent successfully to {}", peer_addr),
            Err(e) => error!("Failed to send file to {}: {:?}", peer_addr, e),
        }
        result
    }

    /// 后台接收循环
    async fn run_receiver_loop(
        endpoint: Endpoint,
        download_dir: Arc<PathBuf>,
        event_tx: mpsc::Sender<TransferEvent>,
    ) {
        info!("Transfer receiver started, listening for incoming files");
        info!("Endpoint local address: {:?}", endpoint.local_addr());

        loop {
            // 1. 等待新连接
            info!("Waiting for incoming connection...");
            let incoming = match endpoint.accept().await {
                Some(inc) => {
                    info!("Incoming connection detected from: {:?}", inc.remote_address());
                    inc
                }
                None => {
                    error!("Endpoint closed, receiver exiting");
                    break;
                }
            };

            // 2. 为每个连接 spawn 独立任务（支持并发）
            let download_dir = download_dir.clone();
            let event_tx = event_tx.clone();

            tokio::spawn(async move {
                info!("Spawned task to handle incoming connection");
                // 3. 建立连接
                let conn = match incoming.await {
                    Ok(c) => {
                        info!("Connection established from: {}", c.remote_address());
                        c
                    }
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
                info!("Starting file reception from {}", sender_addr);

                // 4. 接收文件
                match receive_file(conn, &download_dir).await {
                    Ok(result) => {
                        info!(
                            "File received from {}: {} ({} bytes) -> {:?}",
                            result.sender_addr, result.file_name, result.file_size, result.file_path
                        );

                        // 发送成功事件
                        info!("Sending FileReceived event to daemon");
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
