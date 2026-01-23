use std::time::Instant;

use discovery::Peer;

#[derive(Clone, Debug)]
pub enum PeerState {
    online,
    offline,
}

pub struct Session {
    pub peer: Peer,
    pub state: PeerState,
    pub last_seen: Instant,
}

impl Session {
    pub fn is_online(&self) -> bool {
        matches!(self.state, PeerState::online)
    }
}
