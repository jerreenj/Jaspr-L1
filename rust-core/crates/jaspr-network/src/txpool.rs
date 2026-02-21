//! Transaction Pool (Mempool) implementation
//!
//! Manages pending transactions before they are included in blocks.
//! Features:
//! - Priority queue based on gas price and nonce
//! - Transaction validation and verification
//! - Replacement and eviction policies
//! - Size limits and gas limits

use jaspr_types::{Address, HashValue, SignedTransaction, Transaction, Amount};
use std::collections::{HashMap, BTreeMap, HashSet, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn, debug, error};

/// Transaction pool error
#[derive(Error, Debug)]
pub enum TxPoolError {
    #[error("Transaction already exists: {0}")]
    AlreadyExists(String),
    
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    #[error("Nonce too low: expected {expected}, got {got}")]
    NonceTooLow { expected: u64, got: u64 },
    
    #[error("Nonce gap: expected {expected}, got {got}")]
    NonceGap { expected: u64, got: u64 },
    
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: Amount, available: Amount },
    
    #[error("Gas price too low: minimum {minimum}, got {got}")]
    GasPriceTooLow { minimum: u64, got: u64 },
    
    #[error("Pool is full")]
    PoolFull,
    
    #[error("Transaction too large: {size} bytes (max {max})")]
    TooLarge { size: usize, max: usize },
    
    #[error("Sender banned: {0}")]
    SenderBanned(Address),
}

/// Transaction pool configuration
#[derive(Clone, Debug)]
pub struct TxPoolConfig {
    /// Maximum number of transactions in pool
    pub max_size: usize,
    /// Maximum transactions per sender
    pub max_per_sender: usize,
    /// Maximum transaction size in bytes
    pub max_tx_size: usize,
    /// Minimum gas price (in base units)
    pub min_gas_price: u64,
    /// Maximum gas limit per transaction
    pub max_gas_limit: u64,
    /// Transaction lifetime (seconds)
    pub tx_lifetime_secs: u64,
    /// Enable replacement by higher gas price
    pub allow_replacement: bool,
    /// Minimum gas price bump for replacement (percentage)
    pub replacement_bump_percent: u64,
    /// Priority slots reserved for high-priority txs
    pub priority_slots: usize,
}

impl Default for TxPoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10_000,
            max_per_sender: 100,
            max_tx_size: 128 * 1024, // 128KB
            min_gas_price: 1,
            max_gas_limit: 10_000_000,
            tx_lifetime_secs: 3600, // 1 hour
            allow_replacement: true,
            replacement_bump_percent: 10,
            priority_slots: 100,
        }
    }
}

/// Transaction with metadata
#[derive(Clone, Debug)]
pub struct PooledTransaction {
    /// The transaction
    pub tx: SignedTransaction,
    /// Transaction hash
    pub hash: HashValue,
    /// Sender address
    pub sender: Address,
    /// Nonce
    pub nonce: u64,
    /// Gas price
    pub gas_price: u64,
    /// Gas limit
    pub gas_limit: u64,
    /// Time added to pool (unix timestamp)
    pub added_at: u64,
    /// Whether this is a local transaction
    pub is_local: bool,
    /// Priority score
    pub priority: u64,
    /// Size in bytes
    pub size: usize,
}

impl PooledTransaction {
    /// Create from signed transaction
    pub fn new(tx: SignedTransaction, is_local: bool) -> Self {
        let hash = tx.hash();
        let sender = tx.sender();
        let nonce = tx.nonce();
        let gas_price = tx.gas_price();
        let gas_limit = tx.gas_limit();
        let size = tx.size();
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Calculate priority: local txs get bonus, then by gas price
        let priority = if is_local {
            gas_price + 1_000_000_000
        } else {
            gas_price
        };
        
        Self {
            tx,
            hash,
            sender,
            nonce,
            gas_price,
            gas_limit,
            added_at: now,
            is_local,
            priority,
            size,
        }
    }
}

