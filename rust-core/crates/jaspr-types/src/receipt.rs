//! Transaction receipt types

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use crate::{HashValue, Gas, BlockHeight};

/// Execution status
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Transaction executed successfully
    Success,
    /// Transaction failed with error
    Failed(String),
    /// Out of gas
    OutOfGas,
    /// Move abort with code
    MoveAbort { location: String, code: u64 },
}

impl ExecutionStatus {
    /// Check if successful
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }
}

/// Event emitted during execution
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Event {
    /// Event type (e.g., "0x1::Coin::TransferEvent")
    pub type_tag: String,
    
    /// Event data (BCS encoded)
    pub data: Vec<u8>,
    
    /// Sequence number within transaction
    pub sequence_number: u64,
}

/// State change made during execution
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum StateChange {
    /// Account balance change
    BalanceChange {
        address: crate::Address,
        old_balance: u128,
        new_balance: u128,
    },
    
    /// Resource written
    ResourceWrite {
        address: crate::Address,
        type_tag: String,
        data: Vec<u8>,
    },
    
    /// Resource deleted
    ResourceDelete {
        address: crate::Address,
        type_tag: String,
    },
    
    /// Module published
    ModulePublish {
        address: crate::Address,
        name: String,
    },
}

/// Transaction receipt
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct TransactionReceipt {
    /// Transaction hash
    pub tx_hash: HashValue,
    
    /// Block height where tx was included
    pub block_height: BlockHeight,
    
    /// Index in block
    pub tx_index: u32,
    
    /// Execution status
    pub status: ExecutionStatus,
    
    /// Gas used
    pub gas_used: Gas,
    
    /// Events emitted
    pub events: Vec<Event>,
    
    /// State changes (for indexing)
    pub state_changes: Vec<StateChange>,
    
    /// State root after this transaction
    pub post_state_root: HashValue,
    
    /// Return values (for script calls)
    pub return_values: Vec<Vec<u8>>,
}

impl TransactionReceipt {
    /// Create success receipt
    pub fn success(
        tx_hash: HashValue,
        block_height: BlockHeight,
        tx_index: u32,
        gas_used: Gas,
    ) -> Self {
        Self {
            tx_hash,
            block_height,
            tx_index,
            status: ExecutionStatus::Success,
            gas_used,
            events: Vec::new(),
            state_changes: Vec::new(),
            post_state_root: HashValue::zero(),
            return_values: Vec::new(),
        }
    }

    /// Create failure receipt
    pub fn failed(
        tx_hash: HashValue,
        block_height: BlockHeight,
        tx_index: u32,
        gas_used: Gas,
        error: String,
    ) -> Self {
        Self {
            tx_hash,
            block_height,
            tx_index,
            status: ExecutionStatus::Failed(error),
            gas_used,
            events: Vec::new(),
            state_changes: Vec::new(),
            post_state_root: HashValue::zero(),
            return_values: Vec::new(),
        }
    }

    /// Check if successful
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Add event
    pub fn add_event(&mut self, type_tag: String, data: Vec<u8>) {
        let seq = self.events.len() as u64;
        self.events.push(Event {
            type_tag,
            data,
            sequence_number: seq,
        });
    }

    /// Add state change
    pub fn add_state_change(&mut self, change: StateChange) {
        self.state_changes.push(change);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receipt_success() {
        let receipt = TransactionReceipt::success(
            HashValue::sha256(b"tx"),
            1,
            0,
            50000,
        );
        
        assert!(receipt.is_success());
        assert_eq!(receipt.gas_used, 50000);
    }

    #[test]
    fn test_receipt_failed() {
        let receipt = TransactionReceipt::failed(
            HashValue::sha256(b"tx"),
            1,
            0,
            100000,
            "Insufficient balance".to_string(),
        );
        
        assert!(!receipt.is_success());
    }
}
