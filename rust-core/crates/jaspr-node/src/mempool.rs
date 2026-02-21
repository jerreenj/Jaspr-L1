//! Transaction mempool

use jaspr_types::{SignedTransaction, HashValue, Address, Nonce, Gas};
use parking_lot::RwLock;
use std::collections::{HashMap, BTreeMap};
use std::time::{Instant, Duration};

/// Mempool configuration
#[derive(Clone, Debug)]
pub struct MempoolConfig {
    /// Maximum transactions in mempool
    pub max_size: usize,
    /// Maximum transactions per account
    pub max_per_account: usize,
    /// Transaction expiry time
    pub expiry_time: Duration,
    /// Minimum gas price
    pub min_gas_price: u64,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10000,
            max_per_account: 100,
            expiry_time: Duration::from_secs(3600), // 1 hour
            min_gas_price: 1,
        }
    }
}

/// Transaction entry in mempool
#[derive(Clone, Debug)]
struct TxEntry {
    tx: SignedTransaction,
    added_at: Instant,
    gas_price: u64,
}

/// Transaction mempool
pub struct Mempool {
    config: MempoolConfig,
    /// Transactions by hash
    txs: RwLock<HashMap<HashValue, TxEntry>>,
    /// Transactions by sender and nonce
    by_sender: RwLock<HashMap<Address, BTreeMap<Nonce, HashValue>>>,
    /// Transactions ordered by gas price (for block production)
    by_gas_price: RwLock<BTreeMap<(u64, HashValue), SignedTransaction>>,
}

impl Mempool {
    /// Create new mempool
    pub fn new(config: MempoolConfig) -> Self {
        Self {
            config,
            txs: RwLock::new(HashMap::new()),
            by_sender: RwLock::new(HashMap::new()),
            by_gas_price: RwLock::new(BTreeMap::new()),
        }
    }
    
    /// Add transaction to mempool
    pub fn add_transaction(&self, tx: SignedTransaction) -> Result<(), String> {
        let hash = tx.hash();
        let sender = tx.sender();
        let nonce = tx.nonce();
        let gas_price = tx.transaction.gas_price;
        
        // Check gas price
        if gas_price < self.config.min_gas_price {
            return Err("Gas price too low".to_string());
        }
        
        // Check if already exists
        if self.txs.read().contains_key(&hash) {
            return Err("Transaction already in mempool".to_string());
        }
        
        // Check mempool size
        if self.txs.read().len() >= self.config.max_size {
            // Try to evict lowest gas price tx
            self.evict_lowest_gas_price()?;
        }
        
        // Check per-account limit
        {
            let by_sender = self.by_sender.read();
            if let Some(sender_txs) = by_sender.get(&sender) {
                if sender_txs.len() >= self.config.max_per_account {
                    return Err("Too many transactions from sender".to_string());
                }
            }
        }
        
        // Add to all indexes
        let entry = TxEntry {
            tx: tx.clone(),
            added_at: Instant::now(),
            gas_price,
        };
        
        self.txs.write().insert(hash, entry);
        self.by_sender.write()
            .entry(sender)
            .or_default()
            .insert(nonce, hash);
        self.by_gas_price.write()
            .insert((gas_price, hash), tx);
        
        Ok(())
    }
    
    /// Remove transaction from mempool
    pub fn remove_transaction(&self, hash: &HashValue) -> Option<SignedTransaction> {
        let entry = self.txs.write().remove(hash)?;
        
        // Remove from sender index
        let sender = entry.tx.sender();
        let nonce = entry.tx.nonce();
        
        if let Some(sender_txs) = self.by_sender.write().get_mut(&sender) {
            sender_txs.remove(&nonce);
        }
        
        // Remove from gas price index
        self.by_gas_price.write().remove(&(entry.gas_price, *hash));
        
        Some(entry.tx)
    }
    
    /// Get transaction by hash
    pub fn get_transaction(&self, hash: &HashValue) -> Option<SignedTransaction> {
        self.txs.read().get(hash).map(|e| e.tx.clone())
    }
    
    /// Check if transaction exists
    pub fn contains(&self, hash: &HashValue) -> bool {
        self.txs.read().contains_key(hash)
    }
    
