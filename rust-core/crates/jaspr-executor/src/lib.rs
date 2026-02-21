//! Transaction execution engine for JasprChain
//!
//! Handles:
//! - Transaction validation and execution
//! - State transitions
//! - Gas metering
//! - Receipt generation

pub mod executor;
pub mod gas;
pub mod vm_adapter;

pub use executor::{TransactionExecutor, ExecutionResult, ExecutorConfig};
pub use gas::{GasMeter, GasConfig};
pub use vm_adapter::{VmAdapter, VmContext, VmOutput};
