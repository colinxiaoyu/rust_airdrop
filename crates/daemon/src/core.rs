use std::{path::PathBuf, time::Duration};

use discovery::Discovery;
use session::{SessionEvent, SessionManager};
use tokio::sync::mpsc;
use tracing::info;
use transfer::{TransferEvent, TransferManager};

use crate::event::DaemonEvent;

pub struct DaemonCore {
    discovery: Discovery,
    session_manager: SessionManager,
    transfer_manager: TransferManager,
    session_rx: mpsc::Receiver<SessionEvent>,
    transfer_rx: mpsc::Receiver<TransferEvent>,
    daemon_tx: mpsc::Sender<DaemonEvent>,
    daemon_rx: mpsc::Receiver<DaemonEvent>,
}

impl DaemonCore {
    pub fn new(device_name: String, bind_port: u16, download_dir: PathBuf) -> Self {
        tracing_subscriber::fmt::init();
        info!("airdropd starting...");
        // 1. Discovery
        let discovery = Discovery::new(&device_name);
        // 2. Session
        let (session_tx, session_rx) = mpsc::channel(100);
        let session_manager = SessionManager::new(session_tx);
        // Transfer
        let (transfer_tx, transfer_rx) = mpsc::channel::<TransferEvent>(100);
        let transfer_manager = TransferManager::new(bind_port, download_dir, transfer_tx);
        // 4. Daemon event channel (CLI / internal trigger)
        let (daemon_tx, daemon_rx) = mpsc::channel::<DaemonEvent>(10);
        // 🚧 临时：5 秒后模拟一次“发送文件”
        tokio::spawn({
            let tx = daemon_tx.clone();
            async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                tx.send(DaemonEvent::SendFile {
                    peer_name: "peer".into(),
                    file: PathBuf::from("test.txt"),
                })
                .await
                .ok();
            }
        });

        DaemonCore {
            discovery,
            session_manager,
            transfer_manager,
            session_rx,
            transfer_rx,
            daemon_tx,
            daemon_rx,
        }
    }

    pub async fn tick(&mut self) -> Option<DaemonNotification> {
        tokio::select! {
            // 1. 发现新设备
            Some(peer) = self.discovery.rx.recv() => {
                tracing::info!("发现设备: {}", peer.name);
                self.session_manager.on_peer_discovered(peer).await;
                None  // 内部处理，不需要通知
            }
            // 2. Session 事件（设备上线/下线）
            Some(event) = self.session_rx.recv() => {
                match &event {
                    SessionEvent::PeerOnline(peer) => {
                        tracing::info!("设备上线: {}", peer.name);
                    }
                    SessionEvent::PeerOffline(peer) => {
                        tracing::info!("设备下线: {}", peer.name);
                    }
                }
                Some(DaemonNotification::Session(event))
            }
            // 3. Transfer 事件（文件传输）
            Some(event) = self.transfer_rx.recv() => {
                match &event {
                    TransferEvent::FileReceived { file_name, file_size, file_path, sender_addr } => {
                        tracing::info!("收到文件: {} 来自 {} ({}bytes)",
                            file_name, sender_addr, file_size);
                    }
                    TransferEvent::ReceiveFailed { error, sender_addr } => {
                        tracing::error!("接收失败: {} 来自 {:?}", error, sender_addr);
                    }
                }
                Some(DaemonNotification::Transfer(event))
            }
            // 4. 命令处理（来自 UI 或 CLI）
            Some(cmd) = self.daemon_rx.recv() => {
                self.handle_command(cmd).await;
                None
            }
            // 5. 定时任务：清理离线设备
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                self.session_manager.reap_offline(Duration::from_secs(30)).await;
                None
            }
        }
    }

    async fn handle_command(&mut self, cmd: DaemonEvent) {
        match cmd {
            DaemonEvent::SendFile { peer_name, file } => {
                if let Some(peer) = self.session_manager.find_peer_by_name(&peer_name) {
                    let peer_addr = format!("{}:{}", peer.addr.ip(), 5000);
                    match self.transfer_manager.send(peer_addr, file.clone()).await {
                        Ok(_) => {
                            tracing::info!("成功发送文件: {} 到 {}", file.display(), peer_name);
                        }
                        Err(e) => {
                            tracing::error!("发送文件失败: {:?}", e);
                        }
                    }
                } else {
                    tracing::error!("找不到设备: {}", peer_name);
                }
            }
        }
    }
}

/// Daemon 通知（需要传递给 UI 的事件）
#[derive(Debug, Clone)]
pub enum DaemonNotification {
    Session(SessionEvent),
    Transfer(TransferEvent),
}

/// 设备信息
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub port: u16,
}
