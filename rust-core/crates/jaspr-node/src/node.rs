//! JasprChain Node implementation

use crate::config::NodeConfig;
use crate::mempool::Mempool;
use jaspr_types::{Block, SignedTransaction, HashValue, BlockHeight, ValidatorInfo};
use jaspr_crypto::{KeyPair, sign_transaction};
use jaspr_state::{Database, BlockStore, AccountStore, StateTree};
use jaspr_consensus::{ConsensusEngine, ConsensusConfig, ValidatorSet, MIN_VALIDATOR_STAKE};
use jaspr_executor::{TransactionExecutor, ExecutorConfig};
use jaspr_move_vm::MoveVmAdapter;
use jaspr_network::{NetworkService, NetworkConfig, PeerId, NetworkEvent};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, warn, error, debug};

/// Node state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeState {
    /// Starting up
    Starting,
    /// Syncing with network
    Syncing,
    /// Fully synced and operational
    Running,
    /// Shutting down
    Stopping,
    /// Stopped
    Stopped,
}

/// JasprChain node
pub struct JasprNode {
    config: NodeConfig,
    state: RwLock<NodeState>,
    
    // Storage
    block_store: Arc<BlockStore>,
    account_store: Arc<AccountStore>,
    state_tree: Arc<StateTree>,
    
    // Consensus
    consensus: Arc<ConsensusEngine>,
    
    // Execution
    executor: Arc<TransactionExecutor>,
    move_vm: Arc<MoveVmAdapter>,
    
    // Network
    network: Arc<NetworkService>,
    
    // Mempool
    mempool: Arc<Mempool>,
    
    // Validator key (if validator)
    validator_key: Option<KeyPair>,
}

impl JasprNode {
    /// Create new node
    pub fn new(config: NodeConfig) -> Result<Self, String> {
        info!(chain_id = config.chain_id, "Creating JasprChain node");
        
        // Initialize database
        let mut db_config = config.database.clone();
        db_config.path = config.data_dir.join("db").to_string_lossy().to_string();
        
        let database = 
            Database::open(&db_config)
                .map_err(|e| format!("Failed to open database: {}", e))?;
        
        // Initialize stores (database is cloned internally)
        let db_for_blocks = database.clone();
        let db_for_accounts = database.clone();
        let block_store = Arc::new(BlockStore::new(db_for_blocks));
        let account_store = Arc::new(AccountStore::new(db_for_accounts));
        let state_tree = Arc::new(StateTree::new());
        
        // Initialize consensus
        let consensus = Arc::new(ConsensusEngine::new(config.consensus.clone()));
        
        // Initialize executor
        let executor_config = ExecutorConfig {
            chain_id: config.chain_id,
            ..Default::default()
        };
        let executor = Arc::new(TransactionExecutor::new(
            executor_config,
            account_store.clone(),
            state_tree.clone(),
        ));
        
        // Initialize Move VM
        let move_vm = Arc::new(MoveVmAdapter::new());
        
        // Initialize network
        let local_key = KeyPair::generate();
        let local_id = PeerId::from_public_key(local_key.public_key());
        let network = Arc::new(NetworkService::new(config.network.clone(), local_id));
        
        // Initialize mempool
        let mempool = Arc::new(Mempool::default());
        
        Ok(Self {
            config,
            state: RwLock::new(NodeState::Starting),
            database,
            block_store,
            account_store,
            state_tree,
            consensus,
            executor,
            move_vm,
            network,
            mempool,
            validator_key: None,
        })
    }
    
    /// Set validator key (makes this a validator node)
    pub fn set_validator_key(&mut self, key: KeyPair) {
        info!(address = %key.address(), "Setting validator key");
        self.validator_key = Some(key);
    }
    
    /// Initialize genesis state
    pub async fn init_genesis(&self) -> Result<(), String> {
        info!("Initializing genesis state");
        
        // Create genesis block
        let genesis = self.block_store.init_genesis(self.config.chain_id)
            .map_err(|e| format!("Failed to create genesis: {}", e))?;
        
        info!(hash = %genesis.hash(), "Genesis block created");
        
        // Initialize genesis accounts (community pool, foundation, etc.)
        self.init_genesis_accounts().await?;
        
        // Initialize genesis validators
        self.init_genesis_validators().await?;
        
        Ok(())
    }
    
    /// Initialize genesis accounts
    async fn init_genesis_accounts(&self) -> Result<(), String> {
        use jaspr_types::Account;
        
        // Community pool (20% of 1B JASPR)
        let community_pool = Address::from_public_key(b"jaspr_community_pool");
        let community_account = Account::new(
            community_pool,
            200_000_000_000_000_000, // 200M JASPR in base units
        );
        self.account_store.put_account(&community_account)
            .map_err(|e| e.to_string())?;
        
        info!(address = %community_pool, balance = 200_000_000, "Created community pool");
        
        Ok(())
    }
    
    /// Initialize genesis validators
    async fn init_genesis_validators(&self) -> Result<(), String> {
        // In production, this would read from genesis.json
        // For now, create a few genesis validators
        
        for i in 0..20 {
            let validator = ValidatorInfo::new(
                jaspr_types::Address::from_public_key(format!("genesis_validator_{}", i).as_bytes()),
                format!("Genesis Validator {}", i + 1),
                MIN_VALIDATOR_STAKE * 10, // 320K JASPR each
            );
            
            self.consensus.validator_set().register_validator(validator)
                .map_err(|e| format!("Failed to register validator: {}", e))?;
        }
        
        info!(count = 20, "Registered genesis validators");
        
        Ok(())
    }
    
