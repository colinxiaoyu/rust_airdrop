use discovery::Peer;

#[derive(Debug, Clone)]
pub enum SessionEvent {
    PeerOnline(Peer),
    PeerOffline(Peer),
}
