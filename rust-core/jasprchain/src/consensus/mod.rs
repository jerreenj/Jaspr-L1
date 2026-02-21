//! Consensus module - BLS signatures, VRF, committee finality

mod block;
mod validator;
mod proposer;
mod finality;

pub use block::{Block, BlockHeader};
pub use validator::{Validator, ValidatorSet, ValidatorStats};
pub use proposer::ProposerSelection;
pub use finality::FinalityEngine;
