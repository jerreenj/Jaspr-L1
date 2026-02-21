//! Committee finality engine with BLS aggregation

use crate::consensus::{Block, ValidatorSet};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Aggregated committee attestation for a block
pub struct CommitteeAttestation {
    pub block_hash: String,
    pub height: u64,
    pub aggregate_signature: Vec<u8>,
    pub signers: Vec<String>,
    pub voting_power: u64,
    pub total_voting_power: u64,
    pub finalized: bool,
    pub finality_timestamp: Option<u64>,
}

impl CommitteeAttestation {
    pub fn participation_rate(&self) -> f64 {
        if self.total_voting_power == 0 {
            return 0.0;
        }
        (self.voting_power as f64 / self.total_voting_power as f64) * 100.0
    }
}

/// Vote message from a validator
struct VoteMessage {
    block_hash: String,
    height: u64,
    validator_address: String,
    signature: Vec<u8>,
    timestamp: u64,
}

/// Manages block finality through BLS attestations
pub struct FinalityEngine {
    prevotes: HashMap<String, Vec<VoteMessage>>,
    precommits: HashMap<String, Vec<VoteMessage>>,
    attestations: HashMap<String, CommitteeAttestation>,
    proposal_times: HashMap<String, u64>,
}

impl FinalityEngine {
    pub const TARGET_FINALITY_MS: u64 = 2000;
    
    pub fn new() -> Self {
        Self {
            prevotes: HashMap::new(),
            precommits: HashMap::new(),
            attestations: HashMap::new(),
            proposal_times: HashMap::new(),
        }
    }
    
    pub fn on_block_proposed(&mut self, block: &Block) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.proposal_times.insert(block.hash(), now);
    }
    
    pub fn add_prevote(
        &mut self,
        block_hash: &str,
        height: u64,
        validator: &str,
        signature: Vec<u8>,
    ) -> bool {
        let votes = self.prevotes.entry(block_hash.to_string()).or_default();
        
        if votes.iter().any(|v| v.validator_address == validator) {
            return false;
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        votes.push(VoteMessage {
            block_hash: block_hash.to_string(),
            height,
            validator_address: validator.to_string(),
            signature,
            timestamp: now,
        });
        
        true
    }
    
    pub fn add_precommit(
        &mut self,
        block_hash: &str,
        height: u64,
        validator: &str,
        signature: Vec<u8>,
        validator_set: &ValidatorSet,
    ) -> Option<CommitteeAttestation> {
        let votes = self.precommits.entry(block_hash.to_string()).or_default();
        
        if votes.iter().any(|v| v.validator_address == validator) {
            return None;
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        votes.push(VoteMessage {
            block_hash: block_hash.to_string(),
            height,
            validator_address: validator.to_string(),
            signature,
            timestamp: now,
        });
        
        self.check_finality(block_hash, height, validator_set)
    }
    
    fn check_finality(
        &mut self,
        block_hash: &str,
        height: u64,
        validator_set: &ValidatorSet,
    ) -> Option<CommitteeAttestation> {
        let votes = self.precommits.get(block_hash)?;
        
        let mut voting_power = 0u64;
        let mut signers = Vec::new();
        let mut signatures = Vec::new();
        
        for vote in votes {
            if let Some(v) = validator_set.get_validator(&vote.validator_address) {
                if v.active && !v.jailed {
                    voting_power += v.voting_power();
                    signers.push(vote.validator_address.clone());
                    signatures.extend(&vote.signature);
                }
            }
        }
        
        let total = validator_set.total_voting_power();
        let finalized = validator_set.has_supermajority(voting_power);
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let attestation = CommitteeAttestation {
            block_hash: block_hash.to_string(),
            height,
            aggregate_signature: signatures,
            signers,
            voting_power,
            total_voting_power: total,
            finalized,
            finality_timestamp: if finalized { Some(now) } else { None },
        };
        
        if finalized {
            self.attestations.insert(block_hash.to_string(), attestation.clone());
        }
        
        Some(attestation)
    }
    
    pub fn is_finalized(&self, block_hash: &str) -> bool {
        self.attestations
            .get(block_hash)
            .map(|a| a.finalized)
            .unwrap_or(false)
    }
    
    pub fn get_finality_time(&self, block_hash: &str) -> Option<u64> {
        let proposal_time = self.proposal_times.get(block_hash)?;
        let attestation = self.attestations.get(block_hash)?;
        attestation.finality_timestamp.map(|t| t - proposal_time)
    }
}

impl Default for FinalityEngine {
    fn default() -> Self {
        Self::new()
    }
}
