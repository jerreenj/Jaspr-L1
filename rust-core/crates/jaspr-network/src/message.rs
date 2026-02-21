//! Network message types

use jaspr_types::{Block, SignedTransaction, HashValue, BlockHeight};
use crate::peer::PeerId;
use borsh::{BorshSerialize, BorshDeserialize};
use serde::{Serialize, Deserialize};

/// Message types
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum MessageType {
    /// Handshake (initial connection)
    Handshake,
    /// Ping for keepalive
    Ping,
    /// Pong response
    Pong,
    /// Get blocks request
    GetBlocks,
    /// Blocks response
    Blocks,
    /// New block announcement
    NewBlock,
    /// Get transactions request
    GetTransactions,
    /// Transactions response
    Transactions,
    /// New transaction announcement
    NewTransaction,
    /// Get peers request
    GetPeers,
    /// Peers response
    Peers,
    /// Status update
    Status,
    /// Attestation
    Attestation,
}

/// Handshake message
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct HandshakeMessage {
    /// Protocol version
    pub protocol_version: u32,
    /// Chain ID
    pub chain_id: u64,
    /// Current block height
    pub height: BlockHeight,
    /// Genesis block hash
    pub genesis_hash: HashValue,
    /// Client version string
    pub client_version: String,
    /// Node's public key (for identity)
    pub public_key: Vec<u8>,
}

impl HandshakeMessage {
    pub fn new(chain_id: u64, height: BlockHeight, genesis_hash: HashValue) -> Self {
        Self {
            protocol_version: 1,
            chain_id,
            height,
            genesis_hash,
            client_version: "JasprChain/0.1.0".to_string(),
            public_key: Vec::new(),
        }
    }
}

/// Status message
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct StatusMessage {
    /// Current block height
    pub height: BlockHeight,
    /// Best block hash
    pub best_hash: HashValue,
    /// Total difficulty / voting power
    pub total_power: u64,
}

/// Get blocks request
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct GetBlocksRequest {
    /// Start height
    pub start: BlockHeight,
    /// End height
    pub end: BlockHeight,
}

/// Blocks response
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct BlocksResponse {
    pub blocks: Vec<Block>,
}

/// Get transactions request
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct GetTransactionsRequest {
    /// Transaction hashes to request
    pub hashes: Vec<HashValue>,
}

/// Transactions response
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct TransactionsResponse {
    pub transactions: Vec<SignedTransaction>,
}

/// Peer address info
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct PeerAddress {
    /// IP address (as string)
    pub ip: String,
    /// Port
    pub port: u16,
    /// Last seen timestamp
    pub last_seen: u64,
}

/// Peers response
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct PeersResponse {
    pub peers: Vec<PeerAddress>,
}

/// Network message wrapper
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct NetworkMessage {
    /// Message type
    pub msg_type: MessageType,
    /// Request ID (for matching responses)
    pub request_id: u64,
    /// Message payload (serialized)
    pub payload: Vec<u8>,
}

impl NetworkMessage {
    /// Create new message
    pub fn new(msg_type: MessageType, request_id: u64, payload: Vec<u8>) -> Self {
        Self {
            msg_type,
            request_id,
            payload,
        }
    }
    
    /// Create handshake message
    pub fn handshake(msg: HandshakeMessage) -> Self {
        Self::new(
            MessageType::Handshake,
            0,
            borsh::to_vec(&msg).unwrap_or_default(),
        )
    }
    
    /// Create ping message
    pub fn ping(request_id: u64) -> Self {
        Self::new(MessageType::Ping, request_id, Vec::new())
    }
    
    /// Create pong message
    pub fn pong(request_id: u64) -> Self {
        Self::new(MessageType::Pong, request_id, Vec::new())
    }
    
    /// Create new block message
    pub fn new_block(block: &Block) -> Self {
        Self::new(
            MessageType::NewBlock,
            0,
            borsh::to_vec(block).unwrap_or_default(),
        )
    }
    
    /// Create new transaction message
    pub fn new_transaction(tx: &SignedTransaction) -> Self {
        Self::new(
            MessageType::NewTransaction,
            0,
            borsh::to_vec(tx).unwrap_or_default(),
        )
    }
    
    /// Create status message
    pub fn status(msg: StatusMessage) -> Self {
        Self::new(
            MessageType::Status,
            0,
            borsh::to_vec(&msg).unwrap_or_default(),
        )
    }
    
    /// Create get blocks request
    pub fn get_blocks(request_id: u64, start: BlockHeight, end: BlockHeight) -> Self {
        let req = GetBlocksRequest { start, end };
        Self::new(
            MessageType::GetBlocks,
            request_id,
            borsh::to_vec(&req).unwrap_or_default(),
        )
    }
    
    /// Create blocks response
    pub fn blocks(request_id: u64, blocks: Vec<Block>) -> Self {
        let resp = BlocksResponse { blocks };
        Self::new(
            MessageType::Blocks,
            request_id,
            borsh::to_vec(&resp).unwrap_or_default(),
        )
    }
    
    /// Parse payload as specific type
    pub fn parse_payload<T: BorshDeserialize>(&self) -> Result<T, String> {
        borsh::from_slice(&self.payload)
            .map_err(|e| format!("Failed to parse payload: {}", e))
    }
    
    /// Serialize message
    pub fn serialize(&self) -> Vec<u8> {
        borsh::to_vec(self).unwrap_or_default()
    }
    
    /// Deserialize message
    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        borsh::from_slice(data)
            .map_err(|e| format!("Failed to deserialize message: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_roundtrip() {
        let msg = NetworkMessage::ping(42);
        let bytes = msg.serialize();
        let decoded = NetworkMessage::deserialize(&bytes).unwrap();
        
        assert_eq!(decoded.msg_type, MessageType::Ping);
        assert_eq!(decoded.request_id, 42);
    }
    
    #[test]
    fn test_handshake() {
        let handshake = HandshakeMessage::new(1, 100, HashValue::zero());
        let msg = NetworkMessage::handshake(handshake.clone());
        
        let parsed: HandshakeMessage = msg.parse_payload().unwrap();
        assert_eq!(parsed.chain_id, 1);
        assert_eq!(parsed.height, 100);
    }
    
    #[test]
    fn test_new_block_message() {
        let block = Block::genesis(1);
        let msg = NetworkMessage::new_block(&block);
        
        let parsed: Block = msg.parse_payload().unwrap();
        assert_eq!(parsed.height(), 0);
    }
}
