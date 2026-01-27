use daemon::DaemonCore;
use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::AppHandle;

/// 应用全局状态
pub struct AppState {
    /// DaemonCore 实例（可能未初始化）
    pub daemon: Arc<RwLock<Option<DaemonCore>>>,
    /// Tauri App Handle（用于发送事件）
    pub app_handle: AppHandle,
}

impl AppState {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            daemon: Arc::new(RwLock::new(None)),
            app_handle,
        }
    }
}
