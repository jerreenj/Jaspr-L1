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

pub use peer::{PeerId, PeerInfo, PeerManager};
pub use message::{NetworkMessage, MessageType};
pub use gossip::GossipProtocol;
pub use service::{NetworkService, NetworkConfig, NetworkEvent};
