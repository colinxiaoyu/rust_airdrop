use std::path::PathBuf;

pub enum DaemonEvent {
    SendFile { peer_name: String, file: PathBuf },
}
