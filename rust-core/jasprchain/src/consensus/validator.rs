//! Validator management for JasprChain
//! HyperLiquid model: Start with 4 validators, scale up

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::crypto::BLSKeyPair;

/// Performance statistics for a validator
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ValidatorStats {
    pub blocks_proposed: u64,
    pub blocks_attested: u64,
    pub blocks_missed: u64,
    pub uptime_percentage: f64,
    pub last_active: u64,
}

/// Represents a validator node in JasprChain
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Validator {
    pub address: String,
    pub bls_public_key: Vec<u8>,
    pub stake: u64,
    pub active: bool,
    pub jailed: bool,
    pub stats: ValidatorStats,
    pub commission_rate: u32,  // Basis points (100 = 1%)
    pub name: String,
}

impl Validator {
    pub fn new(address: &str, stake: u64, name: &str) -> Self {
        Self {
            address: address.to_string(),
            bls_public_key: Vec::new(),
            stake,
            active: true,
            jailed: false,
            stats: ValidatorStats {
                uptime_percentage: 100.0,
                ..Default::default()
            },
            commission_rate: 500,  // 5%
            name: name.to_string(),
        }
    }
    
    pub fn voting_power(&self) -> u64 {
        if self.active && !self.jailed {
            self.stake
        } else {
            0
        }
    }
}

/// Manages the set of active validators
pub struct ValidatorSet {
    validators: HashMap<String, Validator>,
    bls_keys: HashMap<String, BLSKeyPair>,
}

impl ValidatorSet {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            bls_keys: HashMap::new(),
        }
    }
    
    pub fn add_validator(&mut self, address: &str, stake: u64, name: &str) -> Validator {
        let bls_keys = BLSKeyPair::generate();
        let mut validator = Validator::new(address, stake, name);
        validator.bls_public_key = bls_keys.public_key_bytes();
        
        self.validators.insert(address.to_string(), validator.clone());
        self.bls_keys.insert(address.to_string(), bls_keys);
        
        validator
    }
    
    pub fn get_validator(&self, address: &str) -> Option<&Validator> {
        self.validators.get(address)
    }
    
    pub fn get_active_validators(&self) -> Vec<&Validator> {
        self.validators
            .values()
            .filter(|v| v.active && !v.jailed)
            .collect()
    }
    
    pub fn total_stake(&self) -> u64 {
        self.get_active_validators()
            .iter()
            .map(|v| v.stake)
            .sum()
    }
    
    pub fn total_voting_power(&self) -> u64 {
        self.get_active_validators()
            .iter()
            .map(|v| v.voting_power())
            .sum()
    }
    
    pub fn has_supermajority(&self, voting_power: u64) -> bool {
        let total = self.total_voting_power();
        if total == 0 {
            return false;
        }
        voting_power >= (total * 2) / 3
    }
    
    pub fn slash_validator(&mut self, address: &str, amount: u64) {
        if let Some(validator) = self.validators.get_mut(address) {
            validator.stake = validator.stake.saturating_sub(amount);
            if validator.stake == 0 {
                validator.jailed = true;
            }
        }
    }
}

impl Default for ValidatorSet {
    fn default() -> Self {
        Self::new()
    }
}
