//! Consensus engine for JasprChain
//!
//! Implements Proof-of-Stake consensus with:
//! - Validator set management
//! - Block proposal selection
//! - Block validation and attestation
//! - Finality determination

pub mod validator_set;
pub mod block_producer;
pub mod attestation;
pub mod engine;

pub use validator_set::{ValidatorSet, ValidatorStatus};
pub use block_producer::BlockProducer;
pub use attestation::{Attestation, AttestationPool};
pub use engine::{ConsensusEngine, ConsensusConfig};
