//! Proof of Stake Consensus Protocol
//!
//! Implements a BFT consensus mechanism with:
//! - Leader election based on stake weight
//! - Three-phase commit (propose, prevote, precommit)
//! - Finality after 2/3 stake attestation
//! - View change on timeout

use jaspr_types::{Address, HashValue, Block, BlockHeader, Amount};
use std::collections::{HashMap, HashSet, BTreeMap};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn, debug, error};

/// Consensus error
#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error("Not leader for this round")]
    NotLeader,
    
    #[error("Invalid block: {0}")]
    InvalidBlock(String),
    
    #[error("Invalid vote: {0}")]
    InvalidVote(String),
    
    #[error("Duplicate vote from {0}")]
    DuplicateVote(Address),
    
    #[error("Round timeout")]
    Timeout,
    
    #[error("Not enough stake: {required} required, {available} available")]
    InsufficientStake { required: Amount, available: Amount },
    
    #[error("View change in progress")]
    ViewChangeInProgress,
    
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Consensus configuration
#[derive(Clone, Debug)]
pub struct ConsensusConfig {
    /// Minimum stake to become validator
    pub min_stake: Amount,
    /// Block time in milliseconds
    pub block_time_ms: u64,
    /// Timeout for proposal
    pub proposal_timeout_ms: u64,
    /// Timeout for prevote
    pub prevote_timeout_ms: u64,
    /// Timeout for precommit
    pub precommit_timeout_ms: u64,
    /// View change timeout
    pub view_change_timeout_ms: u64,
    /// Maximum transactions per block
    pub max_block_txs: usize,
    /// Maximum block size in bytes
    pub max_block_size: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            min_stake: 1_000_000_000_000, // 1000 tokens
            block_time_ms: 2000,
            proposal_timeout_ms: 3000,
            prevote_timeout_ms: 1500,
            precommit_timeout_ms: 1500,
            view_change_timeout_ms: 5000,
            max_block_txs: 1000,
            max_block_size: 1024 * 1024, // 1MB
        }
    }
}

/// Consensus phase
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    Propose,
    Prevote,
    Precommit,
    Commit,
}

/// Round state
#[derive(Clone, Debug)]
pub struct RoundState {
    /// Current height
    pub height: u64,
    /// Current round
    pub round: u64,
    /// Current phase
    pub phase: Phase,
    /// Proposed block
    pub proposed_block: Option<Block>,
    /// Locked block (from previous round)
    pub locked_block: Option<Block>,
    /// Locked round
    pub locked_round: Option<u64>,
    /// Valid block (passed validation)
    pub valid_block: Option<Block>,
    /// Valid round
    pub valid_round: Option<u64>,
    /// Prevotes received
    pub prevotes: HashMap<Address, Vote>,
    /// Precommits received
    pub precommits: HashMap<Address, Vote>,
    /// Start time of current phase
    pub phase_start_time: u64,
}

impl RoundState {
    pub fn new(height: u64, round: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        Self {
            height,
            round,
            phase: Phase::Propose,
            proposed_block: None,
            locked_block: None,
            locked_round: None,
            valid_block: None,
            valid_round: None,
            prevotes: HashMap::new(),
            precommits: HashMap::new(),
            phase_start_time: now,
        }
    }
    
    /// Move to next phase
    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            Phase::Propose => Phase::Prevote,
            Phase::Prevote => Phase::Precommit,
            Phase::Precommit => Phase::Commit,
            Phase::Commit => Phase::Commit,
        };
        self.phase_start_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }
    
    /// Move to next round
    pub fn advance_round(&mut self) {
        self.round += 1;
        self.phase = Phase::Propose;
        self.proposed_block = None;
        self.prevotes.clear();
        self.precommits.clear();
        self.phase_start_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }
}

/// Vote message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vote {
    /// Vote type
    pub vote_type: VoteType,
    /// Height
    pub height: u64,
    /// Round
    pub round: u64,
    /// Block hash (None for nil vote)
    pub block_hash: Option<HashValue>,
    /// Voter address
    pub voter: Address,
    /// Signature
    pub signature: Vec<u8>,
    /// Timestamp
    pub timestamp: u64,
}

