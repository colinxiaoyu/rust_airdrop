# Airdrop 项目开发计划

## 当前任务状态

### ✅ 已完成
- TransferManager 自动接收文件功能实现
- 事件驱动架构完整
- Discovery + Session + Transfer 模块集成

### 🔄 下一步：Tauri 桌面应用化

---

## Tauri 集成方案概览

### 核心理念
将现有的 Daemon 核心逻辑重构为库，被 Tauri 应用复用，实现跨平台桌面应用。

**设计原则**:
- **关注点分离**: UI 层只关心展示，业务逻辑在 Rust 层
- **异步优先**: 利用 Tokio 的异步能力，保持 UI 响应性
- **事件驱动**: 通过 Tauri 的事件系统实现前后端通信
- **类型安全**: TypeScript + Rust 的端到端类型安全

### 架构设计

```
┌─────────────────────────────────────────────────────────┐
│                    Tauri 桌面应用                         │
│                                                           │
│  ┌──────────────────┐           ┌────────────────────┐  │
│  │   React UI       │  Events   │  Tauri Rust        │  │
│  │  (TypeScript)    │◄──────────┤   Backend          │  │
│  │                  │           │                    │  │
│  │  - 设备列表      │  Commands │  - Tauri Commands  │  │
│  │  - 文件传输      ├──────────►│  - State Manager   │  │
│  │  - 通知提示      │           │  - Event Emitter   │  │
│  └──────────────────┘           └──────────┬─────────┘  │
│                                             │            │
│                                  ┌──────────▼─────────┐  │
│                                  │   DaemonCore       │  │
│                                  │   (Library)        │  │
│                                  │                    │  │
│                                  │  - tick() 循环     │  │
│                                  │  - 命令处理        │  │
│                                  │  - 事件分发        │  │
│                                  └──────────┬─────────┘  │
└─────────────────────────────────────────────┼───────────┘
                                              │
                        ┌─────────────────────┼─────────────────────┐
                        │                     │                     │
                ┌───────▼────────┐   ┌────────▼────────┐   ┌───────▼────────┐
                │   Discovery    │   │  SessionManager │   │TransferManager │
                │                │   │                 │   │                │
                │ - mDNS 服务    │   │ - 会话生命周期  │   │ - 发送/接收    │
                │ - 设备发现     │   │ - 心跳检测      │   │ - 进度跟踪     │
                └────────────────┘   └─────────────────┘   └────────────────┘
```

**数据流**:
1. **发现流程**: Discovery → SessionManager → DaemonCore → Tauri Events → UI
2. **发送文件**: UI → Tauri Command → DaemonCore → TransferManager → 网络
3. **接收文件**: 网络 → TransferManager → DaemonCore → Tauri Events → UI 通知

---

## 实施步骤

### Phase 1: Daemon 重构（核心）

#### 重要架构说明

**Peer 的定义位置**:
- ✅ **Peer 定义在 Discovery 模块中**（不是 Session）
- 原因：Discovery 通过 mDNS 发现设备并创建 Peer，是 Peer 的"产生者"
- Session 和 Transfer 都使用 `discovery::Peer`，避免循环依赖
- Daemon 重新导出 `pub use discovery::Peer;` 便于外部使用

**模块依赖图**:
```
discovery::Peer ─┐
                 ├─> session::SessionManager
                 └─> transfer::TransferManager
                          ↓
                   daemon::DaemonCore
```

#### 1.1 重构为库模式

**目标**: 将 `daemon/src/main.rs` 重构为可复用的库

**为什么要重构成库**:
- **代码复用**: CLI 和 Tauri 共享同一套核心逻辑
- **测试友好**: 库可以被单元测试，而 binary 难以测试
- **灵活部署**: 可以作为独立守护进程，也可以嵌入到应用中
- **清晰边界**: 强制定义清晰的公开 API

**文件结构**:
```
crates/daemon/
├── Cargo.toml          # 更新: 添加 [[bin]]
└── src/
    ├── lib.rs          # 新增：库入口，导出公开 API
    ├── core.rs         # 新增：DaemonCore 核心逻辑
    ├── event.rs        # 现有：事件定义
    └── bin/
        └── airdropd.rs # 可选：独立守护进程（CLI 模式）
```

**Cargo.toml 配置**:
```toml
[package]
name = "daemon"
version = "0.1.0"
edition = "2021"

[lib]
name = "daemon"
path = "src/lib.rs"

# 可选：保留 CLI 版本
[[bin]]
name = "airdropd"
path = "src/bin/airdropd.rs"
required-features = ["cli"]

[features]
default = []
cli = ["clap"]  # CLI 专用依赖

[dependencies]
tokio = { version = "1", features = ["full"] }
discovery = { path = "../discovery" }
session = { path = "../session" }
transfer = { path = "../transfer" }
anyhow = "1"
tracing = "0.1"

# CLI 专用
clap = { version = "4", features = ["derive"], optional = true }
```

