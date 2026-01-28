use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

/// 前端使用的 Peer 信息（简化版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub name: String,
    pub addr: String,
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub port: u16,
}

/// 发送文件
///
/// # 参数
/// - `peer_name`: 目标设备名称
/// - `file_path`: 文件路径
#[tauri::command]
pub async fn send_file(
    state: State<'_, AppState>,
    peer_name: String,
    file_path: String,
) -> Result<(), String> {
    tracing::info!("Command: send_file - {} -> {}", file_path, peer_name);

    // 验证文件路径
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }
    if !path.is_file() {
        return Err(format!("不是文件: {}", file_path));
    }

    // 获取 daemon
    let daemon_lock = state.daemon.read().await;
    let daemon = daemon_lock
        .as_ref()
        .ok_or_else(|| "Daemon 未初始化".to_string())?;

    // 发送文件
    daemon
        .send_file(&peer_name, path)
        .await
        .map_err(|e| format!("发送失败: {}", e))?;

    Ok(())
}

/// 获取在线设备列表
#[tauri::command]
pub async fn list_peers(state: State<'_, AppState>) -> Result<Vec<PeerInfo>, String> {
    let daemon_lock = state.daemon.read().await;
    let daemon = daemon_lock
        .as_ref()
        .ok_or_else(|| "Daemon 未初始化".to_string())?;

    let peers = daemon.get_online_peers();

    Ok(peers
        .into_iter()
        .map(|p| PeerInfo {
            id: p.id,
            name: p.name,
            addr: p.addr.to_string(),
        })
        .collect())
}

/// 获取本设备信息
#[tauri::command]
pub async fn get_device_info(state: State<'_, AppState>) -> Result<DeviceInfo, String> {
    let daemon_lock = state.daemon.read().await;
    let daemon = daemon_lock
        .as_ref()
        .ok_or_else(|| "Daemon 未初始化".to_string())?;

    let info = daemon.get_device_info();

    Ok(DeviceInfo {
        name: info.name,
        port: info.port,
    })
}

/// 获取下载目录
#[tauri::command]
pub async fn get_download_dir() -> Result<String, String> {
    let dir = dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .ok_or_else(|| "无法获取下载目录".to_string())?;

    Ok(dir.to_string_lossy().to_string())
}

/// 检查 Daemon 是否已就绪
#[tauri::command]
pub async fn check_daemon_ready(state: State<'_, AppState>) -> Result<bool, String> {
    let daemon_lock = state.daemon.read().await;
    Ok(daemon_lock.is_some())
}
