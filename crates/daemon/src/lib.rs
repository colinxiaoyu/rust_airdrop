mod core;
mod event;

// 导出公开 API
pub use core::{DaemonCore, DaemonNotification, DeviceInfo};
pub use event::*;

// 重新导出依赖的类型（便于外部使用）
pub use discovery::Peer; // Peer 来自 discovery
pub use session::SessionEvent;
pub use transfer::TransferEvent;