/// Sender transaction queue
#[derive(Debug, Default)]
struct SenderQueue {
    /// Pending transactions sorted by nonce
    pending: BTreeMap<u64, PooledTransaction>,
    /// Queued transactions (nonce gap)
    queued: BTreeMap<u64, PooledTransaction>,
    /// Total gas used by pending txs
    pending_gas: u64,
    /// Next expected nonce
    next_nonce: u64,
}

impl SenderQueue {
    fn new(next_nonce: u64) -> Self {
        Self {
            pending: BTreeMap::new(),
            queued: BTreeMap::new(),
            pending_gas: 0,
            next_nonce,
        }
    }
    
    fn total_count(&self) -> usize {
        self.pending.len() + self.queued.len()
    }
    
    fn add(&mut self, tx: PooledTransaction, config: &TxPoolConfig) -> Result<Option<PooledTransaction>, TxPoolError> {
        let nonce = tx.nonce;
        
        // Check if replacing existing transaction
        if let Some(existing) = self.pending.get(&nonce) {
            if config.allow_replacement {
                let min_new_price = existing.gas_price * (100 + config.replacement_bump_percent) / 100;
                if tx.gas_price >= min_new_price {
                    let replaced = self.pending.insert(nonce, tx);
                    return Ok(replaced);
                } else {
                    return Err(TxPoolError::GasPriceTooLow {
                        minimum: min_new_price,
                        got: tx.gas_price,
                    });
                }
            } else {
                return Err(TxPoolError::AlreadyExists(tx.hash.to_hex()));
            }
        }
        
        // Check nonce validity
        if nonce < self.next_nonce {
            return Err(TxPoolError::NonceTooLow {
                expected: self.next_nonce,
                got: nonce,
            });
        }
        
        // Add to pending or queued
        if nonce == self.next_nonce {
            self.pending.insert(nonce, tx);
            self.next_nonce += 1;
            
            // Promote any queued transactions
            while let Some(queued_tx) = self.queued.remove(&self.next_nonce) {
                self.pending.insert(self.next_nonce, queued_tx);
                self.next_nonce += 1;
            }
        } else {
            // Nonce gap - queue it
            self.queued.insert(nonce, tx);
        }
        
        Ok(None)
    }
    
    fn remove(&mut self, nonce: u64) -> Option<PooledTransaction> {
        self.pending.remove(&nonce).or_else(|| self.queued.remove(&nonce))
    }
    
    fn get_pending(&self) -> Vec<&PooledTransaction> {
        self.pending.values().collect()
    }
    
    fn remove_below_nonce(&mut self, nonce: u64) -> Vec<PooledTransaction> {
        let mut removed = Vec::new();
        
        // Remove from pending
        let to_remove: Vec<_> = self.pending.range(..nonce).map(|(n, _)| *n).collect();
        for n in to_remove {
            if let Some(tx) = self.pending.remove(&n) {
                removed.push(tx);
            }
        }
        
        // Remove from queued
        let to_remove: Vec<_> = self.queued.range(..nonce).map(|(n, _)| *n).collect();
        for n in to_remove {
            if let Some(tx) = self.queued.remove(&n) {
                removed.push(tx);
            }
        }
        
        // Update next nonce
        if nonce > self.next_nonce {
            self.next_nonce = nonce;
        }
        
        removed
    }
}

/// Transaction pool statistics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TxPoolStats {
    /// Total pending transactions
    pub pending_count: usize,
    /// Total queued transactions
    pub queued_count: usize,
    /// Total transactions
    pub total_count: usize,
    /// Number of senders with pending txs
    pub sender_count: usize,
    /// Total gas of pending transactions
    pub pending_gas: u64,
    /// Pool utilization (0-100)
    pub utilization: u8,
    /// Transactions added this session
    pub added: u64,
    /// Transactions removed this session
    pub removed: u64,
    /// Transactions rejected this session
    pub rejected: u64,
    /// Transactions expired this session
    pub expired: u64,
}

