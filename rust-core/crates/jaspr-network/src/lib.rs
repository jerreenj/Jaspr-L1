//! P2P networking for JasprChain
//!
//! Provides:
//! - Peer discovery and management
//! - Block and transaction propagation
//! - Gossip protocol
//! - Network message types

pub mod peer;
pub mod message;
pub mod gossip;
pub mod service;
pub mod libp2p_network;
pub mod sync;

pub use peer::{PeerId, PeerInfo, PeerManager};
pub use message::{NetworkMessage, MessageType};
pub use gossip::GossipProtocol;
pub use service::{NetworkService, NetworkConfig, NetworkEvent};
pub use libp2p_network::{Libp2pNetwork, Libp2pConfig, Libp2pEvent};
pub use sync::{BlockSyncService, SyncConfig, SyncState};
