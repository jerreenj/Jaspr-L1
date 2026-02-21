//! Block attestation and finality

use jaspr_types::{Address, HashValue, BlockHeight};
use jaspr_crypto::{Signature, PublicKey};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};

/// Attestation for a block
#[derive(Clone, Debug)]
pub struct Attestation {
    /// Block hash being attested
    pub block_hash: HashValue,
    /// Block height
    pub block_height: BlockHeight,
    /// Validator address
    pub validator: Address,
    /// Validator's signature
    pub signature: Signature,
    /// Validator's voting power at time of attestation
    pub voting_power: u64,
}

impl Attestation {
    /// Create new attestation
    pub fn new(
        block_hash: HashValue,
        block_height: BlockHeight,
        validator: Address,
        signature: Signature,
        voting_power: u64,
    ) -> Self {
        Self {
            block_hash,
            block_height,
            validator,
            signature,
            voting_power,
        }
    }
    
    /// Verify attestation signature
    pub fn verify(&self, public_key: &PublicKey) -> bool {
        let message = self.signing_message();
        public_key.verify(&message, &self.signature)
    }
    
    /// Get message that was signed
    fn signing_message(&self) -> Vec<u8> {
        let mut msg = Vec::with_capacity(72);
        msg.extend_from_slice(self.block_hash.as_bytes());
        msg.extend_from_slice(&self.block_height.to_le_bytes());
        msg
    }
}

use jaspr_crypto::Verifier;

/// Pool of attestations for pending blocks
pub struct AttestationPool {
    /// Attestations by block hash
    attestations: RwLock<HashMap<HashValue, Vec<Attestation>>>,
    /// Validators who have attested per block (to prevent double voting)
    attested: RwLock<HashMap<HashValue, HashSet<Address>>>,
    /// Finality threshold (percentage of voting power, e.g., 67 = 2/3)
    finality_threshold: u32,
}

impl AttestationPool {
    /// Create new attestation pool
    pub fn new(finality_threshold: u32) -> Self {
        Self {
            attestations: RwLock::new(HashMap::new()),
            attested: RwLock::new(HashMap::new()),
            finality_threshold,
        }
    }
    
    /// Add attestation
    pub fn add_attestation(&self, attestation: Attestation) -> Result<(), String> {
        let block_hash = attestation.block_hash;
        let validator = attestation.validator;
        
        // Check for double voting
        {
            let attested = self.attested.read();
            if let Some(voters) = attested.get(&block_hash) {
                if voters.contains(&validator) {
                    return Err("Validator already attested".to_string());
                }
            }
        }
        
        // Add attestation
        {
            let mut attestations = self.attestations.write();
            attestations.entry(block_hash).or_default().push(attestation);
        }
        
        // Mark validator as having attested
        {
            let mut attested = self.attested.write();
            attested.entry(block_hash).or_default().insert(validator);
        }
        
        Ok(())
    }
    
    /// Get attestations for a block
    pub fn get_attestations(&self, block_hash: &HashValue) -> Vec<Attestation> {
        self.attestations.read()
            .get(block_hash)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Get total voting power attested for a block
    pub fn attested_voting_power(&self, block_hash: &HashValue) -> u64 {
        self.get_attestations(block_hash)
            .iter()
            .map(|a| a.voting_power)
            .sum()
    }
    
    /// Check if block has reached finality
    pub fn is_finalized(&self, block_hash: &HashValue, total_voting_power: u64) -> bool {
        if total_voting_power == 0 {
            return false;
        }
        
        let attested_power = self.attested_voting_power(block_hash);
        let threshold = (total_voting_power * self.finality_threshold as u64) / 100;
        
        attested_power >= threshold
    }
    
    /// Get attestation count for a block
    pub fn attestation_count(&self, block_hash: &HashValue) -> usize {
        self.attestations.read()
            .get(block_hash)
            .map(|v| v.len())
            .unwrap_or(0)
    }
    
    /// Clear attestations for finalized blocks (cleanup)
    pub fn clear_finalized(&self, block_hashes: &[HashValue]) {
        let mut attestations = self.attestations.write();
        let mut attested = self.attested.write();
        
        for hash in block_hashes {
            attestations.remove(hash);
            attested.remove(hash);
        }
    }
    
    /// Prune old attestations (before height)
    pub fn prune_before_height(&self, height: BlockHeight) {
        let mut attestations = self.attestations.write();
        let mut attested = self.attested.write();
        
        let to_remove: Vec<HashValue> = attestations
            .iter()
            .filter(|(_, atts)| atts.first().map(|a| a.block_height < height).unwrap_or(true))
            .map(|(hash, _)| *hash)
            .collect();
        
        for hash in to_remove {
            attestations.remove(&hash);
            attested.remove(&hash);
        }
    }
}

impl Default for AttestationPool {
    fn default() -> Self {
        Self::new(67) // 2/3 threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jaspr_crypto::KeyPair;
    
    fn create_test_attestation(height: BlockHeight) -> Attestation {
        let keypair = KeyPair::generate();
        let block_hash = HashValue::sha256(&height.to_le_bytes());
        
        let msg = {
            let mut m = Vec::with_capacity(72);
            m.extend_from_slice(block_hash.as_bytes());
            m.extend_from_slice(&height.to_le_bytes());
            m
        };
        
        let signature = keypair.sign(&msg);
        
        Attestation::new(
            block_hash,
            height,
            keypair.address(),
            signature,
            100,
        )
    }
    
    #[test]
    fn test_add_attestation() {
        let pool = AttestationPool::new(67);
        
        let att = create_test_attestation(1);
        pool.add_attestation(att.clone()).unwrap();
        
        let attestations = pool.get_attestations(&att.block_hash);
        assert_eq!(attestations.len(), 1);
    }
    
    #[test]
    fn test_no_double_voting() {
        let pool = AttestationPool::new(67);
        
        let keypair = KeyPair::generate();
        let block_hash = HashValue::sha256(b"block1");
        
        let att1 = Attestation::new(
            block_hash,
            1,
            keypair.address(),
            Signature::from_bytes([0u8; 64]),
            100,
        );
        
        pool.add_attestation(att1.clone()).unwrap();
        
        // Second attestation from same validator should fail
        let result = pool.add_attestation(att1);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_finality() {
        let pool = AttestationPool::new(67);
        let block_hash = HashValue::sha256(b"block1");
        
        // Add attestations totaling 70% voting power
        for i in 0..7 {
            let keypair = KeyPair::generate();
            let att = Attestation::new(
                block_hash,
                1,
                keypair.address(),
                Signature::from_bytes([0u8; 64]),
                10, // 10 voting power each
            );
            pool.add_attestation(att).unwrap();
        }
        
        // With 70 out of 100 total, should be finalized (> 67%)
        assert!(pool.is_finalized(&block_hash, 100));
        
        // With 70 out of 200 total, should not be finalized (35%)
        assert!(!pool.is_finalized(&block_hash, 200));
    }
}
