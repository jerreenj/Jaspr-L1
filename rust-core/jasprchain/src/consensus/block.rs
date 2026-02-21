//! Block structures for JasprChain

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

/// Block header containing consensus-critical data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockHeader {
    pub height: u64,
    pub previous_hash: String,
    pub timestamp: u64,
    pub proposer: String,
    pub state_root: String,
    pub transactions_root: String,
    pub receipts_root: String,
    pub committee_signatures: Vec<u8>,
    pub attestation_count: u32,
}

impl BlockHeader {
    pub fn hash(&self) -> String {
        let data = serde_json::to_string(&self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Complete block with header and transactions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<serde_json::Value>,
    pub receipts: Vec<serde_json::Value>,
    pub finalized: bool,
    pub finality_time_ms: Option<u64>,
}

impl Block {
    /// Create genesis block
    pub fn genesis() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let header = BlockHeader {
            height: 0,
            previous_hash: "0".repeat(64),
            timestamp: now,
            proposer: "jaspr1genesis".to_string(),
            state_root: Self::hash_bytes(b"genesis_state"),
            transactions_root: Self::hash_bytes(b""),
            receipts_root: Self::hash_bytes(b""),
            committee_signatures: Vec::new(),
            attestation_count: 0,
        };
        
        Self {
            header,
            transactions: Vec::new(),
            receipts: Vec::new(),
            finalized: true,
            finality_time_ms: Some(0),
        }
    }
    
    pub fn hash(&self) -> String {
        self.header.hash()
    }
    
    fn hash_bytes(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }
}