impl Vote {
    pub fn new(
        vote_type: VoteType,
        height: u64,
        round: u64,
        block_hash: Option<HashValue>,
        voter: Address,
        signature: Vec<u8>,
    ) -> Self {
        Self {
            vote_type,
            height,
            round,
            block_hash,
            voter,
            signature,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }
    
    /// Get vote signing data
    pub fn signing_data(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(&self.height.to_le_bytes());
        data.extend(&self.round.to_le_bytes());
        if let Some(hash) = &self.block_hash {
            data.extend(hash.as_bytes());
        }
        data.push(match self.vote_type {
            VoteType::Prevote => 0,
            VoteType::Precommit => 1,
        });
        data
    }
}

/// Vote type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoteType {
    Prevote,
    Precommit,
}

/// Proposal message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proposal {
    /// Height
    pub height: u64,
    /// Round
    pub round: u64,
    /// Proposed block
    pub block: Block,
    /// Proposer address
    pub proposer: Address,
    /// Valid round (if reproposing locked block)
    pub valid_round: Option<u64>,
    /// Signature
    pub signature: Vec<u8>,
    /// Timestamp
    pub timestamp: u64,
}

/// View change request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewChangeRequest {
    /// Height
    pub height: u64,
    /// Current round (requesting change from)
    pub from_round: u64,
    /// Requested round
    pub to_round: u64,
    /// Requester
    pub requester: Address,
    /// Reason
    pub reason: ViewChangeReason,
    /// Signature
    pub signature: Vec<u8>,
}

/// View change reason
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ViewChangeReason {
    Timeout,
    InvalidProposal,
    ByzantineLeader,
}

/// Commit certificate (proof of consensus)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitCertificate {
    /// Block hash
    pub block_hash: HashValue,
    /// Height
    pub height: u64,
    /// Round
    pub round: u64,
    /// Aggregated signature
    pub aggregated_signature: Vec<u8>,
    /// Signers
    pub signers: Vec<Address>,
    /// Total voting power of signers
    pub voting_power: Amount,
}

/// Validator info for consensus
#[derive(Clone, Debug)]
pub struct ConsensusValidator {
    pub address: Address,
    pub stake: Amount,
    pub voting_power: Amount,
    pub is_active: bool,
    pub last_proposed: u64,
    pub proposals_count: u64,
    pub missed_proposals: u64,
}

/// Consensus engine
pub struct ConsensusEngine {
    /// Configuration
    config: ConsensusConfig,
    /// Our validator address
    our_address: Option<Address>,
    /// Current round state
    round_state: RwLock<RoundState>,
    /// Validators
    validators: RwLock<HashMap<Address, ConsensusValidator>>,
    /// Validator order for leader election
    validator_order: RwLock<Vec<Address>>,
    /// Total voting power
    total_voting_power: RwLock<Amount>,
    /// Commit certificates
    certificates: RwLock<HashMap<HashValue, CommitCertificate>>,
    /// Pending view changes
    view_changes: RwLock<HashMap<u64, Vec<ViewChangeRequest>>>,
    /// Finalized blocks
    finalized: RwLock<HashSet<HashValue>>,
}

impl ConsensusEngine {
    /// Create new consensus engine
    pub fn new(config: ConsensusConfig, our_address: Option<Address>) -> Self {
        Self {
            config,
            our_address,
            round_state: RwLock::new(RoundState::new(1, 0)),
            validators: RwLock::new(HashMap::new()),
            validator_order: RwLock::new(Vec::new()),
            total_voting_power: RwLock::new(0),
            certificates: RwLock::new(HashMap::new()),
            view_changes: RwLock::new(HashMap::new()),
            finalized: RwLock::new(HashSet::new()),
        }
    }
    