/// Account state provider trait
pub trait AccountStateProvider: Send + Sync {
    fn get_nonce(&self, address: &Address) -> u64;
    fn get_balance(&self, address: &Address) -> Amount;
}

/// Simple in-memory account state
pub struct InMemoryAccountState {
    nonces: RwLock<HashMap<Address, u64>>,
    balances: RwLock<HashMap<Address, Amount>>,
}

impl InMemoryAccountState {
    pub fn new() -> Self {
        Self {
            nonces: RwLock::new(HashMap::new()),
            balances: RwLock::new(HashMap::new()),
        }
    }
    
    pub fn set_nonce(&self, address: Address, nonce: u64) {
        self.nonces.write().insert(address, nonce);
    }
    
    pub fn set_balance(&self, address: Address, balance: Amount) {
        self.balances.write().insert(address, balance);
    }
}

impl AccountStateProvider for InMemoryAccountState {
    fn get_nonce(&self, address: &Address) -> u64 {
        *self.nonces.read().get(address).unwrap_or(&0)
    }
    
    fn get_balance(&self, address: &Address) -> Amount {
        *self.balances.read().get(address).unwrap_or(&0)
    }
}

impl Default for InMemoryAccountState {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction pool
pub struct TransactionPool {
    /// Configuration
    config: TxPoolConfig,
    /// Sender queues
    senders: RwLock<HashMap<Address, SenderQueue>>,
    /// All transactions by hash
    by_hash: RwLock<HashMap<HashValue, Address>>,
    /// Priority queue for block production
    priority_queue: RwLock<BTreeMap<(u64, HashValue), Address>>,
    /// Banned senders
    banned: RwLock<HashSet<Address>>,
    /// Account state provider
    state: Arc<dyn AccountStateProvider>,
    /// Statistics
    stats: RwLock<TxPoolStats>,
}

impl TransactionPool {
    /// Create new transaction pool
    pub fn new(config: TxPoolConfig, state: Arc<dyn AccountStateProvider>) -> Self {
        Self {
            config,
            senders: RwLock::new(HashMap::new()),
            by_hash: RwLock::new(HashMap::new()),
            priority_queue: RwLock::new(BTreeMap::new()),
            banned: RwLock::new(HashSet::new()),
            state,
            stats: RwLock::new(TxPoolStats::default()),
        }
    }
    
    /// Add transaction to pool
    pub fn add(&self, tx: SignedTransaction, is_local: bool) -> Result<HashValue, TxPoolError> {
        let pooled = PooledTransaction::new(tx, is_local);
        
        // Validation
        self.validate(&pooled)?;
        
        let hash = pooled.hash;
        let sender = pooled.sender;
        let priority = pooled.priority;
        
        // Check if already exists
        if self.by_hash.read().contains_key(&hash) {
            self.stats.write().rejected += 1;
            return Err(TxPoolError::AlreadyExists(hash.to_hex()));
        }
        
        // Check pool capacity
        if self.len() >= self.config.max_size {
            // Try to evict low priority transaction
            if !self.try_evict(priority) {
                self.stats.write().rejected += 1;
                return Err(TxPoolError::PoolFull);
            }
        }
        
        // Get or create sender queue
        let next_nonce = self.state.get_nonce(&sender);
        let mut senders = self.senders.write();
        let queue = senders.entry(sender).or_insert_with(|| SenderQueue::new(next_nonce));
        
        // Check sender limit
        if queue.total_count() >= self.config.max_per_sender {
            self.stats.write().rejected += 1;
            return Err(TxPoolError::PoolFull);
        }
        
        // Add to queue
        let replaced = queue.add(pooled.clone(), &self.config)?;
        
        // Update indexes
        if let Some(old) = replaced {
            self.by_hash.write().remove(&old.hash);
            self.priority_queue.write().remove(&(old.priority, old.hash));
        }
        
        self.by_hash.write().insert(hash, sender);
        self.priority_queue.write().insert((priority, hash), sender);
        
        // Update stats
        let mut stats = self.stats.write();
        stats.added += 1;
        stats.total_count = self.len();
        stats.sender_count = senders.len();
        
        debug!(hash = %hash, sender = %sender, "Transaction added to pool");
        
        Ok(hash)
    }
    