    /// Start the node
    pub async fn start(&self) -> Result<(), String> {
        info!("Starting JasprChain node");
        *self.state.write() = NodeState::Starting;
        
        // Initialize genesis if needed
        if self.block_store.get_latest_height().map_err(|e| e.to_string())?.is_none() {
            self.init_genesis().await?;
        }
        
        // Start network
        self.network.start().await?;
        
        // Set running state
        *self.state.write() = NodeState::Running;
        
        info!("JasprChain node started");
        
        Ok(())
    }
    
    /// Run the node's main loop
    pub async fn run(&self) {
        let mut block_interval = interval(Duration::from_millis(
            self.config.consensus.block_time_ms
        ));
        
        loop {
            if *self.state.read() != NodeState::Running {
                break;
            }
            
            block_interval.tick().await;
            
            // Try to produce block if we're a validator
            if self.validator_key.is_some() {
                if let Err(e) = self.try_produce_block().await {
                    warn!(error = %e, "Failed to produce block");
                }
            }
            
            // Clean up mempool
            self.mempool.remove_expired();
        }
    }
    
    /// Try to produce a block
    async fn try_produce_block(&self) -> Result<(), String> {
        let height = self.consensus.current_height() + 1;
        
        // Check if we're the proposer
        if !self.consensus.is_proposer(height) {
            return Ok(());
        }
        
        let previous = self.block_store.get_latest_block()
            .map_err(|e| e.to_string())?
            .ok_or("No previous block")?;
        
        // Get transactions from mempool
        let txs = self.mempool.get_top_transactions(
            self.config.consensus.max_txs_per_block
        );
        
        // Add to block producer
        for tx in &txs {
            self.consensus.block_producer().add_transaction(tx.clone());
        }
        
        // Produce block
        let block = self.consensus.try_produce_block(
            previous.hash(),
            self.state_tree.root_hash(),
        ).ok_or("Not our turn to produce")?;
        
        // Execute block
        let (receipts, state_root, gas_used) = self.executor.execute_block(&block).await;
        
        // Update block with execution results
        let mut final_block = block;
        final_block.header.state_root = state_root;
        final_block.header.gas_used = gas_used;
        
        // Store block
        self.block_store.put_block(&final_block)
            .map_err(|e| e.to_string())?;
        
        // Store receipts
        for receipt in &receipts {
            self.block_store.put_receipt(receipt)
                .map_err(|e| e.to_string())?;
        }
        
        // Remove committed transactions from mempool
        self.mempool.remove_committed(&txs);
        
        // Update consensus height
        self.consensus.set_height(final_block.height());
        
        // Broadcast block
        self.network.broadcast_block(&final_block);
        
        info!(
            height = final_block.height(),
            txs = final_block.body.tx_count(),
            gas = gas_used,
            "Produced block"
        );
        
        Ok(())
    }
    
    /// Stop the node
    pub async fn stop(&self) {
        info!("Stopping JasprChain node");
        *self.state.write() = NodeState::Stopping;
        
        self.network.stop().await;
        
        *self.state.write() = NodeState::Stopped;
        info!("JasprChain node stopped");
    }
    
    /// Submit transaction
    pub fn submit_transaction(&self, tx: SignedTransaction) -> Result<HashValue, String> {
        // Validate transaction
        self.executor.validate_transaction(&tx)?;
        
        // Add to mempool
        let hash = tx.hash();
        self.mempool.add_transaction(tx.clone())?;
        
        // Broadcast to network
        self.network.broadcast_transaction(&tx);
        
        debug!(tx = %hash, "Transaction submitted");
        
        Ok(hash)
    }
    
    /// Get block by height
    pub fn get_block(&self, height: BlockHeight) -> Option<Block> {
        self.block_store.get_block_by_height(height).ok().flatten()
    }
    
    /// Get latest block
    pub fn get_latest_block(&self) -> Option<Block> {
        self.block_store.get_latest_block().ok().flatten()
    }
    
    /// Get account balance
    pub fn get_balance(&self, address: &jaspr_types::Address) -> u128 {
        self.account_store.get_balance(address).unwrap_or(0)
    }
    
    /// Get chain height
    pub fn chain_height(&self) -> BlockHeight {
        self.block_store.get_latest_height().ok().flatten().unwrap_or(0)
    }
    
    /// Get node state
    pub fn node_state(&self) -> NodeState {
        *self.state.read()
    }
    
    /// Get node stats
    pub fn stats(&self) -> NodeStats {
        NodeStats {
            state: self.node_state(),
            chain_height: self.chain_height(),
            pending_txs: self.mempool.size(),
            connected_peers: self.network.peer_manager().connected_count(),
            validator_count: self.consensus.validator_set().validator_count(),
            total_stake: self.consensus.validator_set().total_stake(),
        }
    }
}

use jaspr_types::Address;

/// Node statistics
#[derive(Clone, Debug)]
pub struct NodeStats {
    pub state: NodeState,
    pub chain_height: BlockHeight,
    pub pending_txs: usize,
    pub connected_peers: usize,
    pub validator_count: usize,
    pub total_stake: u128,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_node_creation() {
        let config = NodeConfig::devnet()
            .with_data_dir(PathBuf::from("/tmp/jaspr_test"));
        
        let node = JasprNode::new(config);
        assert!(node.is_ok());
    }
}
