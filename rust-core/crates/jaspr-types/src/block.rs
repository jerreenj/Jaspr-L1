//! Block types for JasprChain

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use crate::{Address, HashValue, Hasher, BlockHeight, Timestamp, MerkleTree};

/// Block header containing metadata
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Block height (0 = genesis)
    pub height: BlockHeight,
    
    /// Hash of previous block
    pub previous_hash: HashValue,
    
    /// Timestamp when block was created (ms since epoch)
    pub timestamp: Timestamp,
    
    /// Address of block proposer (validator)
    pub proposer: Address,
    
    /// Root hash of state after executing all transactions
    pub state_root: HashValue,
    
    /// Merkle root of all transactions in block
    pub transactions_root: HashValue,
    
    /// Merkle root of all receipts
    pub receipts_root: HashValue,
    
    /// Gas used by all transactions
    pub gas_used: u64,
    
    /// Gas limit for this block
    pub gas_limit: u64,
    
    /// Chain ID for replay protection
    pub chain_id: u64,
    
    /// BLS aggregate signature from committee (hex encoded)
    pub committee_signature: String,
    
    /// Number of validators who attested
    pub attestation_count: u32,
}

impl BlockHeader {
    /// Create genesis block header
    pub fn genesis(chain_id: u64) -> Self {
        Self {
            height: 0,
            previous_hash: HashValue::zero(),
            timestamp: 0,
            proposer: Address::zero(),
            state_root: HashValue::zero(),
            transactions_root: HashValue::zero(),
            receipts_root: HashValue::zero(),
            gas_used: 0,
            gas_limit: 100_000_000,
            chain_id,
            committee_signature: String::new(),
            attestation_count: 0,
        }
    }

    /// Compute block hash
    pub fn compute_hash(&self) -> HashValue {
        let serialized = borsh::to_vec(self).expect("Serialization should not fail");
        HashValue::sha256(&serialized)
    }
}

impl Hasher for BlockHeader {
    fn hash(&self) -> HashValue {
        self.compute_hash()
    }
}

/// Block body containing transactions
#[derive(Clone, Debug, Default, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct BlockBody {
    /// List of signed transactions
    pub transactions: Vec<crate::SignedTransaction>,
}

impl BlockBody {
    /// Create empty block body
    pub fn new() -> Self {
        Self {
            transactions: Vec::new(),
        }
    }

    /// Compute transactions merkle root
    pub fn compute_transactions_root(&self) -> HashValue {
        let tx_hashes: Vec<HashValue> = self.transactions
            .iter()
            .map(|tx| tx.hash())
            .collect();
        MerkleTree::compute_root(&tx_hashes)
    }

    /// Number of transactions
    pub fn tx_count(&self) -> usize {
        self.transactions.len()
    }
}

/// Complete block with header and body
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    
    /// Block body with transactions
    pub body: BlockBody,
    
    /// Is block finalized (has enough attestations)
    pub finalized: bool,
    
    /// Time taken to finalize (ms)
    pub finality_time_ms: Option<u64>,
}

impl Block {
    /// Create new block
    pub fn new(header: BlockHeader, body: BlockBody) -> Self {
        Self {
            header,
            body,
            finalized: false,
            finality_time_ms: None,
        }
    }

    /// Create genesis block
    pub fn genesis(chain_id: u64) -> Self {
        Self {
            header: BlockHeader::genesis(chain_id),
            body: BlockBody::new(),
            finalized: true,
            finality_time_ms: Some(0),
        }
    }

    /// Get block hash
    pub fn hash(&self) -> HashValue {
        self.header.compute_hash()
    }

    /// Get block height
    pub fn height(&self) -> BlockHeight {
        self.header.height
    }

    /// Mark as finalized
    pub fn finalize(&mut self, time_ms: u64) {
        self.finalized = true;
        self.finality_time_ms = Some(time_ms);
    }

    /// Check if block is valid (basic structural validation)
    pub fn validate(&self) -> Result<(), String> {
        // Verify transactions root matches
        let computed_root = self.body.compute_transactions_root();
        if computed_root != self.header.transactions_root {
            return Err("Transactions root mismatch".to_string());
        }

        // Genesis block special case
        if self.header.height == 0 {
            if !self.header.previous_hash.is_zero() {
                return Err("Genesis block should have zero previous hash".to_string());
            }
        }

        Ok(())
    }
}

impl Hasher for Block {
    fn hash(&self) -> HashValue {
        self.header.compute_hash()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_block() {
        let genesis = Block::genesis(1);
        assert_eq!(genesis.height(), 0);
        assert!(genesis.finalized);
        assert!(genesis.validate().is_ok());
    }

    #[test]
    fn test_block_hash_deterministic() {
        let block = Block::genesis(1);
        let hash1 = block.hash();
        let hash2 = block.hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_empty_body_root() {
        let body = BlockBody::new();
        let root = body.compute_transactions_root();
        assert!(root.is_zero());
    }
}
