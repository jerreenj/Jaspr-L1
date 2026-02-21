//! Mempool with priority lanes

use crate::execution::SignedTransaction;
use crate::sentinel::AISentinel;
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::sync::Arc;
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MempoolLane {
    Liquidation,
    Priority,
    Institutional,
    Normal,
    Sponsored,
    Quarantine,
}

struct MempoolEntry {
    tx: SignedTransaction,
    lane: MempoolLane,
    priority: u64,
}

impl Ord for MempoolEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for MempoolEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for MempoolEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for MempoolEntry {}

/// Mempool with partitioned lanes
pub struct Mempool {
    sentinel: Arc<AISentinel>,
    entries: HashMap<String, MempoolEntry>,
    by_sender: HashMap<String, HashSet<String>>,
    queue: BinaryHeap<MempoolEntry>,
}

impl Mempool {
    pub const MAX_SIZE: usize = 10000;
    pub const MAX_PER_SENDER: usize = 100;
    
    pub fn new(sentinel: Arc<AISentinel>) -> Self {
        Self {
            sentinel,
            entries: HashMap::new(),
            by_sender: HashMap::new(),
            queue: BinaryHeap::new(),
        }
    }
    
    /// Add transaction to mempool
    pub fn add(&mut self, tx: SignedTransaction) -> Result<(), String> {
        let tx_hash = tx.hash();
        let sender = tx.transaction.sender.clone();
        
        // Check duplicates
        if self.entries.contains_key(&tx_hash) {
            return Err("Already in mempool".to_string());
        }
        
        // Check size
        if self.entries.len() >= Self::MAX_SIZE {
            return Err("Mempool full".to_string());
        }
        
        // Check sender limit
        let sender_count = self.by_sender.get(&sender).map(|s| s.len()).unwrap_or(0);
        if sender_count >= Self::MAX_PER_SENDER {
            return Err("Too many pending from sender".to_string());
        }
        
        // Determine lane based on transaction
        let lane = self.determine_lane(&tx);
        let priority = self.calculate_priority(&tx, &lane);
        
        let entry = MempoolEntry {
            tx: tx.clone(),
            lane,
            priority,
        };
        
        self.entries.insert(tx_hash.clone(), entry.clone());
        self.by_sender.entry(sender).or_default().insert(tx_hash);
        self.queue.push(entry);
        
        Ok(())
    }
    
    fn determine_lane(&self, tx: &SignedTransaction) -> MempoolLane {
        let inner = &tx.transaction;
        
        // High gas = priority
        if inner.gas_price > 2_000_000_000 {
            return MempoolLane::Priority;
        }
        
        MempoolLane::Normal
    }
    
    fn calculate_priority(&self, tx: &SignedTransaction, lane: &MempoolLane) -> u64 {
        let base = match lane {
            MempoolLane::Liquidation => 10000,
            MempoolLane::Priority => 5000,
            MempoolLane::Institutional => 4000,
            MempoolLane::Normal => 1000,
            MempoolLane::Sponsored => 500,
            MempoolLane::Quarantine => 100,
        };
        
        let gas_component = tx.transaction.gas_price / 1_000_000;
        base + gas_component
    }
    
    /// Get transactions for next block
    pub fn get_for_block(&mut self, max_count: usize) -> Vec<SignedTransaction> {
        let mut result = Vec::new();
        let mut used = HashSet::new();
        
        for entry in &self.queue {
            if result.len() >= max_count {
                break;
            }
            
            let hash = entry.tx.hash();
            if !used.contains(&hash) {
                result.push(entry.tx.clone());
                used.insert(hash);
            }
        }
        
        result
    }
    
    /// Remove transactions
    pub fn remove(&mut self, tx_hashes: &[String]) {
        for hash in tx_hashes {
            if let Some(entry) = self.entries.remove(hash) {
                let sender = &entry.tx.transaction.sender;
                if let Some(set) = self.by_sender.get_mut(sender) {
                    set.remove(hash);
                }
            }
        }
        // Rebuild queue (simplified)
        self.queue = self.entries.values().cloned().collect();
    }
    
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
