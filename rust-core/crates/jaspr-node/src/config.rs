//! Node configuration

use jaspr_consensus::ConsensusConfig;
use jaspr_network::NetworkConfig;
use jaspr_executor::ExecutorConfig;
use jaspr_state::DatabaseConfig;
use std::path::PathBuf;
use std::net::SocketAddr;

/// Node configuration
#[derive(Clone, Debug)]
pub struct NodeConfig {
    /// Data directory
    pub data_dir: PathBuf,
    /// Chain ID
    pub chain_id: u64,
    /// Network configuration
    pub network: NetworkConfig,
    /// Consensus configuration
    pub consensus: ConsensusConfig,
    /// Executor configuration
    pub executor: ExecutorConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// RPC listen address
    pub rpc_addr: SocketAddr,
    /// Is validator node
    pub is_validator: bool,
    /// Validator key path (if validator)
    pub validator_key_path: Option<PathBuf>,
    /// Log level
    pub log_level: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            chain_id: 1,
            network: NetworkConfig::default(),
            consensus: ConsensusConfig::default(),
            executor: ExecutorConfig::default(),
            database: DatabaseConfig::default(),
            rpc_addr: "0.0.0.0:8545".parse().unwrap(),
            is_validator: false,
            validator_key_path: None,
            log_level: "info".to_string(),
        }
    }
}

impl NodeConfig {
    /// Create testnet configuration
    pub fn testnet() -> Self {
        Self {
            chain_id: 2,
            ..Default::default()
        }
    }
    
    /// Create devnet configuration
    pub fn devnet() -> Self {
        Self {
            chain_id: 31337,
            consensus: ConsensusConfig {
                block_time_ms: 1000, // 1 second blocks
                ..Default::default()
            },
            ..Default::default()
        }
    }
    
    /// With data directory
    pub fn with_data_dir(mut self, path: PathBuf) -> Self {
        self.data_dir = path.clone();
        self.database.path = path.join("db").to_string_lossy().to_string();
        self
    }
    
    /// With validator key
    pub fn with_validator_key(mut self, key_path: PathBuf) -> Self {
        self.is_validator = true;
        self.validator_key_path = Some(key_path);
        self
    }
    
    /// With RPC address
    pub fn with_rpc_addr(mut self, addr: SocketAddr) -> Self {
        self.rpc_addr = addr;
        self
    }
    
    /// With network address
    pub fn with_network_addr(mut self, addr: SocketAddr) -> Self {
        self.network.listen_addr = addr;
        self
    }
    
    /// With bootstrap peers
    pub fn with_bootstrap_peers(mut self, peers: Vec<SocketAddr>) -> Self {
        self.network.bootstrap_peers = peers;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = NodeConfig::default();
        assert_eq!(config.chain_id, 1);
    }
    
    #[test]
    fn test_testnet_config() {
        let config = NodeConfig::testnet();
        assert_eq!(config.chain_id, 2);
    }
    
    #[test]
    fn test_devnet_config() {
        let config = NodeConfig::devnet();
        assert_eq!(config.chain_id, 31337);
        assert_eq!(config.consensus.block_time_ms, 1000);
    }
}