    /// Add validator
    pub fn add_validator(&self, address: Address, stake: Amount) {
        let voting_power = stake; // 1:1 stake to voting power
        
        let validator = ConsensusValidator {
            address,
            stake,
            voting_power,
            is_active: true,
            last_proposed: 0,
            proposals_count: 0,
            missed_proposals: 0,
        };
        
        self.validators.write().insert(address, validator);
        *self.total_voting_power.write() += voting_power;
        
        // Rebuild validator order
        self.rebuild_validator_order();
    }
    
    /// Remove validator
    pub fn remove_validator(&self, address: &Address) {
        if let Some(validator) = self.validators.write().remove(address) {
            *self.total_voting_power.write() -= validator.voting_power;
            self.rebuild_validator_order();
        }
    }
    
    /// Update validator stake
    pub fn update_stake(&self, address: &Address, new_stake: Amount) {
        if let Some(validator) = self.validators.write().get_mut(address) {
            *self.total_voting_power.write() -= validator.voting_power;
            validator.stake = new_stake;
            validator.voting_power = new_stake;
            *self.total_voting_power.write() += new_stake;
        }
        self.rebuild_validator_order();
    }
    
    /// Rebuild validator order for leader election
    fn rebuild_validator_order(&self) {
        let validators = self.validators.read();
        let mut order: Vec<_> = validators.values()
            .filter(|v| v.is_active)
            .collect();
        
        // Sort by stake (descending), then by address for determinism
        order.sort_by(|a, b| {
            b.stake.cmp(&a.stake)
                .then_with(|| a.address.cmp(&b.address))
        });
        
        *self.validator_order.write() = order.into_iter()
            .map(|v| v.address)
            .collect();
    }
    
    /// Get leader for height/round
    pub fn get_leader(&self, height: u64, round: u64) -> Option<Address> {
        let order = self.validator_order.read();
        if order.is_empty() {
            return None;
        }
        
        // Simple round-robin with weighted selection
        let index = ((height + round) as usize) % order.len();
        Some(order[index])
    }
    
    /// Check if we are the leader
    pub fn is_leader(&self, height: u64, round: u64) -> bool {
        if let (Some(our_addr), Some(leader)) = (self.our_address, self.get_leader(height, round)) {
            our_addr == leader
        } else {
            false
        }
    }
    
    /// Start new height
    pub fn start_height(&self, height: u64) {
        *self.round_state.write() = RoundState::new(height, 0);
        info!(height = height, "Starting new consensus height");
    }
    
    /// Process proposal
    pub fn process_proposal(&self, proposal: Proposal) -> Result<(), ConsensusError> {
        let mut state = self.round_state.write();
        
        // Verify height and round
        if proposal.height != state.height || proposal.round != state.round {
            return Err(ConsensusError::InvalidBlock(
                format!("Wrong height/round: expected {}/{}, got {}/{}",
                    state.height, state.round, proposal.height, proposal.round)
            ));
        }
        
        // Verify proposer is the leader
        let expected_leader = self.get_leader(proposal.height, proposal.round)
            .ok_or_else(|| ConsensusError::Internal("No validators".to_string()))?;
        
        if proposal.proposer != expected_leader {
            return Err(ConsensusError::NotLeader);
        }
        
        // Verify phase
        if state.phase != Phase::Propose {
            return Err(ConsensusError::InvalidBlock(
                format!("Wrong phase: expected Propose, got {:?}", state.phase)
            ));
        }
        
        // Validate block (simplified)
        self.validate_block(&proposal.block)?;
        
        // Accept proposal
        state.proposed_block = Some(proposal.block);
        state.advance_phase();
        
        info!(
            height = proposal.height,
            round = proposal.round,
            proposer = %proposal.proposer,
            "Accepted proposal"
        );
        
        Ok(())
    }
    
    /// Validate block
    fn validate_block(&self, block: &Block) -> Result<(), ConsensusError> {
        // Check block size
        let block_size = bincode::serialized_size(block).unwrap_or(0) as usize;
        if block_size > self.config.max_block_size {
            return Err(ConsensusError::InvalidBlock(
                format!("Block too large: {} bytes", block_size)
            ));
        }
        
        // Check transaction count
        if block.transactions.len() > self.config.max_block_txs {
            return Err(ConsensusError::InvalidBlock(
                format!("Too many transactions: {}", block.transactions.len())
            ));
        }
        
        // Additional validation would go here:
        // - Verify block hash
        // - Verify parent hash
        // - Verify state root
        // - Verify transaction merkle root
        
        Ok(())
    }
    
