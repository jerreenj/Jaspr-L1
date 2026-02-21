//! Block synchronization service
//!
//! Handles synchronizing the blockchain state from peers

use jaspr_types::{Block, HashValue, BlockHeight};
use jaspr_state::BlockStore;
use crate::peer::{PeerId, PeerManager, PeerInfo};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{info, warn, debug, error};

/// Sync configuration
#[derive(Clone, Debug)]
pub struct SyncConfig {
    /// Batch size for block requests
    pub batch_size: u64,
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
    /// Request timeout
    pub request_timeout: Duration,
    /// Maximum retries per request
    pub max_retries: u32,
    /// Sync check interval
    pub sync_check_interval: Duration,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            max_concurrent_requests: 5,
            request_timeout: Duration::from_secs(30),
            max_retries: 3,
            sync_check_interval: Duration::from_secs(10),
        }
    }
}

/// Sync state
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncState {
    /// Idle (not syncing)
    Idle,
    /// Finding best peer
    FindingPeers,
    /// Downloading headers
    DownloadingHeaders,
    /// Downloading blocks
    DownloadingBlocks,
    /// Processing blocks
    Processing,
    /// Synced
    Synced,
    /// Failed
    Failed(String),
}

/// Block request tracking
#[derive(Clone, Debug)]
struct BlockRequest {
    /// Start height
    start: BlockHeight,
    /// End height
    end: BlockHeight,
    /// Peer we requested from
    peer_id: PeerId,
    /// Request time
    requested_at: Instant,
    /// Retry count
    retries: u32,
}

/// Sync statistics
#[derive(Clone, Debug, Default)]
pub struct SyncStats {
    /// Current sync state
    pub state: String,
    /// Local height
    pub local_height: BlockHeight,
    /// Target height
    pub target_height: BlockHeight,
    /// Blocks downloaded
    pub blocks_downloaded: u64,
    /// Blocks processed
    pub blocks_processed: u64,
    /// Current download speed (blocks/sec)
    pub download_speed: f64,
    /// Estimated time remaining (seconds)
    pub eta_seconds: u64,
    /// Active requests
    pub active_requests: usize,
    /// Failed requests
    pub failed_requests: u64,
}

/// Block sync service
pub struct BlockSyncService {
    config: SyncConfig,
    /// Peer manager
    peer_manager: Arc<PeerManager>,
    /// Block store
    block_store: Arc<BlockStore>,
    /// Current sync state
    state: RwLock<SyncState>,
    /// Target height to sync to
    target_height: RwLock<Option<BlockHeight>>,
    /// Best peer for syncing
    sync_peer: RwLock<Option<PeerId>>,
    /// Pending requests
    pending_requests: RwLock<HashMap<BlockHeight, BlockRequest>>,
    /// Received blocks awaiting processing
    pending_blocks: RwLock<VecDeque<Block>>,
    /// Heights we've received
    received_heights: RwLock<HashSet<BlockHeight>>,
    /// Stats
    stats: RwLock<SyncStats>,
    /// Start time
    start_time: RwLock<Option<Instant>>,
}

impl BlockSyncService {
    /// Create new sync service
    pub fn new(
        config: SyncConfig,
        peer_manager: Arc<PeerManager>,
        block_store: Arc<BlockStore>,
    ) -> Self {
        Self {
            config,
            peer_manager,
            block_store,
            state: RwLock::new(SyncState::Idle),
            target_height: RwLock::new(None),
            sync_peer: RwLock::new(None),
            pending_requests: RwLock::new(HashMap::new()),
            pending_blocks: RwLock::new(VecDeque::new()),
            received_heights: RwLock::new(HashSet::new()),
            stats: RwLock::new(SyncStats::default()),
            start_time: RwLock::new(None),
        }
    }
    
    /// Check if sync is needed
    pub fn needs_sync(&self) -> bool {
        let local_height = self.block_store.get_latest_height()
            .ok()
            .flatten()
            .unwrap_or(0);
        
        // Check peer heights
        let peers = self.peer_manager.connected_peers();
        for peer in peers {
            if peer.chain_height > local_height + 1 {
                return true;
            }
        }
        
        false
    }
    
