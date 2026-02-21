//! Peer management

use jaspr_crypto::PublicKey;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Instant, Duration};

/// Peer identifier (public key hash)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PeerId([u8; 32]);

impl PeerId {
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    
    /// Create from public key
    pub fn from_public_key(key: &PublicKey) -> Self {
        use jaspr_types::HashValue;
        let hash = HashValue::sha256(&key.to_bytes());
        Self(*hash.as_bytes())
    }
    
    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    
    /// To hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    
    /// Short display (first 8 chars)
    pub fn short(&self) -> String {
        self.to_hex()[..8].to_string()
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.short())
    }
}

/// Peer connection state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeerState {
    /// Connecting
    Connecting,
    /// Connected and ready
    Connected,
    /// Disconnected
    Disconnected,
    /// Banned
    Banned,
}

/// Peer information
#[derive(Clone, Debug)]
pub struct PeerInfo {
    pub id: PeerId,
    pub address: SocketAddr,
    pub state: PeerState,
    pub public_key: Option<PublicKey>,
    /// Is this peer an outbound connection (we initiated)
    pub outbound: bool,
    /// Last seen timestamp
    pub last_seen: Instant,
    /// Connection time
    pub connected_at: Option<Instant>,
    /// Number of messages received
    pub messages_received: u64,
    /// Number of messages sent
    pub messages_sent: u64,
    /// Peer's reported chain height
    pub chain_height: u64,
    /// Peer's client version
    pub client_version: String,
    /// Latency in milliseconds
    pub latency_ms: Option<u64>,
}

impl PeerInfo {
    /// Create new peer info
    pub fn new(id: PeerId, address: SocketAddr, outbound: bool) -> Self {
        Self {
            id,
            address,
            state: PeerState::Connecting,
            public_key: None,
            outbound,
            last_seen: Instant::now(),
            connected_at: None,
            messages_received: 0,
            messages_sent: 0,
            chain_height: 0,
            client_version: String::new(),
            latency_ms: None,
        }
    }
    
    /// Mark as connected
    pub fn set_connected(&mut self) {
        self.state = PeerState::Connected;
        self.connected_at = Some(Instant::now());
        self.last_seen = Instant::now();
    }
    
    /// Mark as disconnected
    pub fn set_disconnected(&mut self) {
        self.state = PeerState::Disconnected;
    }
    
    /// Update last seen
    pub fn touch(&mut self) {
        self.last_seen = Instant::now();
    }
    
    /// Connection duration
    pub fn connection_duration(&self) -> Option<Duration> {
        self.connected_at.map(|t| t.elapsed())
    }
    
    /// Is connected?
    pub fn is_connected(&self) -> bool {
        self.state == PeerState::Connected
    }
}

/// Peer manager
pub struct PeerManager {
    /// Our peer ID
    local_id: PeerId,
    /// Known peers
    peers: RwLock<HashMap<PeerId, PeerInfo>>,
    /// Maximum peers
    max_peers: usize,
    /// Maximum outbound peers
    max_outbound: usize,
    /// Peer timeout
    peer_timeout: Duration,
}

impl PeerManager {
    /// Create new peer manager
    pub fn new(local_id: PeerId, max_peers: usize) -> Self {
        Self {
            local_id,
            peers: RwLock::new(HashMap::new()),
            max_peers,
            max_outbound: max_peers / 2,
            peer_timeout: Duration::from_secs(300), // 5 minutes
        }
    }
    
    /// Get local peer ID
    pub fn local_id(&self) -> &PeerId {
        &self.local_id
    }
    
    /// Add peer
    pub fn add_peer(&self, info: PeerInfo) -> Result<(), String> {
        let mut peers = self.peers.write();
        
        if peers.len() >= self.max_peers {
            return Err("Max peers reached".to_string());
        }
        
        if info.outbound {
            let outbound_count = peers.values().filter(|p| p.outbound).count();
            if outbound_count >= self.max_outbound {
                return Err("Max outbound peers reached".to_string());
            }
        }
        
        peers.insert(info.id, info);
        Ok(())
    }
    
    /// Remove peer
    pub fn remove_peer(&self, id: &PeerId) -> Option<PeerInfo> {
        self.peers.write().remove(id)
    }
    
    /// Get peer
    pub fn get_peer(&self, id: &PeerId) -> Option<PeerInfo> {
        self.peers.read().get(id).cloned()
    }
    
    /// Update peer
    pub fn update_peer<F>(&self, id: &PeerId, f: F)
    where
        F: FnOnce(&mut PeerInfo),
    {
        if let Some(peer) = self.peers.write().get_mut(id) {
            f(peer);
        }
    }
    
    /// Get all connected peers
    pub fn connected_peers(&self) -> Vec<PeerInfo> {
        self.peers.read()
            .values()
            .filter(|p| p.is_connected())
            .cloned()
            .collect()
    }
    
    /// Get all peers
    pub fn all_peers(&self) -> Vec<PeerInfo> {
        self.peers.read().values().cloned().collect()
    }
    
    /// Peer count
    pub fn peer_count(&self) -> usize {
        self.peers.read().len()
    }
    
    /// Connected peer count
    pub fn connected_count(&self) -> usize {
        self.peers.read()
            .values()
            .filter(|p| p.is_connected())
            .count()
    }
    
    /// Ban peer
    pub fn ban_peer(&self, id: &PeerId) {
        if let Some(peer) = self.peers.write().get_mut(id) {
            peer.state = PeerState::Banned;
        }
    }
    
    /// Prune stale peers
    pub fn prune_stale(&self) {
        let mut peers = self.peers.write();
        let timeout = self.peer_timeout;
        
        peers.retain(|_, peer| {
            if peer.state == PeerState::Banned {
                return false;
            }
            if peer.state == PeerState::Disconnected {
                return peer.last_seen.elapsed() < timeout;
            }
            true
        });
    }
    
    /// Get peers to sync from (highest chain height)
    pub fn sync_peers(&self) -> Vec<PeerInfo> {
        let mut peers = self.connected_peers();
        peers.sort_by(|a, b| b.chain_height.cmp(&a.chain_height));
        peers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    
    fn test_socket_addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)
    }
    
    #[test]
    fn test_peer_manager() {
        let local_id = PeerId::from_bytes([1u8; 32]);
        let manager = PeerManager::new(local_id, 10);
        
        let peer_id = PeerId::from_bytes([2u8; 32]);
        let mut info = PeerInfo::new(peer_id, test_socket_addr(), true);
        info.set_connected();
        
        manager.add_peer(info).unwrap();
        
        assert_eq!(manager.peer_count(), 1);
        assert_eq!(manager.connected_count(), 1);
        
        manager.remove_peer(&peer_id);
        assert_eq!(manager.peer_count(), 0);
    }
    
    #[test]
    fn test_max_peers() {
        let local_id = PeerId::from_bytes([1u8; 32]);
        let manager = PeerManager::new(local_id, 2);
        
        for i in 0..3 {
            let peer_id = PeerId::from_bytes([i + 2; 32]);
            let info = PeerInfo::new(peer_id, test_socket_addr(), false);
            
            if i < 2 {
                assert!(manager.add_peer(info).is_ok());
            } else {
                assert!(manager.add_peer(info).is_err());
            }
        }
    }
}