    /// Validate transaction
    fn validate(&self, tx: &PooledTransaction) -> Result<(), TxPoolError> {
        // Check if sender is banned
        if self.banned.read().contains(&tx.sender) {
            return Err(TxPoolError::SenderBanned(tx.sender));
        }
        
        // Check size
        if tx.size > self.config.max_tx_size {
            return Err(TxPoolError::TooLarge {
                size: tx.size,
                max: self.config.max_tx_size,
            });
        }
        
        // Check gas price
        if tx.gas_price < self.config.min_gas_price {
            return Err(TxPoolError::GasPriceTooLow {
                minimum: self.config.min_gas_price,
                got: tx.gas_price,
            });
        }
        
        // Check gas limit
        if tx.gas_limit > self.config.max_gas_limit {
            return Err(TxPoolError::InvalidTransaction(
                format!("Gas limit too high: {} (max {})", tx.gas_limit, self.config.max_gas_limit)
            ));
        }
        
        // Check balance
        let balance = self.state.get_balance(&tx.sender);
        let required = tx.tx.value() + tx.gas_price * tx.gas_limit;
        if balance < required {
            return Err(TxPoolError::InsufficientBalance {
                required,
                available: balance,
            });
        }
        
        Ok(())
    }
    
    /// Try to evict a low priority transaction
    fn try_evict(&self, new_priority: u64) -> bool {
        let mut priority_queue = self.priority_queue.write();
        
        // Find lowest priority transaction
        if let Some(((priority, hash), sender)) = priority_queue.iter().next() {
            if *priority < new_priority {
                let priority = *priority;
                let hash = *hash;
                let sender = *sender;
                drop(priority_queue);
                
                // Remove the transaction
                self.remove(&hash);
                self.stats.write().removed += 1;
                
                debug!(hash = %hash, "Evicted low priority transaction");
                return true;
            }
        }
        
        false
    }
    
    /// Remove transaction by hash
    pub fn remove(&self, hash: &HashValue) -> Option<PooledTransaction> {
        let sender = self.by_hash.write().remove(hash)?;
        
        let mut senders = self.senders.write();
        if let Some(queue) = senders.get_mut(&sender) {
            // Find and remove the transaction
            for (nonce, tx) in queue.pending.iter() {
                if &tx.hash == hash {
                    let nonce = *nonce;
                    let tx = queue.pending.remove(&nonce);
                    self.priority_queue.write().remove(&(tx.as_ref().unwrap().priority, *hash));
                    self.stats.write().removed += 1;
                    return tx;
                }
            }
            for (nonce, tx) in queue.queued.iter() {
                if &tx.hash == hash {
                    let nonce = *nonce;
                    let tx = queue.queued.remove(&nonce);
                    self.priority_queue.write().remove(&(tx.as_ref().unwrap().priority, *hash));
                    self.stats.write().removed += 1;
                    return tx;
                }
            }
        }
        
        None
    }
    
    /// Get transaction by hash
    pub fn get(&self, hash: &HashValue) -> Option<PooledTransaction> {
        let sender = self.by_hash.read().get(hash).cloned()?;
        let senders = self.senders.read();
        let queue = senders.get(&sender)?;
        
        for tx in queue.pending.values() {
            if &tx.hash == hash {
                return Some(tx.clone());
            }
        }
        for tx in queue.queued.values() {
            if &tx.hash == hash {
                return Some(tx.clone());
            }
        }
        
        None
    }
    
