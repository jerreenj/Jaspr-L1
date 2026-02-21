//! Move VM integration for JasprChain
//!
//! This crate provides the Move VM adapter implementation
//! for executing Move smart contracts on JasprChain.

pub mod adapter;
pub mod module_cache;
pub mod stdlib;
pub mod types;
pub mod executor;

pub use adapter::MoveVmAdapter;
pub use module_cache::ModuleCache;
pub use stdlib::StdlibModules;
pub use executor::{MoveVmExecutor, ExecutionContext, MoveExecutionResult};