**核心代码** ([crates/daemon/src/core.rs](crates/daemon/src/core.rs)):
```rust
use discovery::{Discovery, Peer};  // Peer 来自 discovery
use session::{SessionManager, SessionEvent};
use transfer::{TransferManager, TransferEvent};
use tokio::sync::mpsc;
use std::path::PathBuf;
use std::time::Duration;
use anyhow::Result;

/// Daemon 核心，管理所有子模块的生命周期
pub struct DaemonCore {
    // 子模块
    discovery: Discovery,
    session_manager: SessionManager,
    transfer_manager: TransferManager,

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
    pub fn new(device_name: String, bind_port: u16, download_dir: PathBuf) -> Result<Self> {
        // 1. 创建事件通道
        let (session_tx, session_rx) = mpsc::channel(100);
        let (transfer_tx, transfer_rx) = mpsc::channel(100);
        let (daemon_tx, daemon_rx) = mpsc::channel(100);

        // 2. 初始化 Discovery
        let discovery = Discovery::new(device_name.clone(), bind_port)?;

        // 3. 初始化 SessionManager
        let session_manager = SessionManager::new(session_tx);

        // 4. 初始化 TransferManager（自动接收）
        let transfer_manager = TransferManager::new(
            download_dir,
            bind_port,
            transfer_tx,
        )?;

        Ok(Self {
            discovery,
            session_manager,
            transfer_manager,
            session_rx,
            transfer_rx,
            daemon_tx,
            daemon_rx,
        })
    }

    /// 运行一次事件循环迭代（非阻塞）
    ///
    /// 这是 DaemonCore 的心跳函数，应该在一个循环中不断调用
    ///
    /// # 返回值
    /// - `Some(DaemonNotification)`: 有事件需要通知上层
    /// - `None`: 本次迭代没有需要通知的事件
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
                    TransferEvent::FileReceived { from, file, size } => {
                        tracing::info!("收到文件: {} 来自 {} ({}bytes)",
                            file.display(), from, size);
                    }
                    TransferEvent::SendProgress { to, progress } => {
                        tracing::debug!("发送进度: {} {}%", to, progress);
                    }
                    _ => {}
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
                    tracing::error!("发送文件失败: {}", e);
                }
            }
        }
    }

    /// 内部发送文件逻辑
    async fn send_file_internal(&self, peer_name: &str, file: PathBuf) -> Result<()> {
        // 1. 查找目标设备
        let peer = self.session_manager.find_peer_by_name(peer_name)
            .ok_or_else(|| anyhow::anyhow!("设备不在线: {}", peer_name))?;

        // 2. 检查文件是否存在
        if !file.exists() {
            return Err(anyhow::anyhow!("文件不存在: {}", file.display()));
        }

        // 3. 发送文件
        self.transfer_manager.send_file(peer.addr, file).await?;

        Ok(())
    }

    /// 公开 API：发送文件
    ///
    /// # 参数
    /// - `peer_name`: 目标设备名称
    /// - `file`: 要发送的文件路径
    pub async fn send_file(&self, peer_name: &str, file: PathBuf) -> Result<()> {
        self.daemon_tx.send(DaemonEvent::SendFile {
            peer_name: peer_name.to_string(),
            file,
        }).await?;
        Ok(())
    }

    /// 公开 API：获取在线设备列表
    pub fn get_online_peers(&self) -> Vec<Peer> {
        self.session_manager.get_online_peers()
    }

    /// 公开 API：获取本设备信息
    pub fn get_device_info(&self) -> DeviceInfo {
        DeviceInfo {
            name: self.discovery.device_name.clone(),
            port: self.discovery.port,
        }
    }
}

/// Daemon 通知（需要传递给 UI 的事件）
#[derive(Debug, Clone)]
pub enum DaemonNotification {
    Session(SessionEvent),
    Transfer(TransferEvent),
}

/// Daemon 命令（UI 发送给 Daemon 的指令）
#[derive(Debug)]
pub enum DaemonEvent {
    SendFile {
        peer_name: String,
        file: PathBuf,
    },
}

/// 设备信息
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub port: u16,
}
```

**lib.rs 导出** ([crates/daemon/src/lib.rs](crates/daemon/src/lib.rs)):
```rust
mod core;
mod event;

// 导出公开 API
pub use core::{DaemonCore, DaemonNotification, DaemonEvent, DeviceInfo};
pub use event::*;

// 重新导出依赖的类型（便于外部使用）
pub use discovery::Peer;           // Peer 来自 discovery
pub use session::SessionEvent;
pub use transfer::TransferEvent;
```

#### 1.2 扩展 SessionManager

**修改** ([crates/session/src/manager.rs](crates/session/src/manager.rs)):

添加以下方法以支持 Tauri 集成：

```rust
impl SessionManager {
    /// 获取所有在线设备
    pub fn get_online_peers(&self) -> Vec<Peer> {
        self.sessions.values()
            .filter(|s| s.is_online())
            .map(|s| s.peer.clone())
            .collect()
    }

    /// 根据设备名查找设备
    pub fn find_peer_by_name(&self, name: &str) -> Option<Peer> {
        self.sessions.values()
            .find(|s| s.peer.name == name && s.is_online())
            .map(|s| s.peer.clone())
    }

    /// 根据设备 ID 查找设备
    pub fn find_peer_by_id(&self, id: &str) -> Option<Peer> {
        self.sessions.get(id)
            .filter(|s| s.is_online())
            .map(|s| s.peer.clone())
    }

    /// 获取在线设备数量
    pub fn online_count(&self) -> usize {
        self.sessions.values()
            .filter(|s| s.is_online())
            .count()
    }
}
```

#### 1.3 确保 Peer 支持序列化

**修改** ([crates/discovery/src/lib.rs](crates/discovery/src/lib.rs)):

Peer 应该定义在 Discovery 模块中（因为 Discovery 是 Peer 的产生者），确保支持序列化以便 Tauri 使用：

```rust
use serde::{Serialize, Deserialize};
use std::net::SocketAddr;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub id: String,           // 设备唯一标识
    pub name: String,         // 设备名称
    pub addr: SocketAddr,     // 设备地址
    #[serde(skip)]
    pub last_seen: Instant,   // 最后一次心跳（不序列化）
}
```

**模块依赖关系**:
- ✅ Discovery 定义 Peer（产生者）
- ✅ Session 使用 `discovery::Peer`（消费者）
- ✅ Transfer 使用 `discovery::Peer`（消费者）
- ✅ Daemon 重新导出 `pub use discovery::Peer;`（便于外部使用）

---

### Phase 2: 初始化 Tauri 项目

#### 2.1 创建 Tauri 应用

```bash
cd crates
npm create tauri-app@latest
# 项目名: tauri-app
# 选择: React + TypeScript + Vite
```

#### 2.2 目录结构

