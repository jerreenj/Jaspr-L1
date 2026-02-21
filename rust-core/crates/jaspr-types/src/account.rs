//! Account types for JasprChain

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use crate::{Address, HashValue, Amount, Nonce};
use std::collections::HashMap;

/// Token balance
pub type Balance = Amount;

/// Account state
#[derive(Clone, Debug, Default, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct AccountState {
    /// Account balance in base units
    pub balance: Balance,
    
    /// Transaction nonce (sequence number)
    pub nonce: Nonce,
    
    /// Code hash if this is a contract account (zero for EOA)
    pub code_hash: HashValue,
    
    /// Storage root hash
    pub storage_root: HashValue,
    
    /// Is account frozen
    pub frozen: bool,
}

impl AccountState {
    /// Create new account with initial balance
    pub fn new(balance: Balance) -> Self {
        Self {
            balance,
            nonce: 0,
            code_hash: HashValue::zero(),
            storage_root: HashValue::zero(),
            frozen: false,
        }
    }

    /// Create empty account
    pub fn empty() -> Self {
        Self::new(0)
    }

    /// Check if account is empty (can be pruned)
    pub fn is_empty(&self) -> bool {
        self.balance == 0 && self.nonce == 0 && self.code_hash.is_zero()
    }

    /// Check if this is a contract account
    pub fn is_contract(&self) -> bool {
        !self.code_hash.is_zero()
    }

    /// Increment nonce and return new value
    pub fn increment_nonce(&mut self) -> Nonce {
        self.nonce += 1;
        self.nonce
    }
}

/// Full account with resources
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Account {
    /// Account address
    pub address: Address,
    
    /// Account state
    pub state: AccountState,
    
    /// Move resources stored in this account (type_tag -> data)
    pub resources: HashMap<String, Vec<u8>>,
    
    /// Move modules published by this account (module_name -> bytecode)
    pub modules: HashMap<String, Vec<u8>>,
}

impl Account {
    /// Create new account
    pub fn new(address: Address, balance: Balance) -> Self {
        Self {
            address,
            state: AccountState::new(balance),
            resources: HashMap::new(),
            modules: HashMap::new(),
        }
    }

    /// Create empty account
    pub fn empty(address: Address) -> Self {
        Self::new(address, 0)
    }

    /// Get balance
    pub fn balance(&self) -> Balance {
        self.state.balance
    }

    /// Get nonce
    pub fn nonce(&self) -> Nonce {
        self.state.nonce
    }

    /// Check if account can pay amount
    pub fn can_pay(&self, amount: Amount) -> bool {
        self.state.balance >= amount
    }

    /// Debit amount from account
    pub fn debit(&mut self, amount: Amount) -> Result<(), String> {
        if self.state.frozen {
            return Err("Account is frozen".to_string());
        }
        if self.state.balance < amount {
            return Err("Insufficient balance".to_string());
        }
        self.state.balance -= amount;
        Ok(())
    }

    /// Credit amount to account
    pub fn credit(&mut self, amount: Amount) {
        self.state.balance += amount;
    }

    /// Store resource
    pub fn set_resource(&mut self, type_tag: String, data: Vec<u8>) {
        self.resources.insert(type_tag, data);
    }

    /// Get resource
    pub fn get_resource(&self, type_tag: &str) -> Option<&Vec<u8>> {
        self.resources.get(type_tag)
    }

    /// Remove resource
    pub fn remove_resource(&mut self, type_tag: &str) -> Option<Vec<u8>> {
        self.resources.remove(type_tag)
    }

    /// Store module
    pub fn set_module(&mut self, name: String, bytecode: Vec<u8>) {
        self.modules.insert(name, bytecode);
    }

    /// Get module
    pub fn get_module(&self, name: &str) -> Option<&Vec<u8>> {
        self.modules.get(name)
    }

    /// Check if has module
    pub fn has_module(&self, name: &str) -> bool {
        self.modules.contains_key(name)
    }
}

/// Validator info (for staking)
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct ValidatorInfo {
    /// Validator address
    pub address: Address,
    
    /// Validator name/moniker
    pub name: String,
    
    /// Staked amount (self + delegated)
    pub stake: Amount,
    
    /// Commission rate (basis points, e.g., 500 = 5%)
    pub commission_rate: u32,
    
    /// Is validator active
    pub active: bool,
    
    /// Is validator jailed
    pub jailed: bool,
    
    /// Blocks proposed
    pub blocks_proposed: u64,
    
    /// Blocks attested
    pub blocks_attested: u64,
    
    /// Uptime percentage (0-100)
    pub uptime: u32,
}

impl ValidatorInfo {
    /// Create new validator
    pub fn new(address: Address, name: String, stake: Amount) -> Self {
        Self {
            address,
            name,
            stake,
            commission_rate: 500, // 5% default
            active: true,
            jailed: false,
            blocks_proposed: 0,
            blocks_attested: 0,
            uptime: 100,
        }
    }

    /// Calculate voting power
    pub fn voting_power(&self) -> u64 {
        if self.active && !self.jailed {
            (self.stake / 1_000_000_000) as u64 // 1 power per JASPR
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_balance() {
        let addr = Address::from_public_key(b"test");
        let mut account = Account::new(addr, 1000);
        
        assert_eq!(account.balance(), 1000);
        assert!(account.can_pay(500));
        assert!(!account.can_pay(2000));
        
        account.debit(300).unwrap();
        assert_eq!(account.balance(), 700);
        
        account.credit(100);
        assert_eq!(account.balance(), 800);
    }

    #[test]
    fn test_account_resources() {
        let addr = Address::from_public_key(b"test");
        let mut account = Account::new(addr, 0);
        
        account.set_resource("0x1::Coin::Coin".to_string(), vec![1, 2, 3]);
        assert!(account.get_resource("0x1::Coin::Coin").is_some());
        
        let removed = account.remove_resource("0x1::Coin::Coin");
        assert!(removed.is_some());
        assert!(account.get_resource("0x1::Coin::Coin").is_none());
    }

    #[test]
    fn test_validator_voting_power() {
        let addr = Address::from_public_key(b"validator");
        let validator = ValidatorInfo::new(addr, "Test".to_string(), 10_000_000_000_000);
        
        assert_eq!(validator.voting_power(), 10000);
    }
}
