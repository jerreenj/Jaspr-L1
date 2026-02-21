//! VRF-based proposer selection

use crate::consensus::{Validator, ValidatorSet};
use rand::Rng;

/// VRF output for proposer selection
pub struct VRFOutput {
    pub value: [u8; 32],
    pub proof: [u8; 32],
}

impl VRFOutput {
    pub fn as_int(&self) -> u64 {
        u64::from_be_bytes(self.value[..8].try_into().unwrap_or_default())
    }
}

/// Weighted VRF proposer selection
pub struct ProposerSelection<'a> {
    validator_set: &'a ValidatorSet,
    last_proposer: Option<String>,
}

impl<'a> ProposerSelection<'a> {
    pub fn new(validator_set: &'a ValidatorSet) -> Self {
        Self {
            validator_set,
            last_proposer: None,
        }
    }
    
    /// Generate VRF from seed
    pub fn generate_vrf(&self, seed: &[u8]) -> VRFOutput {
        use sha2::{Sha256, Digest};
        
        let mut rng = rand::thread_rng();
        let random_key: [u8; 32] = rng.gen();
        
        let mut hasher = Sha256::new();
        hasher.update(&random_key);
        hasher.update(seed);
        let value: [u8; 32] = hasher.finalize().into();
        
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(&random_key);
        let proof: [u8; 32] = hasher.finalize().into();
        
        VRFOutput { value, proof }
    }
    
    /// Select proposer for a given height
    pub fn select_proposer(&mut self, height: u64, previous_hash: &str) -> Option<&Validator> {
        let active = self.validator_set.get_active_validators();
        if active.is_empty() {
            return None;
        }
        
        let seed = format!("{}:{}", previous_hash, height);
        let vrf = self.generate_vrf(seed.as_bytes());
        
        // Calculate weighted scores
        let mut scores: Vec<(&Validator, u64)> = active
            .iter()
            .map(|v| {
                let base = v.stake;
                let uptime_bonus = (base as f64 * 0.1 * (v.stats.uptime_percentage / 100.0)) as u64;
                let penalty = if Some(v.address.clone()) == self.last_proposer {
                    (base as f64 * 0.05) as u64
                } else {
                    0
                };
                (*v, base + uptime_bonus - penalty)
            })
            .collect();
        
        let total_score: u64 = scores.iter().map(|(_, s)| s).sum();
        if total_score == 0 {
            return Some(active[0]);
        }
        
        let target = vrf.as_int() % total_score;
        let mut cumulative = 0u64;
        
        for (validator, score) in &scores {
            cumulative += score;
            if cumulative > target {
                self.last_proposer = Some(validator.address.clone());
                return Some(validator);
            }
        }
        
        scores.last().map(|(v, _)| *v)
    }
}
