//! JasprChain Core Types
//! 
//! This crate defines all fundamental types used throughout JasprChain:
//! - Addresses, Hashes, Signatures
//! - Blocks, Transactions, Receipts
//! - Account and State types

pub mod address;
pub mod hash;
pub mod block;
pub mod transaction;
pub mod account;
pub mod receipt;
pub mod error;

pub use address::{Address, AccountAddress};
pub use hash::{Hash, HashValue, Hasher};
pub use block::{Block, BlockHeader, BlockBody};
pub use transaction::{Transaction, SignedTransaction, TransactionType, TransactionPayload};
pub use account::{Account, AccountState, Balance, ValidatorInfo};
pub use receipt::{TransactionReceipt, ExecutionStatus};
pub use error::JasprError;

/// Chain ID for network identification
pub type ChainId = u64;

/// Block height
pub type BlockHeight = u64;

/// Timestamp in milliseconds since Unix epoch
pub type Timestamp = u64;

/// Gas unit
pub type Gas = u64;

/// Token amount in base units (1 JASPR = 10^9 base units)
pub type Amount = u128;

/// Nonce for transaction ordering
pub type Nonce = u64;

/// Version for state versioning
pub type Version = u64;
