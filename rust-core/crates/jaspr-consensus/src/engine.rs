//! Main consensus engine

use crate::{ValidatorSet, BlockProducer, BlockProducerConfig, AttestationPool, Attestation};
use jaspr_types::{Block, SignedTransaction, HashValue, Address, BlockHeight, TransactionReceipt};
use jaspr_state::{BlockStore, AccountStore, StateTree};
use jaspr_crypto::{KeyPair, Signer};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{info, warn, error, debug};

/// Consensus configuration
#[derive(Clone, Debug)]
pub struct ConsensusConfig {
    /// Block time in milliseconds
    pub block_time_ms: u64,
    /// Blocks per epoch
    pub blocks_per_epoch: u64,
    /// Finality threshold (percentage)
    pub finality_threshold: u32,
    /// Chain ID
    pub chain_id: u64,
    /// Maximum transactions per block
    pub max_txs_per_block: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            block_time_ms: 2000, // 2 seconds
            blocks_per_epoch: 100,
            finality_threshold: 67, // 2/3
            chain_id: 1,
            max_txs_per_block: 10000,
        }
    }
}

/// Consensus engine state
pub enum ConsensusState {
    /// Syncing with network
    Syncing,
    /// Ready to participate
    Ready,
    /// Actively producing/validating
    Active,
    /// Stopped
    Stopped,
}

/// Consensus engine events
#[derive(Clone, Debug)]
pub enum ConsensusEvent {
    /// New block produced
    BlockProduced(Block),
    /// Block finalized
    BlockFinalized(BlockHeight, HashValue),
    /// New attestation
    AttestationReceived(Attestation),
    /// Epoch changed
    EpochChanged(u64),
}

/// Main consensus engine
pub struct ConsensusEngine {
    config: ConsensusConfig,
    validator_set: Arc<ValidatorSet>,
    block_producer: Arc<BlockProducer>,
    attestation_pool: Arc<AttestationPool>,
    state: RwLock<ConsensusState>,
    /// Our validator key (if we're a validator)
    validator_key: Option<KeyPair>,
    /// Current block height
    current_height: RwLock<BlockHeight>,
    /// Event channel
    event_tx: Option<mpsc::UnboundedSender<ConsensusEvent>>,
}

impl ConsensusEngine {
    /// Create new consensus engine
    pub fn new(config: ConsensusConfig) -> Self {
        let validator_set = Arc::new(ValidatorSet::new(config.blocks_per_epoch));
        
        let block_config = BlockProducerConfig {
            max_txs_per_block: config.max_txs_per_block,
            gas_limit: 100_000_000,
            chain_id: config.chain_id,
        };
        
        let block_producer = Arc::new(BlockProducer::new(block_config));
        let attestation_pool = Arc::new(AttestationPool::new(config.finality_threshold));
        
        Self {
            config,
            validator_set,
            block_producer,
            attestation_pool,
            state: RwLock::new(ConsensusState::Ready),
            validator_key: None,
            current_height: RwLock::new(0),
            event_tx: None,
        }
    }
    
    /// Set validator key (makes this node a validator)
    pub fn set_validator_key(&mut self, key: KeyPair) {
        self.validator_key = Some(key);
    }
    
    /// Set event channel
    pub fn set_event_channel(&mut self, tx: mpsc::UnboundedSender<ConsensusEvent>) {
        self.event_tx = Some(tx);
    }
    
    /// Get validator set
    pub fn validator_set(&self) -> &Arc<ValidatorSet> {
        &self.validator_set
    }
    
    /// Get block producer
    pub fn block_producer(&self) -> &Arc<BlockProducer> {
        &self.block_producer
    }
    
    /// Get attestation pool
    pub fn attestation_pool(&self) -> &Arc<AttestationPool> {
        &self.attestation_pool
    }
    
    /// Get current height
    pub fn current_height(&self) -> BlockHeight {
        *self.current_height.read()
    }
    
