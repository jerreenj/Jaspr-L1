//! JasprChain Node
//!
//! Main node implementation that orchestrates all components:
//! - Consensus engine
//! - Transaction execution
//! - State management
//! - P2P networking
//! - RPC server

pub mod node;
pub mod config;
pub mod rpc;
pub mod rpc_server;
pub mod mempool;
pub mod faucet;

pub use node::JasprNode;
pub use config::NodeConfig;
pub use mempool::Mempool;
pub use faucet::{FaucetService, FaucetConfig};
