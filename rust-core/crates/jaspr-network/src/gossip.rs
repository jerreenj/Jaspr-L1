//! Gossip protocol for block and transaction propagation

use crate::peer::{PeerId, PeerManager};
use crate::message::{NetworkMessage, MessageType};
use jaspr_types::{Block, SignedTransaction, HashValue};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::time::{Instant, Duration};

/// Seen item for deduplication
struct SeenItem {
    first_seen: Instant,
    from_peers: HashSet<PeerId>,
}

/// Gossip protocol for propagating blocks and transactions
pub struct GossipProtocol {
    /// Peer manager reference
    peer_manager: std::sync::Arc<PeerManager>,
    /// Seen blocks (hash -> info)
    seen_blocks: RwLock<HashMap<HashValue, SeenItem>>,
    /// Seen transactions (hash -> info)
    seen_txs: RwLock<HashMap<HashValue, SeenItem>>,
    /// Maximum seen cache size
    max_cache_size: usize,
    /// Item expiry time
    expiry_time: Duration,
}

impl GossipProtocol {
    /// Create new gossip protocol
    pub fn new(peer_manager: std::sync::Arc<PeerManager>) -> Self {
        Self {
            peer_manager,
            seen_blocks: RwLock::new(HashMap::new()),
            seen_txs: RwLock::new(HashMap::new()),
            max_cache_size: 10000,
            expiry_time: Duration::from_secs(300), // 5 minutes
        }
    }
    
    /// Check if block has been seen
    pub fn has_seen_block(&self, hash: &HashValue) -> bool {
        self.seen_blocks.read().contains_key(hash)
    }
    
    /// Check if transaction has been seen
    pub fn has_seen_tx(&self, hash: &HashValue) -> bool {
        self.seen_txs.read().contains_key(hash)
    }
    
    /// Mark block as seen from peer
    pub fn mark_block_seen(&self, hash: HashValue, from: PeerId) {
        let mut seen = self.seen_blocks.write();
        
        if let Some(item) = seen.get_mut(&hash) {
            item.from_peers.insert(from);
        } else {
            // Evict old entries if at capacity
            if seen.len() >= self.max_cache_size {
                self.evict_old(&mut seen);
            }
            
            let mut item = SeenItem {
                first_seen: Instant::now(),
                from_peers: HashSet::new(),
            };
            item.from_peers.insert(from);
            seen.insert(hash, item);
        }
    }
    
    /// Mark transaction as seen from peer
    pub fn mark_tx_seen(&self, hash: HashValue, from: PeerId) {
        let mut seen = self.seen_txs.write();
        
        if let Some(item) = seen.get_mut(&hash) {
            item.from_peers.insert(from);
        } else {
            if seen.len() >= self.max_cache_size {
                self.evict_old(&mut seen);
            }
            
            let mut item = SeenItem {
                first_seen: Instant::now(),
                from_peers: HashSet::new(),
            };
            item.from_peers.insert(from);
            seen.insert(hash, item);
        }
    }
    
    /// Get peers to gossip block to (excluding those who've already seen it)
    pub fn get_block_gossip_targets(&self, hash: &HashValue) -> Vec<PeerId> {
        let seen = self.seen_blocks.read();
        let already_seen = seen.get(hash)
            .map(|item| &item.from_peers)
            .cloned()
            .unwrap_or_default();
        
        self.peer_manager.connected_peers()
            .into_iter()
            .filter(|p| !already_seen.contains(&p.id))
            .map(|p| p.id)
            .collect()
    }
    
    /// Get peers to gossip transaction to
    pub fn get_tx_gossip_targets(&self, hash: &HashValue) -> Vec<PeerId> {
        let seen = self.seen_txs.read();
        let already_seen = seen.get(hash)
            .map(|item| &item.from_peers)
            .cloned()
            .unwrap_or_default();
        
        self.peer_manager.connected_peers()
            .into_iter()
            .filter(|p| !already_seen.contains(&p.id))
            .map(|p| p.id)
            .collect()
    }
    
    /// Process incoming block
    pub fn on_block_received(&self, block: &Block, from: PeerId) -> bool {
        let hash = block.hash();
        
        if self.has_seen_block(&hash) {
            self.mark_block_seen(hash, from);
            return false; // Already seen
        }
        
        self.mark_block_seen(hash, from);
        true // New block
    }
    
    /// Process incoming transaction
    pub fn on_tx_received(&self, tx: &SignedTransaction, from: PeerId) -> bool {
        let hash = tx.hash();
        
        if self.has_seen_tx(&hash) {
            self.mark_tx_seen(hash, from);
            return false; // Already seen
        }
        
        self.mark_tx_seen(hash, from);
        true // New transaction
    }
    
    /// Evict old entries from cache
    fn evict_old(&self, cache: &mut HashMap<HashValue, SeenItem>) {
        let expiry = self.expiry_time;
        cache.retain(|_, item| item.first_seen.elapsed() < expiry);
        
        // If still too full, remove oldest
        if cache.len() >= self.max_cache_size {
            let mut entries: Vec<_> = cache.iter()
                .map(|(k, v)| (*k, v.first_seen))
                .collect();
            entries.sort_by(|a, b| a.1.cmp(&b.1));
            
            let to_remove = entries.len() - (self.max_cache_size / 2);
            for (hash, _) in entries.into_iter().take(to_remove) {
                cache.remove(&hash);
            }
        }
    }
    
    /// Clear all caches
    pub fn clear(&self) {
        self.seen_blocks.write().clear();
        self.seen_txs.write().clear();
    }
    
    /// Get stats
    pub fn stats(&self) -> GossipStats {
        GossipStats {
            seen_blocks: self.seen_blocks.read().len(),
            seen_txs: self.seen_txs.read().len(),
        }
    }
}

/// Gossip statistics
#[derive(Clone, Debug)]
pub struct GossipStats {
    pub seen_blocks: usize,
    pub seen_txs: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    fn create_test_protocol() -> GossipProtocol {
        let local_id = PeerId::from_bytes([1u8; 32]);
        let peer_manager = Arc::new(PeerManager::new(local_id, 10));
        GossipProtocol::new(peer_manager)
    }
    
    #[test]
    fn test_block_seen() {
        let protocol = create_test_protocol();
        let hash = HashValue::sha256(b"block1");
        let peer = PeerId::from_bytes([2u8; 32]);
        
        assert!(!protocol.has_seen_block(&hash));
        
        protocol.mark_block_seen(hash, peer);
        
        assert!(protocol.has_seen_block(&hash));
    }
    
    #[test]
    fn test_tx_seen() {
        let protocol = create_test_protocol();
        let hash = HashValue::sha256(b"tx1");
        let peer = PeerId::from_bytes([2u8; 32]);
        
        assert!(!protocol.has_seen_tx(&hash));
        
        protocol.mark_tx_seen(hash, peer);
        
        assert!(protocol.has_seen_tx(&hash));
    }
}