    /// Get transactions for sender
    pub fn get_sender_transactions(&self, sender: &Address) -> Vec<SignedTransaction> {
        let by_sender = self.by_sender.read();
        let txs = self.txs.read();
        
        by_sender.get(sender)
            .map(|nonces| {
                nonces.values()
                    .filter_map(|hash| txs.get(hash).map(|e| e.tx.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get next nonce for sender (current nonce + pending txs)
    pub fn get_pending_nonce(&self, sender: &Address, current_nonce: Nonce) -> Nonce {
        let by_sender = self.by_sender.read();
        
        match by_sender.get(sender) {
            Some(sender_txs) => {
                let max_nonce = sender_txs.keys().max().copied().unwrap_or(0);
                max_nonce.max(current_nonce) + 1
            }
            None => current_nonce,
        }
    }
    
    /// Get top transactions by gas price for block production
    pub fn get_top_transactions(&self, limit: usize) -> Vec<SignedTransaction> {
        self.by_gas_price.read()
            .values()
            .rev() // Highest gas price first
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Get all pending transactions
    pub fn pending_transactions(&self) -> Vec<SignedTransaction> {
        self.txs.read()
            .values()
            .map(|e| e.tx.clone())
            .collect()
    }
    
    /// Remove expired transactions
    pub fn remove_expired(&self) {
        let expiry = self.config.expiry_time;
        let mut to_remove = Vec::new();
        
        {
            let txs = self.txs.read();
            for (hash, entry) in txs.iter() {
                if entry.added_at.elapsed() > expiry {
                    to_remove.push(*hash);
                }
            }
        }
        
        for hash in to_remove {
            self.remove_transaction(&hash);
        }
    }
    
    /// Remove transactions included in block
    pub fn remove_committed(&self, txs: &[SignedTransaction]) {
        for tx in txs {
            self.remove_transaction(&tx.hash());
        }
    }
    
    /// Evict lowest gas price transaction
    fn evict_lowest_gas_price(&self) -> Result<(), String> {
        let lowest = {
            let by_gas = self.by_gas_price.read();
            by_gas.keys().next().map(|(_, hash)| *hash)
        };
        
        match lowest {
            Some(hash) => {
                self.remove_transaction(&hash);
                Ok(())
            }
            None => Err("Mempool is full".to_string()),
        }
    }
    
    /// Get mempool size
    pub fn size(&self) -> usize {
        self.txs.read().len()
    }
    
    /// Clear mempool
    pub fn clear(&self) {
        self.txs.write().clear();
        self.by_sender.write().clear();
        self.by_gas_price.write().clear();
    }
    
    /// Get mempool stats
    pub fn stats(&self) -> MempoolStats {
        let txs = self.txs.read();
        
        let total_gas: Gas = txs.values()
            .map(|e| e.tx.max_gas())
            .sum();
        
        MempoolStats {
            pending_count: txs.len(),
            total_gas,
            unique_senders: self.by_sender.read().len(),
        }
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new(MempoolConfig::default())
    }
}

/// Mempool statistics
#[derive(Clone, Debug)]
pub struct MempoolStats {
    pub pending_count: usize,
    pub total_gas: Gas,
    pub unique_senders: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use jaspr_types::{Transaction, TransactionPayload, Address};
    
    fn create_test_tx(nonce: Nonce, gas_price: u64) -> SignedTransaction {
        let tx = Transaction::new(
            Address::from_public_key(b"sender"),
            TransactionPayload::Transfer {
                recipient: Address::zero(),
                amount: 1000,
            },
            nonce,
            100000,
            gas_price,
            1,
        );
        
        SignedTransaction::new(tx, vec![0u8; 64], b"sender".to_vec())
    }
    
    #[test]
    fn test_add_remove() {
        let mempool = Mempool::default();
        
        let tx = create_test_tx(0, 100);
        let hash = tx.hash();
        
        mempool.add_transaction(tx).unwrap();
        assert!(mempool.contains(&hash));
        assert_eq!(mempool.size(), 1);
        
        mempool.remove_transaction(&hash);
        assert!(!mempool.contains(&hash));
        assert_eq!(mempool.size(), 0);
    }
    
    #[test]
    fn test_gas_price_ordering() {
        let mempool = Mempool::default();
        
        // Add transactions with different gas prices
        mempool.add_transaction(create_test_tx(0, 100)).unwrap();
        mempool.add_transaction(create_test_tx(1, 200)).unwrap();
        mempool.add_transaction(create_test_tx(2, 50)).unwrap();
        
        let top = mempool.get_top_transactions(3);
        
        // Should be ordered by gas price (highest first)
        assert_eq!(top[0].transaction.gas_price, 200);
        assert_eq!(top[1].transaction.gas_price, 100);
        assert_eq!(top[2].transaction.gas_price, 50);
    }
    
    #[test]
    fn test_reject_low_gas_price() {
        let config = MempoolConfig {
            min_gas_price: 100,
            ..Default::default()
        };
        let mempool = Mempool::new(config);
        
        let tx = create_test_tx(0, 50);
        assert!(mempool.add_transaction(tx).is_err());
    }
}
