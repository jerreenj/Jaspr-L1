//! RPC server for JasprChain

use jaspr_types::{Block, SignedTransaction, HashValue, Address, BlockHeight, TransactionReceipt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// RPC request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
}

/// RPC response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<RpcError>,
    pub id: u64,
}

/// RPC error
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl RpcResponse {
    pub fn success(id: u64, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }
    
    pub fn error(id: u64, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError {
                code,
                message,
                data: None,
            }),
            id,
        }
    }
}

/// Chain info response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainInfo {
    pub chain_id: u64,
    pub block_height: BlockHeight,
    pub latest_block_hash: String,
    pub pending_txs: usize,
    pub connected_peers: usize,
    pub is_syncing: bool,
}

/// Account info response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountInfo {
    pub address: String,
    pub balance: String,
    pub nonce: u64,
    pub is_contract: bool,
}

/// Transaction info response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub hash: String,
    pub sender: String,
    pub nonce: u64,
    pub gas_limit: u64,
    pub gas_price: u64,
    pub status: String,
    pub block_height: Option<BlockHeight>,
    pub block_hash: Option<String>,
}

/// Block info response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockInfo {
    pub height: BlockHeight,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: u64,
    pub proposer: String,
    pub tx_count: usize,
    pub gas_used: u64,
    pub state_root: String,
    pub finalized: bool,
}

impl From<&Block> for BlockInfo {
    fn from(block: &Block) -> Self {
        Self {
            height: block.height(),
            hash: block.hash().to_hex(),
            previous_hash: block.header.previous_hash.to_hex(),
            timestamp: block.header.timestamp,
            proposer: block.header.proposer.to_hex(),
            tx_count: block.body.tx_count(),
            gas_used: block.header.gas_used,
            state_root: block.header.state_root.to_hex(),
            finalized: block.finalized,
        }
    }
}

/// RPC method names
pub mod methods {
    pub const CHAIN_INFO: &str = "jaspr_chainInfo";
    pub const GET_BLOCK_BY_HEIGHT: &str = "jaspr_getBlockByHeight";
    pub const GET_BLOCK_BY_HASH: &str = "jaspr_getBlockByHash";
    pub const GET_LATEST_BLOCK: &str = "jaspr_getLatestBlock";
    pub const GET_ACCOUNT: &str = "jaspr_getAccount";
    pub const GET_BALANCE: &str = "jaspr_getBalance";
    pub const GET_NONCE: &str = "jaspr_getNonce";
    pub const GET_TRANSACTION: &str = "jaspr_getTransaction";
    pub const GET_RECEIPT: &str = "jaspr_getReceipt";
    pub const SEND_TRANSACTION: &str = "jaspr_sendTransaction";
    pub const SEND_RAW_TRANSACTION: &str = "jaspr_sendRawTransaction";
    pub const GET_PENDING_TXS: &str = "jaspr_getPendingTransactions";
    pub const GET_VALIDATORS: &str = "jaspr_getValidators";
    pub const GET_STAKING_INFO: &str = "jaspr_getStakingInfo";
    pub const CALL: &str = "jaspr_call";
    pub const ESTIMATE_GAS: &str = "jaspr_estimateGas";
}

/// RPC error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const BLOCK_NOT_FOUND: i32 = -32001;
    pub const TX_NOT_FOUND: i32 = -32002;
    pub const ACCOUNT_NOT_FOUND: i32 = -32003;
    pub const INSUFFICIENT_FUNDS: i32 = -32004;
    pub const NONCE_TOO_LOW: i32 = -32005;
    pub const GAS_LIMIT_EXCEEDED: i32 = -32006;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rpc_response() {
        let resp = RpcResponse::success(1, serde_json::json!({"height": 100}));
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }
    
    #[test]
    fn test_rpc_error() {
        let resp = RpcResponse::error(1, error_codes::BLOCK_NOT_FOUND, "Block not found".to_string());
        assert!(resp.error.is_some());
        assert!(resp.result.is_none());
    }
}