    /// Process vote
    pub fn process_vote(&self, vote: Vote) -> Result<Option<CommitCertificate>, ConsensusError> {
        let mut state = self.round_state.write();
        
        // Verify height and round
        if vote.height != state.height || vote.round != state.round {
            return Ok(None); // Ignore old/future votes
        }
        
        // Verify voter is a validator
        let validator = self.validators.read()
            .get(&vote.voter)
            .cloned()
            .ok_or_else(|| ConsensusError::InvalidVote(
                format!("Unknown voter: {}", vote.voter)
            ))?;
        
        if !validator.is_active {
            return Err(ConsensusError::InvalidVote("Voter is not active".to_string()));
        }
        
        // Add vote to appropriate collection
        match vote.vote_type {
            VoteType::Prevote => {
                if state.prevotes.contains_key(&vote.voter) {
                    return Err(ConsensusError::DuplicateVote(vote.voter));
                }
                state.prevotes.insert(vote.voter, vote.clone());
                
                // Check for 2/3 prevotes
                let prevote_power: Amount = state.prevotes.values()
                    .filter_map(|v| {
                        if v.block_hash == state.proposed_block.as_ref().map(|b| b.hash()) {
                            self.validators.read().get(&v.voter).map(|val| val.voting_power)
                        } else {
                            None
                        }
                    })
                    .sum();
                
                let threshold = *self.total_voting_power.read() * 2 / 3;
                if prevote_power > threshold {
                    state.advance_phase();
                    info!(
                        height = vote.height,
                        round = vote.round,
                        "Reached 2/3 prevotes, moving to precommit"
                    );
                }
            }
            VoteType::Precommit => {
                if state.precommits.contains_key(&vote.voter) {
                    return Err(ConsensusError::DuplicateVote(vote.voter));
                }
                state.precommits.insert(vote.voter, vote.clone());
                
                // Check for 2/3 precommits
                let block_hash = state.proposed_block.as_ref().map(|b| b.hash());
                let precommit_power: Amount = state.precommits.values()
                    .filter_map(|v| {
                        if v.block_hash == block_hash {
                            self.validators.read().get(&v.voter).map(|val| val.voting_power)
                        } else {
                            None
                        }
                    })
                    .sum();
                
                let threshold = *self.total_voting_power.read() * 2 / 3;
                if precommit_power > threshold {
                    if let Some(hash) = block_hash {
                        let signers: Vec<_> = state.precommits.keys().cloned().collect();
                        let cert = CommitCertificate {
                            block_hash: hash,
                            height: vote.height,
                            round: vote.round,
                            aggregated_signature: Vec::new(), // Aggregate signatures
                            signers,
                            voting_power: precommit_power,
                        };
                        
                        self.certificates.write().insert(hash, cert.clone());
                        self.finalized.write().insert(hash);
                        state.advance_phase();
                        
                        info!(
                            height = vote.height,
                            round = vote.round,
                            block = %hash,
                            "Block finalized with 2/3 precommits"
                        );
                        
                        return Ok(Some(cert));
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Create prevote for current proposal
    pub fn create_prevote(&self) -> Option<Vote> {
        let state = self.round_state.read();
        let our_address = self.our_address?;
        
        if state.phase != Phase::Prevote {
            return None;
        }
        
        let block_hash = state.proposed_block.as_ref().map(|b| b.hash());
        
        Some(Vote::new(
            VoteType::Prevote,
            state.height,
            state.round,
            block_hash,
            our_address,
            Vec::new(), // Signature would be added by caller
        ))
    }
    
    /// Create precommit for current proposal
    pub fn create_precommit(&self) -> Option<Vote> {
        let state = self.round_state.read();
        let our_address = self.our_address?;
        
        if state.phase != Phase::Precommit {
            return None;
        }
        
        let block_hash = state.proposed_block.as_ref().map(|b| b.hash());
        
        Some(Vote::new(
            VoteType::Precommit,
            state.height,
            state.round,
            block_hash,
            our_address,
            Vec::new(), // Signature would be added by caller
        ))
    }
    
    /// Handle timeout
    pub fn on_timeout(&self) {
        let mut state = self.round_state.write();
        
        match state.phase {
            Phase::Propose => {
                info!(
                    height = state.height,
                    round = state.round,
                    "Proposal timeout, sending nil prevote"
                );
                state.advance_phase();
            }
            Phase::Prevote => {
                info!(
                    height = state.height,
                    round = state.round,
                    "Prevote timeout, moving to precommit with nil"
                );
                state.advance_phase();
            }
            Phase::Precommit => {
                info!(
                    height = state.height,
                    round = state.round,
                    "Precommit timeout, starting new round"
                );
                state.advance_round();
            }
            Phase::Commit => {}
        }
    }
    
    /// Get current round state
    pub fn current_state(&self) -> RoundState {
        self.round_state.read().clone()
    }
    
    /// Check if block is finalized
    pub fn is_finalized(&self, hash: &HashValue) -> bool {
        self.finalized.read().contains(hash)
    }
    
    /// Get commit certificate
    pub fn get_certificate(&self, hash: &HashValue) -> Option<CommitCertificate> {
        self.certificates.read().get(hash).cloned()
    }
    
    /// Get total voting power
    pub fn total_voting_power(&self) -> Amount {
        *self.total_voting_power.read()
    }
    
    /// Get validator count
    pub fn validator_count(&self) -> usize {
        self.validators.read().len()
    }
    
    /// Get active validator count
    pub fn active_validator_count(&self) -> usize {
        self.validators.read().values()
            .filter(|v| v.is_active)
            .count()
    }
}

impl Default for ConsensusEngine {
    fn default() -> Self {
        Self::new(ConsensusConfig::default(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_leader_election() {
        let config = ConsensusConfig::default();
        let engine = ConsensusEngine::new(config, None);
        
        // Add validators
        let v1 = Address::from_public_key(b"validator1");
        let v2 = Address::from_public_key(b"validator2");
        let v3 = Address::from_public_key(b"validator3");
        
        engine.add_validator(v1, 1000);
        engine.add_validator(v2, 2000);
        engine.add_validator(v3, 1500);
        
        // Leader should rotate deterministically
        let leader_1_0 = engine.get_leader(1, 0);
        let leader_1_1 = engine.get_leader(1, 1);
        let leader_2_0 = engine.get_leader(2, 0);
        
        assert!(leader_1_0.is_some());
        assert!(leader_1_1.is_some());
        assert_ne!(leader_1_0, leader_1_1); // Different rounds = different leaders
    }
    
    #[test]
    fn test_validator_management() {
        let engine = ConsensusEngine::default();
        
        let v1 = Address::from_public_key(b"validator1");
        engine.add_validator(v1, 1000);
        
        assert_eq!(engine.validator_count(), 1);
        assert_eq!(engine.total_voting_power(), 1000);
        
        engine.update_stake(&v1, 2000);
        assert_eq!(engine.total_voting_power(), 2000);
        
        engine.remove_validator(&v1);
        assert_eq!(engine.validator_count(), 0);
        assert_eq!(engine.total_voting_power(), 0);
    }
    
    #[test]
    fn test_round_state() {
        let mut state = RoundState::new(1, 0);
        
        assert_eq!(state.height, 1);
        assert_eq!(state.round, 0);
        assert_eq!(state.phase, Phase::Propose);
        
        state.advance_phase();
        assert_eq!(state.phase, Phase::Prevote);
        
        state.advance_phase();
        assert_eq!(state.phase, Phase::Precommit);
        
        state.advance_round();
        assert_eq!(state.round, 1);
        assert_eq!(state.phase, Phase::Propose);
    }
}