    /// Start synchronization
    pub fn start_sync(&self) -> Result<(), String> {
        if *self.state.read() != SyncState::Idle {
            return Err("Already syncing".to_string());
        }
        
        info!("Starting block synchronization");
        
        // Find best peer
        *self.state.write() = SyncState::FindingPeers;
        
        let best_peer = self.find_best_peer()?;
        let target = best_peer.chain_height;
        
        *self.sync_peer.write() = Some(best_peer.id);
        *self.target_height.write() = Some(target);
        *self.start_time.write() = Some(Instant::now());
        *self.state.write() = SyncState::DownloadingBlocks;
        
        info!(
            peer = %best_peer.id,
            target = target,
            "Found sync peer"
        );
        
        Ok(())
    }
    
    /// Stop synchronization
    pub fn stop_sync(&self) {
        info!("Stopping block synchronization");
        
        *self.state.write() = SyncState::Idle;
        *self.sync_peer.write() = None;
        *self.target_height.write() = None;
        self.pending_requests.write().clear();
        self.pending_blocks.write().clear();
        self.received_heights.write().clear();
    }
    
    /// Find best peer to sync from
    fn find_best_peer(&self) -> Result<PeerInfo, String> {
        let peers = self.peer_manager.sync_peers();
        
        peers.into_iter()
            .next()
            .ok_or_else(|| "No peers available for sync".to_string())
    }
    
    /// Get current sync state
    pub fn sync_state(&self) -> SyncState {
        self.state.read().clone()
    }
    
    /// Is currently syncing
    pub fn is_syncing(&self) -> bool {
        !matches!(*self.state.read(), SyncState::Idle | SyncState::Synced | SyncState::Failed(_))
    }
    
    /// Request next batch of blocks
    pub fn request_next_batch(&self) -> Option<(PeerId, BlockHeight, BlockHeight)> {
        if *self.state.read() != SyncState::DownloadingBlocks {
            return None;
        }
        
        let target = (*self.target_height.read())?;
        let peer_id = (*self.sync_peer.read())?;
        
        let local_height = self.block_store.get_latest_height()
            .ok()
            .flatten()
            .unwrap_or(0);
        
        // Check concurrent request limit
        let pending_count = self.pending_requests.read().len();
        if pending_count >= self.config.max_concurrent_requests {
            return None;
        }
        
        // Find next range to request
        let pending = self.pending_requests.read();
        let received = self.received_heights.read();
        
        let mut start = local_height + 1;
        
        // Skip heights we've already requested or received
        while start <= target {
            if !pending.contains_key(&start) && !received.contains(&start) {
                break;
            }
            start += 1;
        }
        
        if start > target {
            // Check if we're done
            if pending.is_empty() {
                *self.state.write() = SyncState::Synced;
                info!("Block sync complete");
            }
            return None;
        }
        
        let end = (start + self.config.batch_size - 1).min(target);
        
        // Record request
        drop(pending);
        drop(received);
        
        self.pending_requests.write().insert(start, BlockRequest {
            start,
            end,
            peer_id,
            requested_at: Instant::now(),
            retries: 0,
        });
        
        debug!(start = start, end = end, peer = %peer_id, "Requesting blocks");
        
        Some((peer_id, start, end))
    }
    
    /// Handle received blocks
    pub fn on_blocks_received(
        &self,
        _peer_id: PeerId,
        blocks: Vec<Block>,
    ) {
        if blocks.is_empty() {
            return;
        }
        
        let first_height = blocks.first().map(|b| b.height()).unwrap_or(0);
        let last_height = blocks.last().map(|b| b.height()).unwrap_or(0);
        
        debug!(
            first = first_height,
            last = last_height,
            count = blocks.len(),
            "Received blocks"
        );
        
        // Remove from pending
        self.pending_requests.write().remove(&first_height);
        
        // Mark as received
        {
            let mut received = self.received_heights.write();
            for block in &blocks {
                received.insert(block.height());
            }
        }
        
        // Add to pending blocks queue
        {
            let mut pending = self.pending_blocks.write();
            for block in blocks {
                pending.push_back(block);
            }
        }
        
        // Update stats
        {
            let mut stats = self.stats.write();
            stats.blocks_downloaded += (last_height - first_height + 1) as u64;
        }
    }
    
