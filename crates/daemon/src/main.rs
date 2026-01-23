use std::{path::PathBuf, time::Duration};

use discovery::Discovery;
use session::{event::SessionEvent, manager::SessionManager};
use tokio::sync::mpsc;
use tracing::error;
use tracing::info;
use transfer::event::TransferEvent;
use transfer::manager::TransferManamger;

use crate::event::DaemonEvent;

pub mod event;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("airdropd starting...");

    // 1. Discovery
    let mut discovery = Discovery::new("MyDevice");

    // 2. Session
    let (session_tx, mut session_rx) = mpsc::channel(100);
    let mut session_manager = SessionManager::new(session_tx);

    // Transfer
    let (transfer_tx, mut transfer_rx) = mpsc::channel::<TransferEvent>(100);
    let download_dir = PathBuf::from("./downloads");
    let transfer = TransferManamger::new(5000, download_dir, transfer_tx);

    // 4. Daemon event channel (CLI / internal trigger)
    let (daemon_tx, mut daemon_rx) = mpsc::channel::<DaemonEvent>(10);

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

    // 5. 主事件循环
    loop {
        tokio::select! {
            // Discovery → Session
            Some(peer) = discovery.rx.recv() => {
                session_manager.on_peer_discovered(peer).await;
            }

            // Session → Daemon
            Some(event) = session_rx.recv() => {
                match event {
                    SessionEvent::PeerOnline(peer) => {
                        info!("peer online: {}", peer.name);
                    },
                    SessionEvent::PeerOffline(peer) => {
                        info!("peer offline: {}", peer.name);
                    }
                }
            }

            // Daemon 内部事件（发送文件）
            Some(cmd) = daemon_rx.recv() => {
                match cmd {
                    DaemonEvent::SendFile { peer_name, file } => {
                        if let Some(peer) = session_manager.get_peer(&peer_name) {
                            let addr = format!("{}", peer.addr);
                            info!("sending file {:?} to {}", file, peer_name);
                            if let Err(e) = transfer.send(addr, file).await {
                                error!("transfer failed: {:?}", e)
                            }
                        } else {
                            error!("peer not found: {}", peer_name);
                        }
                    }
                }
            }

            // Transfer → Daemon
            Some(event) = transfer_rx.recv() => {
                match event {
                    TransferEvent::FileReceived {
                        file_name,
                        file_size,
                        file_path,
                        sender_addr,
                    } => {
                        info!(
                            "File received: {} ({} bytes) from {} -> {:?}",
                            file_name, file_size, sender_addr, file_path
                        );
                    }
                    TransferEvent::ReceiveFailed { error, sender_addr } => {
                        error!("File receive failed from {:?}: {}", sender_addr, error);
                    }
                }
            }

            // 定期清理 offline
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                session_manager.reap_offline(Duration::from_secs(30)).await;
            }
        }
    }
}
