//! Airdrop Daemon CLI
//!
//! 这是一个独立的守护进程版本，使用 DaemonCore 库

use clap::Parser;
use daemon::{DaemonCore, DaemonNotification};
use std::path::PathBuf;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "airdropd")]
#[command(about = "Airdrop 守护进程 - 局域网文件快传", long_about = None)]
struct Args {
    /// 设备名称
    #[arg(short, long, default_value = "MyDevice")]
    name: String,

    /// 监听端口
    #[arg(short, long, default_value = "5000")]
    port: u16,

    /// 下载目录
    #[arg(short, long, default_value = "./downloads")]
    download_dir: PathBuf,

    /// 日志级别 (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&args.log_level)),
        )
        .init();

    info!("🚀 Airdrop 守护进程启动");
    info!("   设备名称: {}", args.name);
    info!("   监听端口: {}", args.port);
    info!("   下载目录: {}", args.download_dir.display());

    // 创建下载目录
    if !args.download_dir.exists() {
        std::fs::create_dir_all(&args.download_dir)?;
        info!("   创建下载目录: {}", args.download_dir.display());
    }

    // 初始化 DaemonCore
    let mut daemon = DaemonCore::new(args.name, args.port, args.download_dir)?;

    info!("✅ 初始化完成，开始监听...");
    info!("   按 Ctrl+C 退出");

    // 主事件循环
    loop {
        if let Some(notification) = daemon.tick().await {
            handle_notification(notification);
        }
    }
}

/// 处理 Daemon 通知
fn handle_notification(notification: DaemonNotification) {
    match notification {
        DaemonNotification::Session(event) => {
            use daemon::SessionEvent;
            match event {
                SessionEvent::PeerOnline(peer) => {
                    info!("📱 设备上线: {} ({})", peer.name, peer.addr);
                }
                SessionEvent::PeerOffline(peer) => {
                    info!("📴 设备下线: {} ({})", peer.name, peer.addr);
                }
            }
        }
        DaemonNotification::Transfer(event) => {
            use daemon::TransferEvent;
            match event {
                TransferEvent::FileReceived {
                    file_name,
                    file_size,
                    file_path,
                    sender_addr,
                } => {
                    info!(
                        "📥 收到文件: {} ({} bytes) 来自 {} -> {}",
                        file_name,
                        file_size,
                        sender_addr,
                        file_path.display()
                    );
                }
                TransferEvent::ReceiveFailed { error, sender_addr } => {
                    tracing::error!("❌ 接收失败: {} 来自 {:?}", error, sender_addr);
                }
            }
        }
    }
}
