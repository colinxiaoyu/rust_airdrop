use daemon::{DaemonCore, DaemonNotification, SessionEvent, TransferEvent};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use tracing::{error, info};

use crate::state::AppState;

/// 运行 Daemon 后台任务
///
/// 这是一个长期运行的异步任务，负责：
/// 1. 初始化 DaemonCore
/// 2. 将实例存储到 Tauri 状态
/// 3. 运行事件循环，转发通知到前端
pub async fn run_daemon(app_handle: AppHandle) {
    info!("启动 Daemon 后台任务");

    // 1. 初始化 DaemonCore
    let device_name = whoami::devicename();
    let download_dir = get_download_dir();

    info!("设备名: {}", device_name);
    info!("下载目录: {}", download_dir.display());

    // 尝试多个端口（避免端口冲突）
    let ports_to_try = [5001, 5002, 5003, 5004, 5005];
    let mut daemon = None;
    let mut last_error = String::new();

    for port in ports_to_try {
        info!("尝试绑定端口 {}", port);
        match DaemonCore::new(device_name.clone(), port, download_dir.clone()) {
            Ok(d) => {
                info!("成功绑定端口 {}", port);
                daemon = Some(d);
                break;
            }
            Err(e) => {
                error!("端口 {} 绑定失败: {}", port, e);
                last_error = format!("{}", e);
            }
        }
    }

    let daemon = match daemon {
        Some(d) => d,
        None => {
            error!("无法绑定任何端口，初始化失败");
            let _ = app_handle.emit(
                "daemon-error",
                format!("无法绑定端口 (尝试了 {:?}): {}", ports_to_try, last_error),
            );
            return;
        }
    };

    // 2. 存储到状态
    {
        let state: tauri::State<AppState> = app_handle.state();
        let mut daemon_lock = state.daemon.write().await;
        *daemon_lock = Some(daemon);
    }

    info!("DaemonCore 初始化成功");
    let _ = app_handle.emit("daemon-ready", ());

    // 3. 主事件循环
    loop {
        // 获取 daemon 的可变引用
        let notification = {
            let state: tauri::State<AppState> = app_handle.state();
            let mut daemon_lock = state.daemon.write().await;

            if let Some(daemon) = daemon_lock.as_mut() {
                daemon.tick().await
            } else {
                error!("Daemon 实例不存在");
                break;
            }
        }; // 释放锁

        // 处理通知（在锁外，避免死锁）
        if let Some(notification) = notification {
            emit_to_frontend(&app_handle, notification);
        }
    }

    error!("Daemon 事件循环退出");
}

/// 转发 Daemon 通知到前端
fn emit_to_frontend(app_handle: &AppHandle, notification: DaemonNotification) {
    match notification {
        // Session 事件
        DaemonNotification::Session(SessionEvent::PeerOnline(peer)) => {
            info!("前端事件: peer-online - {}", peer.name);
            if let Err(e) = app_handle.emit("peer-online", &peer) {
                error!("发送事件失败: {}", e);
            }
        }
        DaemonNotification::Session(SessionEvent::PeerOffline(peer)) => {
            info!("前端事件: peer-offline - {}", peer.name);
            if let Err(e) = app_handle.emit("peer-offline", &peer) {
                error!("发送事件失败: {}", e);
            }
        }

        // Transfer 事件
        DaemonNotification::Transfer(event) => match &event {
            TransferEvent::FileReceived {
                file_name,
                file_size,
                file_path,
                sender_addr,
            } => {
                info!(
                    "前端事件: file-received - {} 来自 {}",
                    file_name, sender_addr
                );

                // 序列化为前端友好的格式
                let payload = serde_json::json!({
                    "from": sender_addr.to_string(),
                    "fileName": file_name,
                    "file": file_path.to_string_lossy(),
                    "size": file_size,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                if let Err(e) = app_handle.emit("file-received", payload) {
                    error!("发送事件失败: {}", e);
                }

                // 系统通知
                #[cfg(not(target_os = "linux"))]
                {
                    use tauri_plugin_notification::NotificationExt;
                    let _ = app_handle
                        .notification()
                        .builder()
                        .title("收到文件")
                        .body(format!("来自 {}: {}", sender_addr, file_name))
                        .show();
                }
            }
            TransferEvent::ReceiveFailed { error, sender_addr } => {
                error!("接收失败: {} 来自 {:?}", error, sender_addr);
                let payload = serde_json::json!({
                    "from": sender_addr.as_ref().map(|a| a.to_string()).unwrap_or_else(|| "unknown".to_string()),
                   "error": error,
                });
                let _ = app_handle.emit("receive-error", payload);
            }
        },
    }
}

/// 获取下载目录
fn get_download_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .unwrap_or_else(|| PathBuf::from("./downloads"))
}
