//! Move VM integration for JasprChain

pub mod adapter;
pub mod module_cache;
pub mod stdlib;
pub mod types;
pub mod executor;
pub mod verifier;

pub use adapter::MoveVmAdapter;
pub use module_cache::ModuleCache;
pub use stdlib::StdlibModules;
pub use executor::{MoveVmExecutor, ExecutionContext, MoveExecutionResult};
pub use verifier::{BytecodeVerifier, VerificationResult, AbiGenerator};
