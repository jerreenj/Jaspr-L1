//! Validator set management

use jaspr_types::{Address, Amount, ValidatorInfo, BlockHeight};
use parking_lot::RwLock;
use std::collections::HashMap;
use rand::seq::SliceRandom;
use rand::SeedableRng;

/// Minimum stake to become validator
pub const MIN_VALIDATOR_STAKE: Amount = 32_000_000_000_000; // 32,000 JASPR

/// Maximum number of active validators
pub const MAX_VALIDATORS: usize = 100;

/// Validator status
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidatorStatus {
    /// Validator is active and can propose/attest
    Active,
    /// Validator is inactive (below min stake or voluntarily)
    Inactive,
    /// Validator is jailed (slashed)
    Jailed,
    /// Validator is in unbonding period
    Unbonding,
}

/// Validator with runtime status
#[derive(Clone, Debug)]
pub struct ValidatorEntry {
    pub info: ValidatorInfo,
    pub status: ValidatorStatus,
    /// Blocks since last attestation
    pub blocks_since_attestation: u64,
    /// Total rewards earned
    pub total_rewards: Amount,
    /// Slashing count
    pub slash_count: u32,
}

impl ValidatorEntry {
    pub fn new(info: ValidatorInfo) -> Self {
        Self {
            status: if info.stake >= MIN_VALIDATOR_STAKE {
                ValidatorStatus::Active
            } else {
                ValidatorStatus::Inactive
            },
            info,
            blocks_since_attestation: 0,
            total_rewards: 0,
            slash_count: 0,
        }
    }
    
    pub fn voting_power(&self) -> u64 {
        if self.status == ValidatorStatus::Active {
            self.info.voting_power()
        } else {
            0
        }
    }
}

/// Validator set with selection logic
pub struct ValidatorSet {
    /// All registered validators
    validators: RwLock<HashMap<Address, ValidatorEntry>>,
    /// Current epoch
    epoch: RwLock<u64>,
    /// Blocks per epoch
    blocks_per_epoch: u64,
}

impl ValidatorSet {
    /// Create new validator set
    pub fn new(blocks_per_epoch: u64) -> Self {
        Self {
            validators: RwLock::new(HashMap::new()),
            epoch: RwLock::new(0),
            blocks_per_epoch,
        }
    }
    
    /// Register a new validator
    pub fn register_validator(&self, info: ValidatorInfo) -> Result<(), String> {
        if info.stake < MIN_VALIDATOR_STAKE {
            return Err(format!(
                "Stake {} below minimum {}", 
                info.stake, 
                MIN_VALIDATOR_STAKE
            ));
        }
        
        let entry = ValidatorEntry::new(info.clone());
        self.validators.write().insert(info.address, entry);
        Ok(())
    }
    
    /// Update validator stake
    pub fn update_stake(&self, address: &Address, new_stake: Amount) -> Result<(), String> {
        let mut validators = self.validators.write();
        let entry = validators.get_mut(address)
            .ok_or_else(|| "Validator not found".to_string())?;
        
        entry.info.stake = new_stake;
        
        // Update status based on new stake
        if new_stake < MIN_VALIDATOR_STAKE {
            entry.status = ValidatorStatus::Inactive;
        } else if entry.status == ValidatorStatus::Inactive {
            entry.status = ValidatorStatus::Active;
        }
        
        Ok(())
    }
    
    /// Get validator by address
    pub fn get_validator(&self, address: &Address) -> Option<ValidatorEntry> {
        self.validators.read().get(address).cloned()
    }
    
    /// Get all active validators
    pub fn get_active_validators(&self) -> Vec<ValidatorEntry> {
        self.validators.read()
            .values()
            .filter(|v| v.status == ValidatorStatus::Active)
            .cloned()
            .collect()
    }
    
    /// Get all validators
    pub fn get_all_validators(&self) -> Vec<ValidatorEntry> {
        self.validators.read().values().cloned().collect()
    }
    
    /// Total staked amount
    pub fn total_stake(&self) -> Amount {
        self.validators.read()
            .values()
            .filter(|v| v.status == ValidatorStatus::Active)
            .map(|v| v.info.stake)
            .sum()
    }
    
    /// Total voting power
    pub fn total_voting_power(&self) -> u64 {
        self.get_active_validators()
            .iter()
            .map(|v| v.voting_power())
            .sum()
    }
    
    /// Select block proposer for given height
    /// Uses deterministic weighted random selection based on stake
    pub fn select_proposer(&self, height: BlockHeight) -> Option<Address> {
        let validators = self.get_active_validators();
        if validators.is_empty() {
            return None;
        }
        
        // Calculate total voting power
        let total_power: u64 = validators.iter().map(|v| v.voting_power()).sum();
        if total_power == 0 {
            return None;
        }
        
        // Use block height as seed for deterministic selection
        let mut rng = rand::rngs::StdRng::seed_from_u64(height);
        
        // Weighted selection
        let mut cumulative = 0u64;
        let target = rand::Rng::gen_range(&mut rng, 0..total_power);
        
        for validator in &validators {
            cumulative += validator.voting_power();
            if target < cumulative {
                return Some(validator.info.address);
            }
        }
        
        // Fallback to last validator
        validators.last().map(|v| v.info.address)
    }
    