```
crates/tauri-app/
├── src-tauri/              # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs         # Tauri 入口
│       ├── commands.rs     # Tauri Commands
│       ├── state.rs        # 应用状态
│       └── daemon_bridge.rs # Daemon 桥接
│
└── src/                    # 前端 UI
    ├── App.tsx
    ├── components/
    ├── hooks/
    └── lib/
```

---

### Phase 3: Tauri 后端实现

#### 3.1 主入口 ([src-tauri/src/main.rs](src-tauri/src/main.rs))

```rust
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{Manager, SystemTray};

mod commands;
mod state;
mod daemon_bridge;

use state::AppState;

fn main() {
    let tray = SystemTray::new();

    tauri::Builder::default()
        .system_tray(tray)
        .setup(|app| {
            // 初始化应用状态
            let state = AppState::new(app.handle());
            app.manage(state);

            // 启动 Daemon 后台任务
            let app_handle = app.handle();
            tauri::async_runtime::spawn(async move {
                daemon_bridge::run_daemon(app_handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::send_file,
            commands::list_peers,
            commands::get_device_info,
            commands::select_file,
        ])
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                event.window().hide().unwrap();
                api.prevent_close(); // 最小化到托盘
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

#### 3.2 应用状态 ([src-tauri/src/state.rs](src-tauri/src/state.rs))

```rust
use daemon::DaemonCore;
use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::AppHandle;

pub struct AppState {
    pub daemon: Arc<RwLock<Option<DaemonCore>>>,
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
```

#### 3.3 Daemon 桥接 ([src-tauri/src/daemon_bridge.rs](src-tauri/src/daemon_bridge.rs))

```rust
use daemon::{DaemonCore, DaemonNotification, SessionEvent, TransferEvent};
use tauri::{AppHandle, Manager};
use std::path::PathBuf;
use tracing::{info, error};

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

    let daemon = match DaemonCore::new(device_name, 5000, download_dir) {
        Ok(d) => d,
        Err(e) => {
            error!("初始化 DaemonCore 失败: {}", e);
            // 通知前端初始化失败
            app_handle.emit_all("daemon-error", format!("初始化失败: {}", e)).ok();
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
    app_handle.emit_all("daemon-ready", ()).ok();

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
            if let Err(e) = app_handle.emit_all("peer-online", &peer) {
                error!("发送事件失败: {}", e);
            }
        }
        DaemonNotification::Session(SessionEvent::PeerOffline(peer)) => {
            info!("前端事件: peer-offline - {}", peer.name);
            if let Err(e) = app_handle.emit_all("peer-offline", &peer) {
                error!("发送事件失败: {}", e);
            }
        }

        // Transfer 事件
        DaemonNotification::Transfer(event) => {
            match &event {
                TransferEvent::FileReceived { from, file, size } => {
                    info!("前端事件: file-received - {} 来自 {}", file.display(), from);

                    // 序列化为前端友好的格式
                    let payload = serde_json::json!({
                        "from": from,
                        "file": file.to_string_lossy(),
                        "size": size,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });

                    if let Err(e) = app_handle.emit_all("file-received", payload) {
                        error!("发送事件失败: {}", e);
                    }

                    // 系统通知
                    #[cfg(not(target_os = "linux"))]
                    {
                        use tauri::api::notification::Notification;
                        Notification::new(&app_handle.config().tauri.bundle.identifier)
                            .title("收到文件")
                            .body(format!("来自 {}: {}", from, file.display()))
                            .show()
                            .ok();
                    }
                }
                TransferEvent::SendProgress { to, progress } => {
                    // 发送进度更新（节流：每 5% 发送一次）
                    if progress % 5 == 0 {
                        let payload = serde_json::json!({
                            "to": to,
                            "progress": progress,
                        });
                        app_handle.emit_all("send-progress", payload).ok();
                    }
                }
                TransferEvent::SendComplete { to } => {
                    info!("前端事件: send-complete - {}", to);
                    app_handle.emit_all("send-complete", to).ok();
                }
                TransferEvent::SendError { to, error } => {
                    error!("发送失败: {} - {}", to, error);
                    let payload = serde_json::json!({
                        "to": to,
                        "error": error,
                    });
                    app_handle.emit_all("send-error", payload).ok();
                }
            }
        }
    }
}

/// 获取下载目录
fn get_download_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .unwrap_or_else(|| PathBuf::from("./downloads"))
}
```

**关键设计点**:
1. **锁的作用域**: 在 `tick()` 调用内持有锁，外部处理事件（避免死锁）
2. **错误处理**: 初始化失败时通知前端
3. **系统通知**: 收到文件时弹出系统通知
4. **进度节流**: 发送进度每 5% 更新一次（减少事件频率）
5. **结构化日志**: 使用 `tracing` 记录关键操作

#### 3.4 Tauri Commands ([src-tauri/src/commands.rs](src-tauri/src/commands.rs))

```rust
use tauri::State;
use crate::state::AppState;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

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
    let daemon = daemon_lock.as_ref()
        .ok_or_else(|| "Daemon 未初始化".to_string())?;

    // 发送文件
    daemon.send_file(&peer_name, path)
        .await
        .map_err(|e| format!("发送失败: {}", e))?;

    Ok(())
}

/// 获取在线设备列表
#[tauri::command]
pub async fn list_peers(
    state: State<'_, AppState>,
) -> Result<Vec<PeerInfo>, String> {
    let daemon_lock = state.daemon.read().await;
    let daemon = daemon_lock.as_ref()
        .ok_or_else(|| "Daemon 未初始化".to_string())?;

    let peers = daemon.get_online_peers();

    Ok(peers.into_iter().map(|p| PeerInfo {
        id: p.id,
        name: p.name,
        addr: p.addr.to_string(),
    }).collect())
}