    /// Set current height
    pub fn set_height(&self, height: BlockHeight) {
        *self.current_height.write() = height;
    }
    
    /// Add transaction to mempool
    pub fn submit_transaction(&self, tx: SignedTransaction) {
        self.block_producer.add_transaction(tx);
    }
    
    /// Check if we are the proposer for given height
    pub fn is_proposer(&self, height: BlockHeight) -> bool {
        if let Some(key) = &self.validator_key {
            if let Some(proposer) = self.validator_set.select_proposer(height) {
                return proposer == key.address();
            }
        }
        false
    }
    
    /// Produce a block (if we're the proposer)
    pub fn try_produce_block(&self, previous_hash: HashValue, state_root: HashValue) -> Option<Block> {
        let height = self.current_height() + 1;
        
        if !self.is_proposer(height) {
            return None;
        }
        
        let key = self.validator_key.as_ref()?;
        
        let block = self.block_producer.produce_block(
            height,
            previous_hash,
            key.address(),
            state_root,
        );
        
        // Record proposal
        self.validator_set.record_proposal(&key.address());
        
        info!(height = height, txs = block.body.tx_count(), "Produced block");
        
        // Emit event
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(ConsensusEvent::BlockProduced(block.clone()));
        }
        
        *self.current_height.write() = height;
        
        Some(block)
    }
    
    /// Validate a received block
    pub fn validate_block(&self, block: &Block, previous: &Block) -> Result<(), String> {
        // Check height is sequential
        if block.height() != previous.height() + 1 {
            return Err(format!(
                "Invalid height: expected {}, got {}",
                previous.height() + 1,
                block.height()
            ));
        }
        
        // Check previous hash matches
        let expected_prev = previous.hash();
        if block.header.previous_hash != expected_prev {
            return Err("Previous hash mismatch".to_string());
        }
        
        // Check proposer was valid for this height
        let expected_proposer = self.validator_set.select_proposer(block.height());
        if expected_proposer != Some(block.header.proposer) {
            return Err("Invalid proposer".to_string());
        }
        
        // Validate block structure
        block.validate()?;
        
        // Check timestamp is reasonable
        if block.header.timestamp <= previous.header.timestamp {
            return Err("Timestamp not advancing".to_string());
        }
        
        Ok(())
    }
    
    /// Create and submit attestation for a block
    pub fn attest_block(&self, block: &Block) -> Option<Attestation> {
        let key = self.validator_key.as_ref()?;
        
        // Check if we're in the committee
        let committee = self.validator_set.select_committee(block.height(), 20);
        if !committee.contains(&key.address()) {
            return None;
        }
        
        // Get our voting power
        let validator = self.validator_set.get_validator(&key.address())?;
        let voting_power = validator.voting_power();
        
        // Create attestation
        let block_hash = block.hash();
        let mut msg = Vec::with_capacity(72);
        msg.extend_from_slice(block_hash.as_bytes());
        msg.extend_from_slice(&block.height().to_le_bytes());
        
        let signature = key.sign(&msg);
        
        let attestation = Attestation::new(
            block_hash,
            block.height(),
            key.address(),
            signature,
            voting_power,
        );
        
        // Add to pool
        if let Err(e) = self.attestation_pool.add_attestation(attestation.clone()) {
            warn!(error = %e, "Failed to add own attestation");
            return None;
        }
        
        // Record attestation
        self.validator_set.record_attestation(&key.address());
        
        // Emit event
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(ConsensusEvent::AttestationReceived(attestation.clone()));
        }
        
        debug!(height = block.height(), "Attested block");
        
        Some(attestation)
    }
    
    /// Process received attestation
    pub fn process_attestation(&self, attestation: Attestation) -> Result<(), String> {
        self.attestation_pool.add_attestation(attestation.clone())?;
        
        // Record in validator set
        self.validator_set.record_attestation(&attestation.validator);
        
        // Check if block is now finalized
        let total_power = self.validator_set.total_voting_power();
        if self.attestation_pool.is_finalized(&attestation.block_hash, total_power) {
            info!(height = attestation.block_height, "Block finalized");
            
            if let Some(tx) = &self.event_tx {
                let _ = tx.send(ConsensusEvent::BlockFinalized(
                    attestation.block_height,
                    attestation.block_hash,
                ));
            }
        }
        
        Ok(())
    }
    
    /// Check if block is finalized
    pub fn is_finalized(&self, block_hash: &HashValue) -> bool {
        let total_power = self.validator_set.total_voting_power();
        self.attestation_pool.is_finalized(block_hash, total_power)
    }
    
    /// Get finality percentage for a block
    pub fn finality_percentage(&self, block_hash: &HashValue) -> f64 {
        let total_power = self.validator_set.total_voting_power();
        if total_power == 0 {
            return 0.0;
        }
        
        let attested_power = self.attestation_pool.attested_voting_power(block_hash);
        (attested_power as f64 / total_power as f64) * 100.0
    }
    
    /// Process epoch transition
    pub fn check_epoch_transition(&self, height: BlockHeight) {
        let new_epoch = self.validator_set.epoch_for_height(height);
        let current_epoch = self.validator_set.current_epoch();
        
        if new_epoch > current_epoch {
            self.validator_set.advance_epoch();
            
            info!(epoch = new_epoch, "Epoch transition");
            
            if let Some(tx) = &self.event_tx {
                let _ = tx.send(ConsensusEvent::EpochChanged(new_epoch));
            }
        }
    }
    
    /// Get consensus stats
    pub fn stats(&self) -> ConsensusStats {
        ConsensusStats {
            height: self.current_height(),
            epoch: self.validator_set.current_epoch(),
            validator_count: self.validator_set.validator_count(),
            active_validators: self.validator_set.active_count(),
            total_stake: self.validator_set.total_stake(),
            total_voting_power: self.validator_set.total_voting_power(),
            pending_txs: self.block_producer.pending_count(),
        }
    }
}

