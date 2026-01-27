use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use discovery::{Discovery, Peer};
use session::{SessionEvent, SessionManager};
use tokio::sync::mpsc;
use tracing::{error, info};
use transfer::{TransferEvent, TransferManager};

use crate::event::DaemonEvent;

/// Daemon 核心，管理所有子模块的生命周期
pub struct DaemonCore {
    // 子模块
    discovery: Discovery,
    session_manager: SessionManager,
    transfer_manager: TransferManager,

    // 设备信息
    device_name: String,
    bind_port: u16,

    // 事件通道（接收）
    session_rx: mpsc::Receiver<SessionEvent>,
    transfer_rx: mpsc::Receiver<TransferEvent>,

    // 命令通道（发送/接收）
    daemon_tx: mpsc::Sender<DaemonEvent>,
    daemon_rx: mpsc::Receiver<DaemonEvent>,
}

impl DaemonCore {
    /// 创建新的 DaemonCore 实例
    ///
    /// # 参数
    /// - `device_name`: 本设备名称（用于广播和显示）
    /// - `bind_port`: 监听端口（Discovery 和 Transfer 使用）
    /// - `download_dir`: 接收文件的保存目录
    ///
    /// # 注意
    /// 调用者应该在调用此函数之前初始化 tracing (如 `tracing_subscriber::fmt::init()`)
    pub fn new(device_name: String, bind_port: u16, download_dir: PathBuf) -> Result<Self> {
        info!("Initializing DaemonCore...");
        info!("Device name: {}", device_name);
        info!("Bind port: {}", bind_port);
        info!("Download dir: {}", download_dir.display());

        // 1. 创建事件通道
        let (session_tx, session_rx) = mpsc::channel(100);
        let (transfer_tx, transfer_rx) = mpsc::channel(100);
        let (daemon_tx, daemon_rx) = mpsc::channel(100);

        // 2. 初始化 Discovery
        let discovery = Discovery::new(&device_name);

        // 3. 初始化 SessionManager
        let session_manager = SessionManager::new(session_tx);

        // 4. 初始化 TransferManager（自动接收）
        let transfer_manager = TransferManager::new(bind_port, download_dir, transfer_tx)?;

        info!("DaemonCore initialized successfully");

        Ok(Self {
            discovery,
            session_manager,
            transfer_manager,
            device_name,
            bind_port,
            session_rx,
            transfer_rx,
            daemon_tx,
            daemon_rx,
        })
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

    /// 处理命令
    async fn handle_command(&mut self, cmd: DaemonEvent) {
        match cmd {
            DaemonEvent::SendFile { peer_name, file } => {
                if let Err(e) = self.send_file_internal(&peer_name, file).await {
                    error!("发送文件失败: {}", e);
                }
            }
        }
    }

    /// 内部发送文件逻辑
    async fn send_file_internal(&self, peer_name: &str, file: PathBuf) -> Result<()> {
        // 1. 查找目标设备
        let peer = self
            .session_manager
            .find_peer_by_name(peer_name)
            .ok_or_else(|| anyhow::anyhow!("设备不在线: {}", peer_name))?;

        // 2. 检查文件是否存在
        if !file.exists() {
            return Err(anyhow::anyhow!("文件不存在: {}", file.display()));
        }

        // 3. 构造地址并发送文件
        let peer_addr = format!("{}:{}", peer.addr.ip(), self.bind_port);
        self.transfer_manager.send(peer_addr, file.clone()).await?;

        info!("成功发送文件: {} 到 {}", file.display(), peer_name);
        Ok(())
    }

    /// 公开 API：发送文件
    ///
    /// # 参数
    /// - `peer_name`: 目标设备名称
    /// - `file`: 要发送的文件路径
    pub async fn send_file(&self, peer_name: &str, file: PathBuf) -> Result<()> {
        self.daemon_tx
            .send(DaemonEvent::SendFile {
                peer_name: peer_name.to_string(),
                file,
            })
            .await?;
        Ok(())
    }

    /// 公开 API：获取在线设备列表
    pub fn get_online_peers(&self) -> Vec<Peer> {
        self.session_manager.get_online_peers()
    }

    /// 公开 API：获取本设备信息
    pub fn get_device_info(&self) -> DeviceInfo {
        DeviceInfo {
            name: self.device_name.clone(),
            port: self.bind_port,
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub port: u16,
}
