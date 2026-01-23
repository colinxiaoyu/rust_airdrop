use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use discovery::Peer;
use tokio::sync::mpsc;

use crate::{
    event::SessionEvent,
    session::{PeerState, Session},
};

pub struct SessionManager {
    sessions: HashMap<String, Session>,
    tx: mpsc::Sender<SessionEvent>,
}

impl SessionManager {
    pub fn new(tx: mpsc::Sender<SessionEvent>) -> Self {
        Self {
            sessions: HashMap::new(),
            tx,
        }
    }

    pub async fn on_peer_discovered(&mut self, peer: Peer) {
        let now = Instant::now();

        match self.sessions.get_mut((&peer.name)) {
            Some(session) => {
                session.last_seen = now;
                session.state = PeerState::online;
            }
            None => {
                let session = Session {
                    peer: peer.clone(),
                    state: PeerState::online,
                    last_seen: now,
                };
                self.sessions.insert(peer.name.clone(), session);
                let _ = self.tx.send(SessionEvent::PeerOnline(peer)).await;
            }
        }
    }

    pub async fn reap_offline(&mut self, timeout: Duration) {
        let now = Instant::now();
        let mut offline = Vec::new();

        for (name, session) in &self.sessions {
            if now.duration_since(session.last_seen) > timeout {
                offline.push(name.clone());
            }
        }

        for name in offline {
            if let Some(session) = self.sessions.remove(&name) {
                let _ = self.tx.send(SessionEvent::PeerOffline(session.peer)).await;
            }
        }
    }

    pub fn get_peer(&self, name: &str) -> Option<&Peer> {
        self.sessions.get(name).map(|s| &s.peer)
    }
}

impl SessionManager {
    /// 获取所有在线设备
    pub fn get_online_peers(&self) -> Vec<Peer> {
        self.sessions
            .values()
            .filter(|s| s.is_online())
            .map(|s| s.peer.clone())
            .collect()
    }

    /// 根据设备名查找设备
    pub fn find_peer_by_name(&self, name: &str) -> Option<Peer> {
        self.sessions
            .values()
            .find(|s| s.peer.name == name && s.is_online())
            .map(|s| s.peer.clone())
    }

    /// 根据设备 ID 查找设备
    pub fn find_peer_by_id(&self, id: &str) -> Option<Peer> {
        self.sessions
            .get(id)
            .filter(|s| s.is_online())
            .map(|s| s.peer.clone())
    }

    /// 获取在线设备数量
    pub fn online_count(&self) -> usize {
        self.sessions.values().filter(|s| s.is_online()).count()
    }
}