/// Consensus statistics
#[derive(Clone, Debug)]
pub struct ConsensusStats {
    pub height: BlockHeight,
    pub epoch: u64,
    pub validator_count: usize,
    pub active_validators: usize,
    pub total_stake: u128,
    pub total_voting_power: u64,
    pub pending_txs: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use jaspr_types::ValidatorInfo;
    use crate::validator_set::MIN_VALIDATOR_STAKE;
    
    fn setup_engine_with_validators(count: usize) -> ConsensusEngine {
        let mut engine = ConsensusEngine::new(ConsensusConfig::default());
        
        for i in 0..count {
            let keypair = KeyPair::generate();
            let validator = ValidatorInfo::new(
                keypair.address(),
                format!("Validator {}", i),
                MIN_VALIDATOR_STAKE,
            );
            engine.validator_set.register_validator(validator).unwrap();
            
            // Set first validator as our key
            if i == 0 {
                engine.set_validator_key(keypair);
            }
        }
        
        engine
    }
    
    #[test]
    fn test_engine_creation() {
        let engine = ConsensusEngine::new(ConsensusConfig::default());
        assert_eq!(engine.current_height(), 0);
        assert_eq!(engine.validator_set.validator_count(), 0);
    }
    
    #[test]
    fn test_proposer_selection() {
        let engine = setup_engine_with_validators(5);
        
        // Should have a proposer
        let proposer = engine.validator_set.select_proposer(1);
        assert!(proposer.is_some());
    }
    
    #[test]
    fn test_stats() {
        let engine = setup_engine_with_validators(5);
        
        let stats = engine.stats();
        assert_eq!(stats.validator_count, 5);
        assert_eq!(stats.active_validators, 5);
        assert!(stats.total_stake > 0);
    }
}
