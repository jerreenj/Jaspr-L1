//! Network service

use crate::peer::{PeerId, PeerManager, PeerInfo, PeerState};
use crate::message::{NetworkMessage, MessageType, HandshakeMessage, StatusMessage};
use crate::gossip::GossipProtocol;
use jaspr_types::{Block, SignedTransaction, HashValue, BlockHeight};
use tokio::sync::mpsc;
use parking_lot::RwLock;
use std::sync::Arc;
use std::net::SocketAddr;
use tracing::{info, warn, debug, error};

/// Network configuration
#[derive(Clone, Debug)]
pub struct NetworkConfig {
    /// Listen address
    pub listen_addr: SocketAddr,
    /// Maximum peers
    pub max_peers: usize,
    /// Chain ID
    pub chain_id: u64,
    /// Bootstrap peers
    pub bootstrap_peers: Vec<SocketAddr>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:30303".parse().unwrap(),
            max_peers: 50,
            chain_id: 1,
            bootstrap_peers: Vec::new(),
        }
    }
}

/// Network events
#[derive(Clone, Debug)]
pub enum NetworkEvent {
    /// New peer connected
    PeerConnected(PeerId),
    /// Peer disconnected
    PeerDisconnected(PeerId),
    /// New block received
    BlockReceived(Block, PeerId),
    /// New transaction received
    TransactionReceived(SignedTransaction, PeerId),
    /// Sync requested
    SyncRequested(PeerId, BlockHeight, BlockHeight),
}

/// Network service
pub struct NetworkService {
    config: NetworkConfig,
    peer_manager: Arc<PeerManager>,
    gossip: Arc<GossipProtocol>,
    /// Current chain height
    chain_height: RwLock<BlockHeight>,
    /// Genesis hash
    genesis_hash: RwLock<HashValue>,
    /// Event sender
    event_tx: Option<mpsc::UnboundedSender<NetworkEvent>>,
    /// Running state
    running: RwLock<bool>,
}

impl NetworkService {
    /// Create new network service
    pub fn new(config: NetworkConfig, local_id: PeerId) -> Self {
        let peer_manager = Arc::new(PeerManager::new(local_id, config.max_peers));
        let gossip = Arc::new(GossipProtocol::new(Arc::clone(&peer_manager)));
        
        Self {
            config,
            peer_manager,
            gossip,
            chain_height: RwLock::new(0),
            genesis_hash: RwLock::new(HashValue::zero()),
            event_tx: None,
            running: RwLock::new(false),
        }
    }
    
    /// Set event channel
    pub fn set_event_channel(&mut self, tx: mpsc::UnboundedSender<NetworkEvent>) {
        self.event_tx = Some(tx);
    }
    
    /// Set chain state
    pub fn set_chain_state(&self, height: BlockHeight, genesis: HashValue) {
        *self.chain_height.write() = height;
        *self.genesis_hash.write() = genesis;
    }
    
    /// Get peer manager
    pub fn peer_manager(&self) -> &Arc<PeerManager> {
        &self.peer_manager
    }
    
    /// Get gossip protocol
    pub fn gossip(&self) -> &Arc<GossipProtocol> {
        &self.gossip
    }
    
    /// Start the network service
    pub async fn start(&self) -> Result<(), String> {
        *self.running.write() = true;
        info!(addr = %self.config.listen_addr, "Network service starting");
        
        // In production, this would:
        // 1. Start TCP listener
        // 2. Connect to bootstrap peers
        // 3. Start peer discovery
        
        Ok(())
    }
    
    /// Stop the network service
    pub async fn stop(&self) {
        *self.running.write() = false;
        info!("Network service stopped");
    }
    
    /// Is running?
    pub fn is_running(&self) -> bool {
        *self.running.read()
    }
    
    /// Connect to peer
    pub async fn connect(&self, addr: SocketAddr) -> Result<PeerId, String> {
        // Generate peer ID (in production, would be from handshake)
        let peer_id = PeerId::from_bytes(jaspr_types::HashValue::sha256(
            format!("{}", addr).as_bytes()
        ).as_bytes().try_into().unwrap());
        
        let mut info = PeerInfo::new(peer_id, addr, true);
        info.set_connected();
        
        self.peer_manager.add_peer(info)?;
        
        // Emit event
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(NetworkEvent::PeerConnected(peer_id));
        }
        
        info!(peer = %peer_id, addr = %addr, "Connected to peer");
        
