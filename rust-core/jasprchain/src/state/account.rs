//! Account state

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
    pub code_hash: Option<String>,
    pub storage_root: Option<String>,
    pub is_contract: bool,
}

impl Account {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
            balance: 0,
            nonce: 0,
            code_hash: None,
            storage_root: None,
            is_contract: false,
        }
    }
}
