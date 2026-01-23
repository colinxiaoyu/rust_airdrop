use std::{net::SocketAddr, path::PathBuf};

pub enum TransferEvent {
    FileReceived {
        file_name: String,
        file_size: u64,
        file_path: PathBuf,
        sender_addr: SocketAddr,
    },

    ReceiveFailed {
        error: String,
        sender_addr: Option<SocketAddr>,
    },
}
