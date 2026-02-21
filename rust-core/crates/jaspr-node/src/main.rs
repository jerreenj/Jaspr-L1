//! JasprChain Node Binary

use jaspr_node::{JasprNode, NodeConfig};
use jaspr_crypto::KeyPair;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "jasprchain")]
#[command(about = "JasprChain - High-performance L1 blockchain with Move VM")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the node
    Run {
        /// Data directory
        #[arg(long, default_value = "./data")]
        data_dir: PathBuf,
        
        /// Chain ID (1 = mainnet, 2 = testnet, 31337 = devnet)
        #[arg(long, default_value_t = 1)]
        chain_id: u64,
        
        /// RPC listen address
        #[arg(long, default_value = "0.0.0.0:8545")]
        rpc_addr: String,
        
        /// P2P listen address
        #[arg(long, default_value = "0.0.0.0:30303")]
        p2p_addr: String,
        
        /// Run as validator
        #[arg(long)]
        validator: bool,
        
        /// Validator key file
        #[arg(long)]
        validator_key: Option<PathBuf>,
        
        /// Bootstrap peers (comma-separated)
        #[arg(long)]
        bootstrap: Option<String>,
        
        /// Log level
        #[arg(long, default_value = "info")]
        log_level: String,
    },
    
    /// Generate a new keypair
    Keygen {
        /// Output file for private key
        #[arg(long)]
        output: Option<PathBuf>,
    },
    
    /// Initialize a new chain
    Init {
        /// Data directory
        #[arg(long, default_value = "./data")]
        data_dir: PathBuf,
        
        /// Chain ID
        #[arg(long, default_value_t = 1)]
        chain_id: u64,
    },
    
    /// Show node version and info
    Version,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Run {
            data_dir,
            chain_id,
            rpc_addr,
            p2p_addr,
            validator,
            validator_key,
            bootstrap,
            log_level,
        } => {
            // Setup logging
            let level = match log_level.to_lowercase().as_str() {
                "trace" => Level::TRACE,
                "debug" => Level::DEBUG,
                "info" => Level::INFO,
                "warn" => Level::WARN,
                "error" => Level::ERROR,
                _ => Level::INFO,
            };
            
            let subscriber = FmtSubscriber::builder()
                .with_max_level(level)
                .with_target(false)
                .init();
            
            info!("JasprChain Node v0.1.0");
            info!(chain_id = chain_id, data_dir = %data_dir.display(), "Starting node");
            
            // Parse addresses
            let rpc_addr: std::net::SocketAddr = rpc_addr.parse()?;
            let p2p_addr: std::net::SocketAddr = p2p_addr.parse()?;
            
            // Parse bootstrap peers
            let bootstrap_peers: Vec<std::net::SocketAddr> = bootstrap
                .map(|s| {
                    s.split(',')
                        .filter_map(|addr| addr.trim().parse().ok())
                        .collect()
                })
                .unwrap_or_default();
            
            // Create config
            let config = NodeConfig {
                data_dir: data_dir.clone(),
                chain_id,
                is_validator: validator,
                validator_key_path: validator_key.clone(),
                log_level: log_level.clone(),
                ..Default::default()
            }
            .with_data_dir(data_dir)
            .with_rpc_addr(rpc_addr)
            .with_network_addr(p2p_addr)
            .with_bootstrap_peers(bootstrap_peers);
            
            // Create node
            let mut node = JasprNode::new(config)?;
            
            // Load validator key if specified
            if validator {
                if let Some(key_path) = validator_key {
                    let key_hex = std::fs::read_to_string(&key_path)?;
                    let key = jaspr_crypto::PrivateKey::from_hex(key_hex.trim())?;
                    let keypair = KeyPair::from_private_key(key);
                    node.set_validator_key(keypair);
                    info!("Loaded validator key");
                } else {
                    // Generate ephemeral key for testing
                    let keypair = KeyPair::generate();
                    info!(address = %keypair.address(), "Generated ephemeral validator key");
                    node.set_validator_key(keypair);
                }
            }
            
            // Start node
            node.start().await?;
            
            // Run node
            info!("Node is running. Press Ctrl+C to stop.");
            
            // Wait for shutdown signal
            tokio::signal::ctrl_c().await?;
            
            // Stop node
            node.stop().await;
            
            info!("Node stopped");
        }
        
        Commands::Keygen { output } => {
            let keypair = KeyPair::generate();
            
            let private_hex = keypair.export_private_hex();
            let public_hex = keypair.public_key().to_hex();
            let address = keypair.address();
            
            println!("Generated new keypair:");
            println!("  Address: {}", address.to_hex());
            println!("  Public Key: {}", public_hex);
            
            if let Some(path) = output {
                std::fs::write(&path, &private_hex)?;
                println!("  Private key saved to: {}", path.display());
            } else {
                println!("  Private Key: {}", private_hex);
            }
        }
        
        Commands::Init { data_dir, chain_id } => {
            println!("Initializing JasprChain data directory...");
            
            // Create directories
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(data_dir.join("db"))?;
            
            println!("Created data directory: {}", data_dir.display());
            println!("Chain ID: {}", chain_id);
            println!("Ready to start node with: jasprchain run --data-dir {} --chain-id {}", 
                data_dir.display(), chain_id);
        }
        
        Commands::Version => {
            println!("JasprChain Node");
            println!("Version: 0.1.0");
            println!("Rust Edition: 2021");
            println!("Features: Move VM, PoS Consensus, P2P Networking");
        }
    }
    
    Ok(())
}