    /// Get next block to process
    pub fn next_block_to_process(&self) -> Option<Block> {
        let local_height = self.block_store.get_latest_height()
            .ok()
            .flatten()
            .unwrap_or(0);
        
        let expected_height = local_height + 1;
        
        let mut pending = self.pending_blocks.write();
        
        // Find the block with expected height
        let pos = pending.iter().position(|b| b.height() == expected_height)?;
        
        pending.remove(pos)
    }
    
    /// Mark block as processed
    pub fn mark_block_processed(&self, height: BlockHeight) {
        let mut stats = self.stats.write();
        stats.blocks_processed += 1;
        stats.local_height = height;
    }
    
    /// Check for timed out requests
    pub fn check_timeouts(&self) {
        let mut pending = self.pending_requests.write();
        let timeout = self.config.request_timeout;
        let max_retries = self.config.max_retries;
        
        let mut to_retry = Vec::new();
        let mut to_remove = Vec::new();
        
        for (height, request) in pending.iter() {
            if request.requested_at.elapsed() > timeout {
                if request.retries < max_retries {
                    to_retry.push(*height);
                } else {
                    to_remove.push(*height);
                }
            }
        }
        
        for height in to_remove {
            pending.remove(&height);
            warn!(height = height, "Block request failed after max retries");
            
            let mut stats = self.stats.write();
            stats.failed_requests += 1;
        }
        
        for height in to_retry {
            if let Some(request) = pending.get_mut(&height) {
                request.retries += 1;
                request.requested_at = Instant::now();
                debug!(height = height, retries = request.retries, "Retrying block request");
            }
        }
    }
    
    /// Get sync progress (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        let local_height = self.block_store.get_latest_height()
            .ok()
            .flatten()
            .unwrap_or(0);
        
        match *self.target_height.read() {
            Some(target) if target > 0 => (local_height as f64) / (target as f64),
            _ => 1.0,
        }
    }
    
    /// Get sync statistics
    pub fn stats(&self) -> SyncStats {
        let mut stats = self.stats.read().clone();
        
        stats.state = format!("{:?}", *self.state.read());
        stats.local_height = self.block_store.get_latest_height()
            .ok()
            .flatten()
            .unwrap_or(0);
        stats.target_height = self.target_height.read().unwrap_or(0);
        stats.active_requests = self.pending_requests.read().len();
        
        // Calculate speed and ETA
        if let Some(start) = *self.start_time.read() {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed > 0.0 && stats.blocks_processed > 0 {
                stats.download_speed = stats.blocks_processed as f64 / elapsed;
                
                let remaining = stats.target_height.saturating_sub(stats.local_height);
                stats.eta_seconds = (remaining as f64 / stats.download_speed) as u64;
            }
        }
        
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jaspr_state::Database;
    
    fn create_test_sync_service() -> BlockSyncService {
        let local_id = PeerId::from_bytes([1u8; 32]);
        let peer_manager = Arc::new(PeerManager::new(local_id, 10));
        
        let db = Database::open_in_memory().unwrap();
        let block_store = Arc::new(BlockStore::new(db));
        
        BlockSyncService::new(
            SyncConfig::default(),
            peer_manager,
            block_store,
        )
    }
    
    #[test]
    fn test_sync_service_creation() {
        let service = create_test_sync_service();
        assert_eq!(service.sync_state(), SyncState::Idle);
        assert!(!service.is_syncing());
    }
    
    #[test]
    fn test_progress_calculation() {
        let service = create_test_sync_service();
        
        *service.target_height.write() = Some(100);
        
        // Initial progress should be based on stored height (0)
        assert!(service.progress() < 0.1);
    }
}
