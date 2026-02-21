//! JasprChain - High-performance Layer 1 Blockchain
//!
//! Core modules:
//! - consensus: BLS signatures, VRF proposer selection, committee finality
//! - execution: Parallel transaction processing with conflict detection
//! - state: Sparse Merkle Tree state management
//! - wallet: MPC + Account Abstraction
//! - sentinel: AI risk scoring hooks
//! - network: P2P networking, mempool
//! - crypto: Cryptographic primitives

pub mod consensus;
pub mod execution;
pub mod state;
pub mod wallet;
pub mod sentinel;
pub mod network;
pub mod crypto;

use std::sync::Arc;
use tokio::sync::RwLock;

pub use consensus::{Block, BlockHeader, Validator, ValidatorSet};
pub use execution::{Transaction, SignedTransaction, ParallelExecutor};
pub use state::{StateStore, SparseMerkleTree};
pub use wallet::{MPCWallet, AccountAbstraction};
pub use sentinel::AISentinel;
pub use network::Mempool;

/// Chain configuration
pub struct ChainConfig {
    pub chain_id: u64,
    pub block_time_ms: u64,
    pub target_tps: u64,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            chain_id: 1,
            block_time_ms: 2000,
            target_tps: 10000,
        }
    }
}

/// Main JasprChain instance
pub struct JasprChain {
    pub config: ChainConfig,
    pub state: Arc<RwLock<StateStore>>,
    pub validator_set: Arc<RwLock<ValidatorSet>>,
    pub mempool: Arc<RwLock<Mempool>>,
    pub sentinel: Arc<AISentinel>,
    blocks: Arc<RwLock<Vec<Block>>>,
}

impl JasprChain {
    pub async fn new(config: ChainConfig) -> Self {
        let state = Arc::new(RwLock::new(StateStore::new()));
        let validator_set = Arc::new(RwLock::new(ValidatorSet::new()));
        let sentinel = Arc::new(AISentinel::new());
        let mempool = Arc::new(RwLock::new(Mempool::new(sentinel.clone())));
        
        let mut chain = Self {
            config,
            state,
            validator_set,
            mempool,
            sentinel,
            blocks: Arc::new(RwLock::new(Vec::new())),
        };
        
        chain.initialize().await;
        chain
    }
    
    async fn initialize(&mut self) {
        // Create genesis block
        let genesis = Block::genesis();
        self.blocks.write().await.push(genesis);
        
        // Initialize validators (HyperLiquid style - 4 initial)
        let mut vs = self.validator_set.write().await;
        vs.add_validator("jaspr1validator1", 10_000_000_000_000, "Jaspr Labs");
        vs.add_validator("jaspr1validator2", 8_000_000_000_000, "Foundation");
        vs.add_validator("jaspr1validator3", 6_000_000_000_000, "Community");
        vs.add_validator("jaspr1validator4", 4_000_000_000_000, "Ecosystem");
    }
    
    pub async fn height(&self) -> usize {
        self.blocks.read().await.len() - 1
    }
    
    pub async fn latest_block(&self) -> Option<Block> {
        self.blocks.read().await.last().cloned()
    }
}
