//! Consensus engine for JasprChain
//!
//! Implements Proof-of-Stake consensus with:
//! - Validator set management
//! - Block proposal selection
//! - Block validation and attestation
//! - Finality determination
//! - BFT consensus protocol

pub mod validator_set;
pub mod block_producer;
pub mod attestation;
pub mod engine;
pub mod bft;

pub use validator_set::{ValidatorSet, ValidatorStatus, MIN_VALIDATOR_STAKE};
pub use block_producer::{BlockProducer, BlockProducerConfig};
pub use attestation::{Attestation, AttestationPool};
pub use engine::{ConsensusEngine, ConsensusConfig};
pub use bft::{RoundState, Vote, VoteType, Proposal, CommitCertificate, Phase};