    /// Select committee for attestation
    pub fn select_committee(&self, height: BlockHeight, size: usize) -> Vec<Address> {
        let validators = self.get_active_validators();
        if validators.is_empty() {
            return Vec::new();
        }
        
        let mut rng = rand::rngs::StdRng::seed_from_u64(height.wrapping_mul(7919)); // Different seed
        let mut shuffled = validators;
        shuffled.shuffle(&mut rng);
        
        shuffled.into_iter()
            .take(size.min(MAX_VALIDATORS))
            .map(|v| v.info.address)
            .collect()
    }
    
    /// Jail a validator (for slashing)
    pub fn jail_validator(&self, address: &Address) -> Result<(), String> {
        let mut validators = self.validators.write();
        let entry = validators.get_mut(address)
            .ok_or_else(|| "Validator not found".to_string())?;
        
        entry.status = ValidatorStatus::Jailed;
        entry.info.jailed = true;
        entry.slash_count += 1;
        
        Ok(())
    }
    
    /// Unjail a validator
    pub fn unjail_validator(&self, address: &Address) -> Result<(), String> {
        let mut validators = self.validators.write();
        let entry = validators.get_mut(address)
            .ok_or_else(|| "Validator not found".to_string())?;
        
        if entry.info.stake < MIN_VALIDATOR_STAKE {
            return Err("Stake below minimum".to_string());
        }
        
        entry.status = ValidatorStatus::Active;
        entry.info.jailed = false;
        
        Ok(())
    }
    
    /// Record attestation from validator
    pub fn record_attestation(&self, address: &Address) {
        let mut validators = self.validators.write();
        if let Some(entry) = validators.get_mut(address) {
            entry.blocks_since_attestation = 0;
            entry.info.blocks_attested += 1;
        }
    }
    
    /// Record block proposal
    pub fn record_proposal(&self, address: &Address) {
        let mut validators = self.validators.write();
        if let Some(entry) = validators.get_mut(address) {
            entry.info.blocks_proposed += 1;
        }
    }
    
    /// Increment epoch
    pub fn advance_epoch(&self) {
        let mut epoch = self.epoch.write();
        *epoch += 1;
    }
    
    /// Get current epoch
    pub fn current_epoch(&self) -> u64 {
        *self.epoch.read()
    }
    
    /// Calculate epoch from block height
    pub fn epoch_for_height(&self, height: BlockHeight) -> u64 {
        height / self.blocks_per_epoch
    }
    
    /// Validator count
    pub fn validator_count(&self) -> usize {
        self.validators.read().len()
    }
    
    /// Active validator count
    pub fn active_count(&self) -> usize {
        self.get_active_validators().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_validator(name: &str, stake: Amount) -> ValidatorInfo {
        ValidatorInfo::new(
            Address::from_public_key(name.as_bytes()),
            name.to_string(),
            stake,
        )
    }
    
    #[test]
    fn test_register_validator() {
        let set = ValidatorSet::new(100);
        
        let validator = create_test_validator("validator1", MIN_VALIDATOR_STAKE);
        set.register_validator(validator.clone()).unwrap();
        
        let entry = set.get_validator(&validator.address).unwrap();
        assert_eq!(entry.status, ValidatorStatus::Active);
    }
    
    #[test]
    fn test_reject_low_stake() {
        let set = ValidatorSet::new(100);
        
        let validator = create_test_validator("validator1", MIN_VALIDATOR_STAKE - 1);
        assert!(set.register_validator(validator).is_err());
    }
    
    #[test]
    fn test_proposer_selection() {
        let set = ValidatorSet::new(100);
        
        for i in 0..5 {
            let validator = create_test_validator(&format!("v{}", i), MIN_VALIDATOR_STAKE * (i as u128 + 1));
            set.register_validator(validator).unwrap();
        }
        
        // Proposer selection should be deterministic
        let proposer1 = set.select_proposer(100);
        let proposer2 = set.select_proposer(100);
        assert_eq!(proposer1, proposer2);
        
        // Different heights should potentially have different proposers
        let proposer3 = set.select_proposer(101);
        // Note: might be same by chance, but selection works
        assert!(proposer3.is_some());
    }
    
    #[test]
    fn test_jail_unjail() {
        let set = ValidatorSet::new(100);
        
        let validator = create_test_validator("validator1", MIN_VALIDATOR_STAKE);
        set.register_validator(validator.clone()).unwrap();
        
        set.jail_validator(&validator.address).unwrap();
        let entry = set.get_validator(&validator.address).unwrap();
        assert_eq!(entry.status, ValidatorStatus::Jailed);
        
        set.unjail_validator(&validator.address).unwrap();
        let entry = set.get_validator(&validator.address).unwrap();
        assert_eq!(entry.status, ValidatorStatus::Active);
    }
}
