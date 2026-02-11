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

    // Android 特殊处理：检查是否是 content:// URI
    #[cfg(target_os = "android")]
    {
        if file_path.starts_with("content://") {
            return Err(format!(
                "Android content:// URI 暂不支持直接发送。请使用文件管理器选择文件。路径: {}",
                file_path
            ));
        }
    }

    // 验证文件路径
    let path = PathBuf::from(&file_path);

    tracing::debug!("检查文件是否存在: {}", path.display());
    if !path.exists() {
        tracing::error!("文件不存在: {}", file_path);
        return Err(format!("文件不存在: {}\n请确保应用有读取文件的权限", file_path));
    }

    if !path.is_file() {
        tracing::error!("路径不是文件: {}", file_path);
        return Err(format!("不是文件: {}", file_path));
    }

    // 检查文件是否可读
    match std::fs::metadata(&path) {
        Ok(metadata) => {
            tracing::info!("文件信息 - 大小: {} 字节, 只读: {}", metadata.len(), metadata.permissions().readonly());
        }
        Err(e) => {
            tracing::error!("无法读取文件元数据: {}", e);
            return Err(format!("无法访问文件: {}。错误: {}", file_path, e));
        }
    }

    // 获取 daemon
    let daemon_lock = state.daemon.read().await;
    let daemon = daemon_lock
        .as_ref()
        .ok_or_else(|| {
            tracing::error!("Daemon 未初始化");
            "Daemon 未初始化。请等待初始化完成后再试".to_string()
        })?;

    // 发送文件
    tracing::info!("开始发送文件到 {}", peer_name);
    daemon
        .send_file(&peer_name, path)
        .await
        .map_err(|e| {
            tracing::error!("发送失败: {}", e);
            format!("发送失败: {}", e)
        })?;

    tracing::info!("文件发送成功");
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
    #[cfg(target_os = "android")]
    {
        use std::path::PathBuf;
        // Android: 使用应用专属外部存储（无需额外权限）
        // 路径: /storage/emulated/0/Android/data/com.colin.tauri-airdrop/files/Downloads
        let dir = std::env::var("EXTERNAL_STORAGE")
            .ok()
            .map(|s| PathBuf::from(s).join("Android/data/com.colin.tauri-airdrop/files/Downloads"))
            .or_else(|| Some(PathBuf::from("/sdcard/Download")))
            .ok_or_else(|| "无法获取下载目录".to_string())?;

        Ok(dir.to_string_lossy().to_string())
    }

    #[cfg(not(target_os = "android"))]
    {
        let dir = dirs::download_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
            .ok_or_else(|| "无法获取下载目录".to_string())?;

        Ok(dir.to_string_lossy().to_string())
    }
}

/// 检查 Daemon 是否已就绪
#[tauri::command]
pub async fn check_daemon_ready(state: State<'_, AppState>) -> Result<bool, String> {
    let daemon_lock = state.daemon.read().await;
    Ok(daemon_lock.is_some())
}

/// 文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
}

/// 获取文件信息
#[tauri::command]
pub async fn get_file_info(file_path: String) -> Result<FileInfo, String> {
    tracing::debug!("获取文件信息: {}", file_path);

    // Android 特殊处理：content:// URI
    #[cfg(target_os = "android")]
    {
        if file_path.starts_with("content://") {
            tracing::warn!("收到 content:// URI: {}", file_path);
            return Err(format!(
                "暂不支持 content:// URI。请通过文件管理器选择文件。"
            ));
        }
    }

    let path = PathBuf::from(&file_path);

    if !path.exists() {
        tracing::error!("文件不存在: {}", file_path);
        return Err(format!("文件不存在: {}", file_path));
    }

    if !path.is_file() {
        tracing::error!("不是文件: {}", file_path);
        return Err(format!("不是文件: {}", file_path));
    }

    let metadata = std::fs::metadata(&path)
        .map_err(|e| {
            tracing::error!("无法读取文件元数据: {}", e);
            format!("无法读取文件信息: {}", e)
        })?;

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("未知文件")
        .to_string();

    tracing::info!("文件信息 - 名称: {}, 大小: {} 字节", name, metadata.len());

    Ok(FileInfo {
        path: file_path,
        name,
        size: metadata.len(),
    })
}