    /// Check if transaction exists
    pub fn contains(&self, hash: &HashValue) -> bool {
        self.by_hash.read().contains_key(hash)
    }
    
    /// Get pending transactions for block production
    pub fn pending(&self, max_count: usize, max_gas: u64) -> Vec<PooledTransaction> {
        let mut result = Vec::new();
        let mut total_gas = 0u64;
        
        // Iterate by priority (highest first)
        let priority_queue = self.priority_queue.read();
        let senders = self.senders.read();
        
        for ((priority, hash), sender) in priority_queue.iter().rev() {
            if result.len() >= max_count {
                break;
            }
            
            if let Some(queue) = senders.get(sender) {
                for tx in queue.pending.values() {
                    if &tx.hash == hash {
                        if total_gas + tx.gas_limit <= max_gas {
                            result.push(tx.clone());
                            total_gas += tx.gas_limit;
                        }
                        break;
                    }
                }
            }
        }
        
        result
    }
    
    /// Get all pending transactions for a sender
    pub fn pending_for_sender(&self, sender: &Address) -> Vec<PooledTransaction> {
        self.senders.read()
            .get(sender)
            .map(|q| q.pending.values().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Update account state (called after block execution)
    pub fn on_block_executed(&self, updates: &[(Address, u64)]) {
        let mut senders = self.senders.write();
        let mut by_hash = self.by_hash.write();
        let mut priority_queue = self.priority_queue.write();
        
        for (address, new_nonce) in updates {
            if let Some(queue) = senders.get_mut(address) {
                // Remove executed transactions
                let removed = queue.remove_below_nonce(*new_nonce);
                for tx in removed {
                    by_hash.remove(&tx.hash);
                    priority_queue.remove(&(tx.priority, tx.hash));
                }
                
                // Remove empty sender queues
                if queue.total_count() == 0 {
                    senders.remove(address);
                }
            }
        }
        
        self.stats.write().total_count = self.by_hash.read().len();
    }
    
    /// Remove expired transactions
    pub fn remove_expired(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let expiry = now - self.config.tx_lifetime_secs;
        
        let to_remove: Vec<HashValue> = {
            let senders = self.senders.read();
            senders.values()
                .flat_map(|q| q.pending.values().chain(q.queued.values()))
                .filter(|tx| tx.added_at < expiry)
                .map(|tx| tx.hash)
                .collect()
        };
        
        for hash in to_remove {
            self.remove(&hash);
            self.stats.write().expired += 1;
        }
    }
    
    /// Ban a sender
    pub fn ban_sender(&self, sender: Address) {
        self.banned.write().insert(sender);
        
        // Remove all their transactions
        if let Some(queue) = self.senders.write().remove(&sender) {
            let mut by_hash = self.by_hash.write();
            let mut priority_queue = self.priority_queue.write();
            
            for tx in queue.pending.values().chain(queue.queued.values()) {
                by_hash.remove(&tx.hash);
                priority_queue.remove(&(tx.priority, tx.hash));
            }
        }
        
        info!(sender = %sender, "Sender banned from pool");
    }
    
    /// Unban a sender
    pub fn unban_sender(&self, sender: &Address) {
        self.banned.write().remove(sender);
    }
    
    /// Get pool length
    pub fn len(&self) -> usize {
        self.by_hash.read().len()
    }
    
    /// Check if pool is empty
    pub fn is_empty(&self) -> bool {
        self.by_hash.read().is_empty()
    }
    
    /// Get pool statistics
    pub fn stats(&self) -> TxPoolStats {
        let senders = self.senders.read();
        let mut pending_count = 0;
        let mut queued_count = 0;
        let mut pending_gas = 0;
        
        for queue in senders.values() {
            pending_count += queue.pending.len();
            queued_count += queue.queued.len();
            pending_gas += queue.pending.values().map(|tx| tx.gas_limit).sum::<u64>();
        }
        
        let mut stats = self.stats.read().clone();
        stats.pending_count = pending_count;
        stats.queued_count = queued_count;
        stats.total_count = pending_count + queued_count;
        stats.sender_count = senders.len();
        stats.pending_gas = pending_gas;
        stats.utilization = ((stats.total_count * 100) / self.config.max_size.max(1)) as u8;
        
        stats
    }
    
    /// Clear the pool
    pub fn clear(&self) {
        self.senders.write().clear();
        self.by_hash.write().clear();
        self.priority_queue.write().clear();
        *self.stats.write() = TxPoolStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_tx(sender: Address, nonce: u64, gas_price: u64) -> SignedTransaction {
        // Create a mock signed transaction
        SignedTransaction::mock(sender, nonce, gas_price)
    }
    
    #[test]
    fn test_pool_add_remove() {
        let state = Arc::new(InMemoryAccountState::new());
        let pool = TransactionPool::new(TxPoolConfig::default(), state.clone());
        
        let sender = Address::from_public_key(b"test");
        state.set_balance(sender, 1_000_000_000_000);
        
        let tx = create_test_tx(sender, 0, 100);
        let hash = pool.add(tx, true).unwrap();
        
        assert!(pool.contains(&hash));
        assert_eq!(pool.len(), 1);
        
        pool.remove(&hash);
        assert!(!pool.contains(&hash));
        assert_eq!(pool.len(), 0);
    }
    
    #[test]
    fn test_pool_nonce_ordering() {
        let state = Arc::new(InMemoryAccountState::new());
        let pool = TransactionPool::new(TxPoolConfig::default(), state.clone());
        
        let sender = Address::from_public_key(b"test");
        state.set_balance(sender, 1_000_000_000_000);
        
        // Add transactions out of order
        let tx2 = create_test_tx(sender, 2, 100);
        let tx0 = create_test_tx(sender, 0, 100);
        let tx1 = create_test_tx(sender, 1, 100);
        
        pool.add(tx2, true).unwrap(); // Should be queued (nonce gap)
        pool.add(tx0, true).unwrap(); // Should be pending
        pool.add(tx1, true).unwrap(); // Should promote tx2
        
        let pending = pool.pending_for_sender(&sender);
        assert_eq!(pending.len(), 3);
        assert_eq!(pending[0].nonce, 0);
        assert_eq!(pending[1].nonce, 1);
        assert_eq!(pending[2].nonce, 2);
    }
    
    #[test]
    fn test_pool_replacement() {
        let state = Arc::new(InMemoryAccountState::new());
        let config = TxPoolConfig {
            allow_replacement: true,
            replacement_bump_percent: 10,
            ..Default::default()
        };
        let pool = TransactionPool::new(config, state.clone());
        
        let sender = Address::from_public_key(b"test");
        state.set_balance(sender, 1_000_000_000_000);
        
        let tx1 = create_test_tx(sender, 0, 100);
        let tx2 = create_test_tx(sender, 0, 111); // 11% higher
        
        pool.add(tx1, true).unwrap();
        let hash2 = pool.add(tx2, true).unwrap();
        
        assert_eq!(pool.len(), 1);
        assert!(pool.contains(&hash2));
    }
    
    #[test]
    fn test_pool_ban_sender() {
        let state = Arc::new(InMemoryAccountState::new());
        let pool = TransactionPool::new(TxPoolConfig::default(), state.clone());
        
        let sender = Address::from_public_key(b"test");
        state.set_balance(sender, 1_000_000_000_000);
        
        let tx = create_test_tx(sender, 0, 100);
        pool.add(tx, true).unwrap();
        
        assert_eq!(pool.len(), 1);
        
        pool.ban_sender(sender);
        
        assert_eq!(pool.len(), 0);
        
        // New transactions should be rejected
        let tx2 = create_test_tx(sender, 1, 100);
        assert!(pool.add(tx2, true).is_err());
    }
}
