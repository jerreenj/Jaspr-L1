//! Error types for JasprChain

use thiserror::Error;

/// Main error type for JasprChain
#[derive(Error, Debug)]
pub enum JasprError {
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Invalid hash: {0}")]
    InvalidHash(String),

    #[error("Insufficient balance: need {needed}, have {available}")]
    InsufficientBalance { needed: u128, available: u128 },

    #[error("Invalid nonce: expected {expected}, got {got}")]
    InvalidNonce { expected: u64, got: u64 },

    #[error("Transaction expired")]
    TransactionExpired,

    #[error("Gas limit exceeded")]
    GasLimitExceeded,

    #[error("Block not found: {0}")]
    BlockNotFound(u64),

    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Invalid block: {0}")]
    InvalidBlock(String),

    #[error("Consensus error: {0}")]
    ConsensusError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Move VM error: {0}")]
    MoveVmError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Validator not found: {0}")]
    ValidatorNotFound(String),

    #[error("Account frozen")]
    AccountFrozen,

    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    #[error("Invalid chain ID: expected {expected}, got {got}")]
    InvalidChainId { expected: u64, got: u64 },

    #[error("Duplicate transaction")]
    DuplicateTransaction,

    #[error("Mempool full")]
    MempoolFull,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for JasprError {
    fn from(e: std::io::Error) -> Self {
        JasprError::Internal(e.to_string())
    }
}

impl From<hex::FromHexError> for JasprError {
    fn from(e: hex::FromHexError) -> Self {
        JasprError::InvalidHash(e.to_string())
    }
}

/// Result type alias
pub type JasprResult<T> = Result<T, JasprError>;
