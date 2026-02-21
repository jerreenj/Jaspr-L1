use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use jasprchain::{JasprChain, ChainConfig};

#[derive(Parser)]
#[command(name = "jasprchain")]
#[command(about = "JasprChain L1 Node", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    config: String,
    
    #[arg(short, long, default_value = "8545")]
    rpc_port: u16,
    
    #[arg(short, long, default_value = "30303")]
    p2p_port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    let cli = Cli::parse();
    
    info!("Starting JasprChain node...");
    info!("RPC Port: {}", cli.rpc_port);
    info!("P2P Port: {}", cli.p2p_port);
    
    // Initialize chain
    let config = ChainConfig::default();
    let chain = JasprChain::new(config).await;
    
    info!("Chain initialized at height: {}", chain.height().await);
    info!("Genesis block created");
    
    // Start RPC server
    // start_rpc_server(chain.clone(), cli.rpc_port).await?;
    
    // Start P2P networking
    // start_p2p(chain.clone(), cli.p2p_port).await?;
    
    // Start block production loop
    // block_producer(chain).await?;
    
    info!("JasprChain node running");
    
    // Keep running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");
    
    Ok(())
}