/// 获取本设备信息
#[tauri::command]
pub async fn get_device_info(
    state: State<'_, AppState>,
) -> Result<DeviceInfo, String> {
    let daemon_lock = state.daemon.read().await;
    let daemon = daemon_lock.as_ref()
        .ok_or_else(|| "Daemon 未初始化".to_string())?;

    let info = daemon.get_device_info();

    Ok(DeviceInfo {
        name: info.name,
        port: info.port,
    })
}

/// 选择文件（打开文件对话框）
#[tauri::command]
pub async fn select_file() -> Result<Option<String>, String> {
    use tauri::api::dialog::blocking::FileDialogBuilder;

    let file = FileDialogBuilder::new()
        .set_title("选择要发送的文件")
        .pick_file();

    Ok(file.map(|p| p.to_string_lossy().to_string()))
}

/// 选择文件夹（打开文件夹对话框）
#[tauri::command]
pub async fn select_folder() -> Result<Option<String>, String> {
    use tauri::api::dialog::blocking::FileDialogBuilder;

    let folder = FileDialogBuilder::new()
        .set_title("选择要发送的文件夹")
        .pick_folder();

    Ok(folder.map(|p| p.to_string_lossy().to_string()))
}

/// 打开文件所在目录
#[tauri::command]
pub async fn show_in_folder(file_path: String) -> Result<(), String> {
    let path = PathBuf::from(&file_path);

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = path.parent() {
            std::process::Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// 获取下载目录
#[tauri::command]
pub async fn get_download_dir() -> Result<String, String> {
    let dir = dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .ok_or_else(|| "无法获取下载目录".to_string())?;

    Ok(dir.to_string_lossy().to_string())
}
```

**主入口更新** ([src-tauri/src/main.rs](src-tauri/src/main.rs)):
```rust
.invoke_handler(tauri::generate_handler![
    commands::send_file,
    commands::list_peers,
    commands::get_device_info,
    commands::select_file,
    commands::select_folder,
    commands::show_in_folder,
    commands::get_download_dir,
])
```

---

### Phase 4: 前端 UI

#### 4.1 技术栈

- **React 18 + TypeScript**: 类型安全的 UI 开发
- **Vite**: 快速构建和热重载
- **Tailwind CSS**: 实用优先的样式框架
- **shadcn/ui**: 高质量的 UI 组件库（基于 Radix UI）
- **Zustand**: 轻量级状态管理（比 Redux 简单）
- **React Query / SWR**: 异步状态管理（可选）

#### 4.1.1 项目初始化

```bash
# 创建 Tauri 项目
cd crates
npm create tauri-app@latest

# 选项:
# - 项目名: tauri-app
# - 包管理器: pnpm (推荐，比 npm 快)
# - UI 模板: React + TypeScript
# - 构建工具: Vite

# 进入项目
cd tauri-app

# 安装依赖
pnpm install

# 安装额外依赖
pnpm add zustand
pnpm add -D tailwindcss postcss autoprefixer
pnpm add lucide-react  # 图标库
pnpm add clsx tailwind-merge  # 样式工具
pnpm add date-fns  # 时间格式化

# 初始化 Tailwind
npx tailwindcss init -p
```

**Tailwind 配置** ([tailwind.config.js](tailwind.config.js)):
```js
/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}
```

**全局样式** ([src/index.css](src/index.css)):
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  body {
    @apply bg-zinc-50 text-zinc-900;
  }
}
```

#### 4.2 Tauri API 封装 ([src/lib/tauri.ts](src/lib/tauri.ts))

```typescript
import { invoke } from '@tauri-apps/api/tauri';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

// ============ 类型定义 ============

export interface Peer {
  id: string;
  name: string;
  addr: string;
}

export interface DeviceInfo {
  name: string;
  port: number;
}

export interface FileReceivedEvent {
  from: string;
  file: string;
  size: number;
  timestamp: string;
}

export interface SendProgressEvent {
  to: string;
  progress: number;
}

export interface SendErrorEvent {
  to: string;
  error: string;
}

// ============ API 封装 ============

/**
 * Tauri 后端 API 封装
 * 提供类型安全的 Rust 后端调用接口
 */
export const tauriApi = {
  // ---- 设备管理 ----

  /**
   * 获取在线设备列表
   */
  listPeers: async (): Promise<Peer[]> => {
    return invoke<Peer[]>('list_peers');
  },

  /**
   * 获取本设备信息
   */
  getDeviceInfo: async (): Promise<DeviceInfo> => {
    return invoke<DeviceInfo>('get_device_info');
  },

  // ---- 文件传输 ----

  /**
   * 发送文件到指定设备
   * @param peerName 目标设备名称
   * @param filePath 文件路径
   */
  sendFile: async (peerName: string, filePath: string): Promise<void> => {
    return invoke<void>('send_file', { peerName, filePath });
  },

  // ---- 文件选择 ----

  /**
   * 打开文件选择对话框
   * @returns 选中的文件路径，如果取消返回 null
   */
  selectFile: async (): Promise<string | null> => {
    return invoke<string | null>('select_file');
  },

  /**
   * 打开文件夹选择对话框
   * @returns 选中的文件夹路径，如果取消返回 null
   */
  selectFolder: async (): Promise<string | null> => {
    return invoke<string | null>('select_folder');
  },

  /**
   * 在文件管理器中显示文件
   * @param filePath 文件路径
   */
  showInFolder: async (filePath: string): Promise<void> => {
    return invoke<void>('show_in_folder', { filePath });
  },

  /**
   * 获取下载目录
   */
  getDownloadDir: async (): Promise<string> => {
    return invoke<string>('get_download_dir');
  },

  // ---- 事件监听 ----

  events: {
    /**
     * 监听设备上线事件
     */
    onPeerOnline: (callback: (peer: Peer) => void): Promise<UnlistenFn> => {
      return listen<Peer>('peer-online', (event) => callback(event.payload));
    },

    /**
     * 监听设备下线事件
     */
    onPeerOffline: (callback: (peer: Peer) => void): Promise<UnlistenFn> => {
      return listen<Peer>('peer-offline', (event) => callback(event.payload));
    },

    /**
     * 监听文件接收事件
     */
    onFileReceived: (callback: (event: FileReceivedEvent) => void): Promise<UnlistenFn> => {
      return listen<FileReceivedEvent>('file-received', (event) => callback(event.payload));
    },

    /**
     * 监听发送进度事件
     */
    onSendProgress: (callback: (event: SendProgressEvent) => void): Promise<UnlistenFn> => {
      return listen<SendProgressEvent>('send-progress', (event) => callback(event.payload));
    },

    /**
     * 监听发送完成事件
     */
    onSendComplete: (callback: (to: string) => void): Promise<UnlistenFn> => {
      return listen<string>('send-complete', (event) => callback(event.payload));
    },

    /**
     * 监听发送错误事件
     */
    onSendError: (callback: (event: SendErrorEvent) => void): Promise<UnlistenFn> => {
      return listen<SendErrorEvent>('send-error', (event) => callback(event.payload));
    },

    /**
     * 监听 Daemon 就绪事件
     */
    onDaemonReady: (callback: () => void): Promise<UnlistenFn> => {
      return listen('daemon-ready', () => callback());
    },

    /**
     * 监听 Daemon 错误事件
     */
    onDaemonError: (callback: (error: string) => void): Promise<UnlistenFn> => {
      return listen<string>('daemon-error', (event) => callback(event.payload));
    },
  },
};

// ============ 工具函数 ============

/**
 * 格式化文件大小
 */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
}

/**
 * 从文件路径提取文件名
 */
export function getFileName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}
```

#### 4.3 状态管理 Store ([src/store/index.ts](src/store/index.ts))

```typescript
import { create } from 'zustand';
import { Peer, FileReceivedEvent } from '../lib/tauri';

// ============ 类型定义 ============

export interface TransferHistory {
  id: string;
  type: 'sent' | 'received';
  peer: string;
  file: string;
  size: number;
  timestamp: string;
  status: 'pending' | 'progress' | 'completed' | 'failed';
  progress?: number;
  error?: string;
}

export interface AppState {
  // Daemon 状态
  daemonReady: boolean;
  daemonError: string | null;

  // 设备列表
  peers: Peer[];
  selectedPeer: Peer | null;

  // 传输历史
  transferHistory: TransferHistory[];

  // UI 状态
  sidebarOpen: boolean;

  // Actions
  setDaemonReady: (ready: boolean) => void;
  setDaemonError: (error: string | null) => void;
  setPeers: (peers: Peer[]) => void;
  addPeer: (peer: Peer) => void;
  removePeer: (peerId: string) => void;
  selectPeer: (peer: Peer | null) => void;
  addTransfer: (transfer: TransferHistory) => void;
  updateTransferProgress: (id: string, progress: number) => void;
  updateTransferStatus: (id: string, status: TransferHistory['status'], error?: string) => void;
  toggleSidebar: () => void;
}

// ============ Store ============

export const useAppStore = create<AppState>((set) => ({
  // 初始状态
  daemonReady: false,
  daemonError: null,
  peers: [],
  selectedPeer: null,
  transferHistory: [],
  sidebarOpen: true,

  // Daemon 状态
  setDaemonReady: (ready) => set({ daemonReady: ready, daemonError: ready ? null : undefined }),
  setDaemonError: (error) => set({ daemonError: error, daemonReady: false }),

  // 设备管理
  setPeers: (peers) => set({ peers }),
  addPeer: (peer) => set((state) => {
    const exists = state.peers.find((p) => p.id === peer.id);
    return exists ? state : { peers: [...state.peers, peer] };
  }),
  removePeer: (peerId) => set((state) => ({
    peers: state.peers.filter((p) => p.id !== peerId),
    selectedPeer: state.selectedPeer?.id === peerId ? null : state.selectedPeer,
  })),
  selectPeer: (peer) => set({ selectedPeer: peer }),

  // 传输历史
  addTransfer: (transfer) => set((state) => ({
    transferHistory: [transfer, ...state.transferHistory],
  })),
  updateTransferProgress: (id, progress) => set((state) => ({
    transferHistory: state.transferHistory.map((t) =>
      t.id === id ? { ...t, progress, status: 'progress' as const } : t
    ),
  })),
  updateTransferStatus: (id, status, error) => set((state) => ({
    transferHistory: state.transferHistory.map((t) =>
      t.id === id ? { ...t, status, error } : t
    ),
  })),

  // UI 状态
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
}));
```

#### 4.4 自定义 Hooks

**设备列表 Hook** ([src/hooks/usePeers.ts](src/hooks/usePeers.ts)):

```typescript
import { useEffect, useState } from 'react';
import { tauriApi } from '../lib/tauri';
import { useAppStore } from '../store';

/**
 * 设备列表管理 Hook
 * 负责加载设备列表、监听设备上下线事件
 */
export function usePeers() {
  const { peers, addPeer, removePeer, setPeers } = useAppStore();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;

    // 加载初始设备列表
    const loadPeers = async () => {
      try {
        const data = await tauriApi.listPeers();
        if (mounted) {
          setPeers(data);
          setError(null);
        }
      } catch (err) {
        if (mounted) {
          setError(err instanceof Error ? err.message : '加载设备失败');
        }
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    };

    loadPeers();

    // 监听设备上线事件
    const setupListeners = async () => {
      const unlistenOnline = await tauriApi.events.onPeerOnline((peer) => {
        console.log('设备上线:', peer);
        addPeer(peer);
      });

      const unlistenOffline = await tauriApi.events.onPeerOffline((peer) => {
        console.log('设备下线:', peer);
        removePeer(peer.id);
      });

      return () => {
        unlistenOnline();
        unlistenOffline();
      };
    };

    const cleanup = setupListeners();

    return () => {
      mounted = false;
      cleanup.then((fn) => fn());
    };
  }, [addPeer, removePeer, setPeers]);

  const refresh = async () => {
    setLoading(true);
    try {
      const data = await tauriApi.listPeers();
      setPeers(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : '刷新失败');
    } finally {
      setLoading(false);
    }
  };

  return { peers, loading, error, refresh };
}
```

**文件传输 Hook** ([src/hooks/useFileTransfer.ts](src/hooks/useFileTransfer.ts)):

```typescript
import { useEffect } from 'react';
import { tauriApi, getFileName } from '../lib/tauri';
import { useAppStore } from '../store';
import { nanoid } from 'nanoid';

/**
 * 文件传输管理 Hook
 * 负责发送文件、监听传输事件
 */
export function useFileTransfer() {
  const { addTransfer, updateTransferProgress, updateTransferStatus } = useAppStore();

  useEffect(() => {
    const setupListeners = async () => {
      // 监听文件接收事件
      const unlistenReceived = await tauriApi.events.onFileReceived((event) => {
        console.log('收到文件:', event);
        addTransfer({
          id: nanoid(),
          type: 'received',
          peer: event.from,
          file: getFileName(event.file),
          size: event.size,
          timestamp: event.timestamp,
          status: 'completed',
        });
      });

      // 监听发送进度事件
      const unlistenProgress = await tauriApi.events.onSendProgress((event) => {
        // 更新最新的待发送传输记录
        updateTransferProgress(event.to, event.progress);
      });

      // 监听发送完成事件
      const unlistenComplete = await tauriApi.events.onSendComplete((to) => {
        console.log('发送完成:', to);
        updateTransferStatus(to, 'completed');
      });

      // 监听发送错误事件
      const unlistenError = await tauriApi.events.onSendError((event) => {
        console.error('发送错误:', event);
        updateTransferStatus(event.to, 'failed', event.error);
      });

      return () => {
        unlistenReceived();
        unlistenProgress();
        unlistenComplete();
        unlistenError();
      };
    };

    const cleanup = setupListeners();

    return () => {
      cleanup.then((fn) => fn());
    };
  }, [addTransfer, updateTransferProgress, updateTransferStatus]);

  /**
   * 发送文件
   */
  const sendFile = async (peerName: string, filePath: string) => {
    const transferId = nanoid();

    // 添加到传输历史
    addTransfer({
      id: transferId,
      type: 'sent',
      peer: peerName,
      file: getFileName(filePath),
      size: 0, // 后端会通过事件更新
      timestamp: new Date().toISOString(),
      status: 'pending',
    });

    try {
      await tauriApi.sendFile(peerName, filePath);
    } catch (err) {
      updateTransferStatus(transferId, 'failed', err instanceof Error ? err.message : '发送失败');
      throw err;
    }
  };

  return { sendFile };
}
```

#### 4.4 主界面 ([src/App.tsx](src/App.tsx))

```typescript
import { usePeers } from './hooks/usePeers';

function App() {
  const { peers, loading } = usePeers();
  const [selectedPeer, setSelectedPeer] = useState<Peer | null>(null);

  const handleSendFile = async () => {
    if (!selectedPeer) return;

    const filePath = await tauriApi.selectFile();
    if (filePath) {
      await tauriApi.sendFile(selectedPeer.name, filePath);
    }
  };

  return (
    <div className="h-screen flex flex-col">
      <header className="bg-zinc-900 text-white p-4">
        <h1 className="text-xl font-bold">Airdrop</h1>
      </header>

      <main className="flex-1 flex">
        <aside className="w-64 bg-zinc-50 border-r p-4">
          <h2 className="font-semibold mb-4">在线设备 ({peers.length})</h2>
          {loading ? (
            <p>加载中...</p>
          ) : (
            <div className="space-y-2">
              {peers.map((peer) => (
                <button
                  key={peer.id}
                  onClick={() => setSelectedPeer(peer)}
                  className={`w-full p-3 text-left rounded ${
                    selectedPeer?.id === peer.id
                      ? 'bg-blue-500 text-white'
                      : 'bg-white hover:bg-gray-100'
                  }`}
                >
                  <div className="font-medium">{peer.name}</div>
                  <div className="text-sm opacity-70">{peer.addr}</div>
                </button>
              ))}
            </div>
          )}
        </aside>

        <section className="flex-1 p-6 flex flex-col items-center justify-center">
          {selectedPeer ? (
            <>
              <h2 className="text-2xl mb-4">发送文件到 {selectedPeer.name}</h2>
              <button
                onClick={handleSendFile}
                className="px-6 py-3 bg-blue-500 text-white rounded-lg hover:bg-blue-600"
              >
                选择文件
              </button>
            </>
          ) : (
            <p className="text-gray-500">请选择一个设备</p>
          )}
        </section>
      </main>
    </div>
  );
}

export default App;
```

---

## Phase 5: 调试与优化

### 5.1 日志配置

**Rust 后端日志** ([src-tauri/src/main.rs](src-tauri/src/main.rs)):

```rust
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() {
    // 初始化日志
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env()
            .add_directive("airdrop=debug".parse().unwrap())
            .add_directive("daemon=debug".parse().unwrap())
        )
        .init();

    // ... Tauri 初始化
}
```

**前端日志** ([src/lib/logger.ts](src/lib/logger.ts)):

```typescript
const isDev = import.meta.env.DEV;

export const logger = {
  debug: (...args: any[]) => {
    if (isDev) console.log('[DEBUG]', ...args);
  },
  info: (...args: any[]) => {
    console.log('[INFO]', ...args);
  },
  error: (...args: any[]) => {
    console.error('[ERROR]', ...args);
  },
};
```

### 5.2 开发模式运行

```bash
# 启动开发服务器（带热重载）
pnpm tauri dev

# 只启动前端（调试 UI）
pnpm dev

# 构建前端（不运行）
pnpm build
```

### 5.3 性能优化

**减少重渲染**:
```typescript
// 使用 React.memo 包裹组件
export const PeerCard = React.memo(({ peer }: { peer: Peer }) => {
  // ...
});

// 使用 useCallback 缓存回调
const handleSend = useCallback(async () => {
  // ...
}, [selectedPeer]);
```

**事件节流**:
```typescript
// 使用 lodash/throttle 或自定义节流
import { throttle } from 'lodash-es';

const handleProgress = useMemo(
  () => throttle((event: SendProgressEvent) => {
    updateProgress(event);
  }, 200),
  []
);
```

---

## Phase 6: 打包与部署

### 6.1 Tauri 配置

**tauri.conf.json 关键配置**:

```json
{
  "build": {
    "beforeBuildCommand": "pnpm build",
    "beforeDevCommand": "pnpm dev",
    "devPath": "http://localhost:5173",
    "distDir": "../dist"
  },
  "package": {
    "productName": "Airdrop",
    "version": "0.1.0"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "shell": {
        "open": true
      },
      "dialog": {
        "open": true,
        "save": true
      },
      "fs": {
        "all": false,
        "readFile": true,
        "writeFile": true,
        "exists": true
      },
      "notification": {
        "all": true
      }
    },
    "bundle": {
      "active": true,
      "identifier": "com.airdrop.app",
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ],
      "resources": [],
      "externalBin": [],
      "copyright": "",
      "category": "Utility",
      "shortDescription": "本地网络文件快传工具",
      "longDescription": "",
      "deb": {
        "depends": []
      },
      "macOS": {
        "frameworks": [],
        "minimumSystemVersion": "10.13",
        "exceptionDomain": "",
        "signingIdentity": null,
        "entitlements": null
      },
      "windows": {
        "certificateThumbprint": null,
        "digestAlgorithm": "sha256",
        "timestampUrl": ""
      }
    },
    "systemTray": {
      "iconPath": "icons/icon.png",
      "iconAsTemplate": true,
      "menuOnLeftClick": false
    },
    "windows": [
      {
        "title": "Airdrop",
        "width": 1200,
        "height": 800,
        "resizable": true,
        "fullscreen": false,
        "decorations": true,
        "minWidth": 800,
        "minHeight": 600
      }
    ]
  }
}
```

### 6.2 打包命令

```bash
# 开发构建（未签名）
pnpm tauri build

# macOS 签名构建
pnpm tauri build -- --target universal-apple-darwin

# Windows 构建
pnpm tauri build -- --target x86_64-pc-windows-msvc

# Linux 构建
pnpm tauri build -- --target x86_64-unknown-linux-gnu
```

**输出目录**:
- macOS: `src-tauri/target/release/bundle/dmg/`
- Windows: `src-tauri/target/release/bundle/msi/`
- Linux: `src-tauri/target/release/bundle/appimage/`

### 6.3 代码签名

**macOS 签名**:
```bash
# 设置环境变量
export APPLE_CERTIFICATE="Developer ID Application: Your Name (XXXXXXXXXX)"
export APPLE_ID="your@email.com"
export APPLE_PASSWORD="app-specific-password"

# 打包 + 签名
pnpm tauri build
```

**Windows 签名**:
```bash
# 使用证书签名
signtool sign /f certificate.pfx /p password /tr http://timestamp.digicert.com Airdrop.exe
```

### 6.4 自动更新

**Cargo.toml 添加依赖**:
```toml
[dependencies]
tauri = { version = "1", features = ["updater"] }
```

**tauri.conf.json 配置**:
```json
{
  "tauri": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://releases.myapp.com/{{target}}/{{current_version}}"
      ],
      "dialog": true,
      "pubkey": "YOUR_PUBLIC_KEY"
    }
  }
}
```

---

## 实施顺序

### Week 1: 后端重构
1. ✅ 重构 Daemon 为库模式 (2 days)
   - 创建 lib.rs 和 core.rs
   - 实现 DaemonCore 结构体
   - 添加 tick() 事件循环
2. ✅ 扩展 SessionManager (1 day)
   - 添加 get_online_peers()
   - 添加 find_peer_by_name()
3. ✅ 测试库模式 (1 day)
   - 编写单元测试
   - 验证事件流

### Week 2: Tauri 集成
4. ✅ 初始化 Tauri 项目 (1 day)
   - 创建项目结构
   - 配置 Cargo.toml
5. ✅ 实现 Daemon 桥接 (2 days)
   - daemon_bridge.rs
   - 事件转发逻辑
6. ✅ 实现 Tauri Commands (1 day)
   - send_file, list_peers 等
   - 错误处理

### Week 3: 前端开发
7. ✅ 搭建前端基础架构 (1 day)
   - 安装依赖
   - 配置 Tailwind
   - 创建 Store
8. ✅ 实现设备列表组件 (2 days)
   - PeerList 组件
   - usePeers Hook
9. ✅ 实现文件传输界面 (2 days)
   - 文件选择
   - 发送逻辑
   - 进度显示

### Week 4: 功能完善
10. ✅ 事件监听和通知 (2 days)
    - 接收文件通知
    - 系统通知集成
11. ✅ 系统托盘集成 (1 day)
    - 托盘菜单
    - 最小化行为
12. ✅ 打包和测试 (2 days)
    - 多平台打包
    - 端到端测试

---

## 常见问题与解决方案

### Q1: Daemon 阻塞 Tauri 主线程？

**问题**: `daemon.tick()` 在异步任务中运行，不会阻塞主线程

**解决方案**:
- Daemon 运行在 `tauri::async_runtime` 中
- 事件通过 `emit_all()` 非阻塞发送到前端
- 使用 `RwLock` 避免死锁

### Q2: 前端如何处理大量设备上下线事件？

**问题**: 设备频繁上下线导致 UI 抖动

**解决方案**:
```typescript
// 使用防抖处理设备列表更新
const debouncedUpdatePeers = useMemo(
  () => debounce((peers: Peer[]) => {
    setPeers(peers);
  }, 300),
  []
);
```

### Q3: 传输进度更新太频繁？

**问题**: 每 1% 都发送事件导致性能问题

**解决方案**:
- 后端节流：每 5% 发送一次
- 前端节流：使用 throttle 处理
- 只在 UI 可见时更新

### Q4: 如何处理文件权限错误？

**问题**: macOS/Linux 下载目录权限不足

**解决方案**:
```rust
// 自动降级到用户目录
let download_dir = dirs::download_dir()
    .filter(|p| p.exists() && is_writable(p))
    .or_else(|| dirs::home_dir().map(|h| h.join("AirdropDownloads")))
    .unwrap_or_else(|| PathBuf::from("./downloads"));

// 确保目录存在
std::fs::create_dir_all(&download_dir)?;
```

### Q5: Windows Defender 报毒？

**问题**: 未签名的应用被标记为病毒

**解决方案**:
1. 购买代码签名证书
2. 使用 SignTool 签名
3. 提交到 Microsoft SmartScreen 白名单

### Q6: macOS Gatekeeper 阻止运行？

**问题**: "App is damaged and can't be opened"

**解决方案**:
```bash
# 用户手动移除隔离属性
xattr -cr /Applications/Airdrop.app

# 或：使用签名证书构建
codesign --force --deep --sign "Developer ID" Airdrop.app
```

---

## 关键优势

1. **代码复用**: Daemon 逻辑被 CLI 和 Tauri 共享
   - 减少重复代码
   - 一次修复，多处受益
   - 便于维护

2. **事件驱动**: 利用现有的 mpsc 架构，无缝集成
   - 松耦合设计
   - 易于扩展
   - 天然支持并发

3. **异步优先**: Tokio + Tauri 完美配合
   - 非阻塞 I/O
   - 高并发性能
   - UI 始终响应

4. **跨平台**: 一次编写，Windows/macOS/Linux 都支持
   - 统一的代码库
   - 原生性能
   - 系统级集成

5. **现代化 UI**: React + TypeScript + Tailwind
   - 类型安全
   - 组件化
   - 易于定制

6. **安全性**: Tauri 的安全模型
   - 最小权限原则
   - 白名单 API
   - 沙箱隔离

---

## 技术决策对比

### 为什么选 Tauri 而不是 Electron？

| 特性 | Tauri | Electron |
|------|-------|----------|
| 二进制大小 | ~5 MB | ~150 MB |
| 内存占用 | ~50 MB | ~200 MB |
| 启动速度 | 快 | 慢 |
| 安全性 | 高（白名单） | 中（全开放） |
| 跨平台 | ✅ | ✅ |
| 热更新 | ✅ | ✅ |

### 为什么用 Zustand 而不是 Redux？

| 特性 | Zustand | Redux |
|------|---------|-------|
| 学习曲线 | 低 | 高 |
| 样板代码 | 少 | 多 |
| TypeScript | 原生支持 | 需配置 |
| 性能 | 好 | 好 |
| 社区 | 中 | 大 |

**结论**: 对于中小型应用，Zustand 更轻量、更易用

---

## 下一步行动

### 推荐方案 1: 按阶段实施（稳妥）

**适合**: 团队开发、长期项目

**步骤**:
1. ✅ Week 1: 重构 Daemon（确保核心稳定）
2. ✅ Week 2: Tauri 集成（建立桥接）
3. ✅ Week 3: 前端开发（实现 UI）
4. ✅ Week 4: 测试打包（完善细节）

**命令**:
```bash
# 开始 Phase 1
cd crates/daemon
mkdir -p src/bin
touch src/lib.rs src/core.rs src/bin/airdropd.rs
```

### 推荐方案 2: 快速原型（快速验证）

**适合**: 个人项目、快速 MVP

**步骤**:
1. 直接创建 Tauri 项目
2. 临时嵌入 Daemon 代码（不分离）
3. 跑通基本流程
4. 后续再重构

**命令**:
```bash
# 创建 Tauri 项目
cd crates
npm create tauri-app@latest
cd tauri-app

# 添加 daemon 依赖
cd src-tauri
cargo add daemon --path ../../daemon
```

### 推荐方案 3: 仅 CLI 优化（保守）

**适合**: 暂时不需要 GUI

**步骤**:
1. 保持当前 CLI 架构
2. 优化 Daemon 代码结构
3. 未来需要时再集成 Tauri

**命令**:
```bash
# 改进现有 CLI
cd crates/daemon
cargo add clap --features derive
```

---

## 我的建议

根据你的项目状态：
- **已完成核心功能**（Discovery + Session + Transfer）
- **架构清晰**（事件驱动）
- **代码质量高**（模块化设计）

**建议：采用方案 1（按阶段实施）**

**理由**:
1. 你的代码已经很接近库模式，重构成本低
2. 分阶段实施风险小，每步都可验证
3. 最终产物质量高，易于维护

**具体下一步**:
```bash
# 1. 创建库模式文件结构
cd /Users/colin/Desktop/rust/airdrop/crates/daemon
mkdir -p src/bin
touch src/lib.rs src/core.rs

# 2. 让我帮你实现 Phase 1（Daemon 重构）
# 3. 然后初始化 Tauri 项目
# 4. 逐步实现前端
```

---

## 让我们开始吧！

选择一个方案，告诉我：

**A.** "开始 Phase 1 重构" - 我会帮你创建文件并实现 DaemonCore
**B.** "快速原型验证" - 我会直接创建 Tauri 项目并跑通流程
**C.** "还有问题想问" - 我会回答你的疑问
**D.** "暂时不需要 GUI" - 我会帮你优化现有 CLI

你的选择是？
