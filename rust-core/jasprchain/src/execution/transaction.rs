//! Transaction types

use serde::{Deserialize, Serialize};
use crate::crypto::sha256_hex;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    Transfer,
    Stake,
    Unstake,
    ContractCall,
    ContractDeploy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub tx_type: TransactionType,
    pub sender: String,
    pub recipient: Option<String>,
    pub amount: u64,
    pub contract_address: Option<String>,
    pub function_name: Option<String>,
    pub args: serde_json::Value,
    pub gas_limit: u64,
    pub gas_price: u64,
    pub nonce: u64,
    pub timestamp: u64,
    pub chain_id: u64,
}

impl Transaction {
    pub fn hash(&self) -> String {
        let data = serde_json::to_string(self).unwrap_or_default();
        sha256_hex(data.as_bytes())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
}

impl SignedTransaction {
    pub fn hash(&self) -> String {
        self.transaction.hash()
    }
}