        Ok(peer_id)
    }
    
    /// Disconnect from peer
    pub fn disconnect(&self, peer_id: &PeerId) {
        if let Some(mut peer) = self.peer_manager.remove_peer(peer_id) {
            peer.set_disconnected();
            
            if let Some(tx) = &self.event_tx {
                let _ = tx.send(NetworkEvent::PeerDisconnected(*peer_id));
            }
            
            info!(peer = %peer_id, "Disconnected from peer");
        }
    }
    
    /// Broadcast block to network
    pub fn broadcast_block(&self, block: &Block) {
        let hash = block.hash();
        let msg = NetworkMessage::new_block(block);
        
        let targets = self.gossip.get_block_gossip_targets(&hash);
        
        debug!(
            block = block.height(),
            targets = targets.len(),
            "Broadcasting block"
        );
        
        for peer_id in targets {
            self.gossip.mark_block_seen(hash, peer_id);
            // In production, would send message to peer
        }
    }
    
    /// Broadcast transaction to network
    pub fn broadcast_transaction(&self, tx: &SignedTransaction) {
        let hash = tx.hash();
        let msg = NetworkMessage::new_transaction(tx);
        
        let targets = self.gossip.get_tx_gossip_targets(&hash);
        
        debug!(
            tx = %hash,
            targets = targets.len(),
            "Broadcasting transaction"
        );
        
        for peer_id in targets {
            self.gossip.mark_tx_seen(hash, peer_id);
            // In production, would send message to peer
        }
    }
    
    /// Handle incoming message
    pub fn handle_message(&self, from: PeerId, msg: NetworkMessage) {
        match msg.msg_type {
            MessageType::Handshake => {
                if let Ok(handshake) = msg.parse_payload::<HandshakeMessage>() {
                    self.handle_handshake(from, handshake);
                }
            }
            MessageType::Ping => {
                // Send pong
                let pong = NetworkMessage::pong(msg.request_id);
                // Would send to peer
            }
            MessageType::NewBlock => {
                if let Ok(block) = msg.parse_payload::<Block>() {
                    self.handle_new_block(from, block);
                }
            }
            MessageType::NewTransaction => {
                if let Ok(tx) = msg.parse_payload::<SignedTransaction>() {
                    self.handle_new_transaction(from, tx);
                }
            }
            MessageType::Status => {
                if let Ok(status) = msg.parse_payload::<StatusMessage>() {
                    self.handle_status(from, status);
                }
            }
            _ => {
                debug!(msg_type = ?msg.msg_type, from = %from, "Unhandled message type");
            }
        }
    }
    
    /// Handle handshake message
    fn handle_handshake(&self, from: PeerId, msg: HandshakeMessage) {
        // Validate chain ID
        if msg.chain_id != self.config.chain_id {
            warn!(
                peer = %from,
                expected = self.config.chain_id,
                got = msg.chain_id,
                "Chain ID mismatch"
            );
            self.disconnect(&from);
            return;
        }
        
        // Update peer info
        self.peer_manager.update_peer(&from, |peer| {
            peer.chain_height = msg.height;
            peer.client_version = msg.client_version.clone();
        });
        
        debug!(peer = %from, height = msg.height, "Handshake received");
    }
    
    /// Handle new block message
    fn handle_new_block(&self, from: PeerId, block: Block) {
        if !self.gossip.on_block_received(&block, from) {
            return; // Already seen
        }
        
        debug!(peer = %from, block = block.height(), "New block received");
        
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(NetworkEvent::BlockReceived(block, from));
        }
    }
    
    /// Handle new transaction message
    fn handle_new_transaction(&self, from: PeerId, tx: SignedTransaction) {
        if !self.gossip.on_tx_received(&tx, from) {
            return; // Already seen
        }
        
        debug!(peer = %from, tx = %tx.hash(), "New transaction received");
        
        if let Some(event_tx) = &self.event_tx {
            let _ = event_tx.send(NetworkEvent::TransactionReceived(tx, from));
        }
    }
    
    /// Handle status message
    fn handle_status(&self, from: PeerId, status: StatusMessage) {
        self.peer_manager.update_peer(&from, |peer| {
            peer.chain_height = status.height;
            peer.touch();
        });
    }
    
    /// Request blocks from peer
    pub fn request_blocks(&self, peer: &PeerId, start: BlockHeight, end: BlockHeight) {
        let msg = NetworkMessage::get_blocks(rand::random(), start, end);
        // Would send to peer
        
        debug!(peer = %peer, start = start, end = end, "Requesting blocks");
    }
    
    /// Get network stats
    pub fn stats(&self) -> NetworkStats {
        NetworkStats {
            connected_peers: self.peer_manager.connected_count(),
            total_peers: self.peer_manager.peer_count(),
            chain_height: *self.chain_height.read(),
            gossip_stats: self.gossip.stats(),
        }
    }
}

/// Network statistics
#[derive(Clone, Debug)]
pub struct NetworkStats {
    pub connected_peers: usize,
    pub total_peers: usize,
    pub chain_height: BlockHeight,
    pub gossip_stats: crate::gossip::GossipStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_service() {
        let local_id = PeerId::from_bytes([1u8; 32]);
        let service = NetworkService::new(NetworkConfig::default(), local_id);
        
        let stats = service.stats();
        assert_eq!(stats.connected_peers, 0);
    }
}
