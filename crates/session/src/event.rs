use discovery::Peer;

#[derive(Debug)]
pub enum SessionEvent {
    PeerOnline(Peer),
    PeerOffline(Peer),
}
