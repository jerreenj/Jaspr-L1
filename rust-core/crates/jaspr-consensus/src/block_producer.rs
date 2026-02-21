//! Block production

use jaspr_types::{Block, BlockHeader, BlockBody, SignedTransaction, HashValue, Address, BlockHeight, Timestamp};
use jaspr_state::StateTree;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;

/// Block production configuration
#[derive(Clone, Debug)]
pub struct BlockProducerConfig {
    /// Maximum transactions per block
    pub max_txs_per_block: usize,
    /// Block gas limit
    pub gas_limit: u64,
    /// Chain ID
    pub chain_id: u64,
}

impl Default for BlockProducerConfig {
    fn default() -> Self {
        Self {
            max_txs_per_block: 10000,
            gas_limit: 100_000_000,
            chain_id: 1,
        }
    }
}

/// Block producer for creating new blocks
pub struct BlockProducer {
    config: BlockProducerConfig,
    /// Pending transactions to include
    pending_txs: RwLock<Vec<SignedTransaction>>,
}

impl BlockProducer {
    /// Create new block producer
    pub fn new(config: BlockProducerConfig) -> Self {
        Self {
            config,
            pending_txs: RwLock::new(Vec::new()),
        }
    }
    
    /// Add transaction to pending pool
    pub fn add_transaction(&self, tx: SignedTransaction) {
        let mut pending = self.pending_txs.write();
        if pending.len() < self.config.max_txs_per_block * 2 {
            pending.push(tx);
        }
    }
    
    /// Get pending transaction count
    pub fn pending_count(&self) -> usize {
        self.pending_txs.read().len()
    }
    
    /// Clear pending transactions
    pub fn clear_pending(&self) {
        self.pending_txs.write().clear();
    }
    
    /// Produce a new block
    pub fn produce_block(
        &self,
        height: BlockHeight,
        previous_hash: HashValue,
        proposer: Address,
        state_root: HashValue,
    ) -> Block {
        let timestamp = current_timestamp();
        
        // Get transactions to include
        let transactions: Vec<SignedTransaction> = {
            let mut pending = self.pending_txs.write();
            let to_include: Vec<_> = pending.drain(..pending.len().min(self.config.max_txs_per_block)).collect();
            to_include
        };
        
        // Create block body
        let body = BlockBody { transactions };
        
        // Calculate transactions root
        let transactions_root = body.compute_transactions_root();
        
        // Calculate gas used (simplified - in production, sum actual gas from execution)
        let gas_used = body.transactions.len() as u64 * 21000; // Base tx cost
        
        // Create header
        let header = BlockHeader {
            height,
            previous_hash,
            timestamp,
            proposer,
            state_root,
            transactions_root,
            receipts_root: HashValue::zero(), // Filled after execution
            gas_used,
            gas_limit: self.config.gas_limit,
            chain_id: self.config.chain_id,
            committee_signature: String::new(),
            attestation_count: 0,
        };
        
        Block::new(header, body)
    }
    
    /// Create genesis block
    pub fn create_genesis(&self) -> Block {
        Block::genesis(self.config.chain_id)
    }
    
    /// Update block after execution (with receipts root and final state root)
    pub fn finalize_block(
        &self,
        mut block: Block,
        receipts_root: HashValue,
        state_root: HashValue,
        gas_used: u64,
    ) -> Block {
        block.header.receipts_root = receipts_root;
        block.header.state_root = state_root;
        block.header.gas_used = gas_used;
        block
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as Timestamp
}

#[cfg(test)]
mod tests {
    use super::*;
    use jaspr_types::{Transaction, TransactionPayload};
    
    #[test]
    fn test_produce_empty_block() {
        let producer = BlockProducer::new(BlockProducerConfig::default());
        
        let block = producer.produce_block(
            1,
            HashValue::zero(),
            Address::zero(),
            HashValue::zero(),
        );
        
        assert_eq!(block.height(), 1);
        assert_eq!(block.body.tx_count(), 0);
    }
    
    #[test]
    fn test_produce_block_with_txs() {
        let producer = BlockProducer::new(BlockProducerConfig::default());
        
        // Add some transactions
        for i in 0..5 {
            let tx = Transaction::new(
                Address::from_public_key(format!("sender{}", i).as_bytes()),
                TransactionPayload::Transfer {
                    recipient: Address::zero(),
                    amount: 1000,
                },
                0,
                100000,
                100,
                1,
            );
            let signed = SignedTransaction::new(tx, vec![0u8; 64], vec![0u8; 32]);
            producer.add_transaction(signed);
        }
        
        assert_eq!(producer.pending_count(), 5);
        
        let block = producer.produce_block(
            1,
            HashValue::zero(),
            Address::zero(),
            HashValue::zero(),
        );
        
        assert_eq!(block.body.tx_count(), 5);
        assert_eq!(producer.pending_count(), 0); // Cleared after production
    }
    
    #[test]
    fn test_genesis() {
        let producer = BlockProducer::new(BlockProducerConfig::default());
        let genesis = producer.create_genesis();
        
        assert_eq!(genesis.height(), 0);
        assert!(genesis.finalized);
    }
}
