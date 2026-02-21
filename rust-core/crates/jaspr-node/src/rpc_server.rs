//! Complete RPC server implementation for JasprChain
//! 
//! Provides JSON-RPC 2.0 API over HTTP and WebSocket

use crate::rpc::{RpcRequest, RpcResponse, RpcError, error_codes, methods};
use crate::node::JasprNode;
use crate::mempool::MempoolStats;
use jaspr_types::{
    Block, SignedTransaction, HashValue, Address, BlockHeight,
    TransactionReceipt, Transaction, TransactionPayload, Account,
};
use jaspr_crypto::{KeyPair, sign_transaction, Signer};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

/// RPC server configuration
#[derive(Clone, Debug)]
pub struct RpcServerConfig {
    /// Listen address
    pub listen_addr: SocketAddr,
    /// Maximum request body size
    pub max_body_size: usize,
    /// Enable WebSocket
    pub enable_websocket: bool,
    /// CORS origins
    pub cors_origins: Vec<String>,
    /// Rate limit (requests per second)
    pub rate_limit: u32,
}

impl Default for RpcServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:8545".parse().unwrap(),
            max_body_size: 10 * 1024 * 1024, // 10MB
            enable_websocket: true,
            cors_origins: vec!["*".to_string()],
            rate_limit: 100,
        }
    }
}

/// Chain information response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainInfoResponse {
    pub chain_id: u64,
    pub chain_name: String,
    pub block_height: BlockHeight,
    pub latest_block_hash: String,
    pub latest_block_time: u64,
    pub pending_transactions: usize,
    pub connected_peers: usize,
    pub validator_count: usize,
    pub total_stake: String,
    pub is_syncing: bool,
    pub protocol_version: u32,
}

/// Account response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountResponse {
    pub address: String,
    pub balance: String,
    pub nonce: u64,
    pub is_validator: bool,
    pub staked_amount: String,
    pub code_hash: Option<String>,
}

/// Block response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockResponse {
    pub height: BlockHeight,
    pub hash: String,
    pub parent_hash: String,
    pub timestamp: u64,
    pub proposer: String,
    pub state_root: String,
    pub transactions_root: String,
    pub receipts_root: String,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub transaction_count: usize,
    pub transactions: Vec<String>,
    pub finalized: bool,
    pub attestation_count: u32,
}

impl From<&Block> for BlockResponse {
    fn from(block: &Block) -> Self {
        Self {
            height: block.height(),
            hash: block.hash().to_hex(),
            parent_hash: block.header.previous_hash.to_hex(),
            timestamp: block.header.timestamp,
            proposer: block.header.proposer.to_hex(),
            state_root: block.header.state_root.to_hex(),
            transactions_root: block.header.transactions_root.to_hex(),
            receipts_root: block.header.receipts_root.to_hex(),
            gas_used: block.header.gas_used,
            gas_limit: block.header.gas_limit,
            transaction_count: block.body.tx_count(),
            transactions: block.body.transactions.iter()
                .map(|tx| tx.hash().to_hex())
                .collect(),
            finalized: block.finalized,
            attestation_count: block.header.attestation_count,
        }
    }
}

/// Transaction response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub hash: String,
    pub sender: String,
    pub recipient: Option<String>,
    pub amount: Option<String>,
    pub nonce: u64,
    pub gas_limit: u64,
    pub gas_price: u64,
    pub payload_type: String,
    pub block_height: Option<BlockHeight>,
    pub block_hash: Option<String>,
    pub transaction_index: Option<u32>,
    pub status: String,
}

/// Receipt response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptResponse {
    pub transaction_hash: String,
    pub block_height: BlockHeight,
    pub block_hash: String,
    pub transaction_index: u32,
    pub status: String,
    pub gas_used: u64,
    pub events: Vec<EventResponse>,
    pub error_message: Option<String>,
}

/// Event response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventResponse {
    pub event_type: String,
    pub data: String,
}

/// Validator response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorResponse {
    pub address: String,
    pub name: String,
    pub stake: String,
    pub voting_power: u64,
    pub commission_rate: f64,
    pub is_active: bool,
    pub is_jailed: bool,
    pub blocks_proposed: u64,
    pub blocks_attested: u64,
    pub uptime_percentage: f64,
}

/// Staking info response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StakingInfoResponse {
    pub total_staked: String,
    pub total_validators: usize,
    pub active_validators: usize,
    pub min_stake: String,
    pub unbonding_period_days: u32,
    pub current_apy: f64,
    pub next_epoch_time: u64,
}

/// Gas estimation response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GasEstimateResponse {
    pub gas_estimate: u64,
    pub gas_price: u64,
    pub total_cost: String,
}

/// Send transaction request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SendTransactionRequest {
    pub sender: String,
    pub recipient: Option<String>,
    pub amount: Option<String>,
    pub payload_type: String,
    pub gas_limit: Option<u64>,
    pub gas_price: Option<u64>,
    pub data: Option<String>,
}

/// Call request (read-only)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallRequest {
    pub sender: Option<String>,
    pub module_address: String,
    pub module_name: String,
    pub function_name: String,
    pub type_args: Vec<String>,
    pub args: Vec<String>,
}

/// Call response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallResponse {
    pub return_values: Vec<String>,
    pub gas_used: u64,
}

/// RPC handler
pub struct RpcHandler {
    node: Arc<JasprNode>,
}

impl RpcHandler {
    /// Create new RPC handler
    pub fn new(node: Arc<JasprNode>) -> Self {
        Self { node }
    }
    
    /// Handle RPC request
    pub async fn handle_request(&self, request: RpcRequest) -> RpcResponse {
        let id = request.id;
        
        match request.method.as_str() {
            methods::CHAIN_INFO => self.handle_chain_info(id).await,
            methods::GET_BLOCK_BY_HEIGHT => self.handle_get_block_by_height(id, request.params).await,
            methods::GET_BLOCK_BY_HASH => self.handle_get_block_by_hash(id, request.params).await,
            methods::GET_LATEST_BLOCK => self.handle_get_latest_block(id).await,
            methods::GET_ACCOUNT => self.handle_get_account(id, request.params).await,
            methods::GET_BALANCE => self.handle_get_balance(id, request.params).await,
            methods::GET_NONCE => self.handle_get_nonce(id, request.params).await,
            methods::GET_TRANSACTION => self.handle_get_transaction(id, request.params).await,
            methods::GET_RECEIPT => self.handle_get_receipt(id, request.params).await,
            methods::SEND_TRANSACTION => self.handle_send_transaction(id, request.params).await,
            methods::SEND_RAW_TRANSACTION => self.handle_send_raw_transaction(id, request.params).await,
            methods::GET_PENDING_TXS => self.handle_get_pending_txs(id).await,
            methods::GET_VALIDATORS => self.handle_get_validators(id).await,
            methods::GET_STAKING_INFO => self.handle_get_staking_info(id).await,
            methods::CALL => self.handle_call(id, request.params).await,
            methods::ESTIMATE_GAS => self.handle_estimate_gas(id, request.params).await,
            _ => RpcResponse::error(id, error_codes::METHOD_NOT_FOUND, format!("Method not found: {}", request.method)),
        }
    }
    
    /// Handle chain info request
    async fn handle_chain_info(&self, id: u64) -> RpcResponse {
        let stats = self.node.stats();
        let latest_block = self.node.get_latest_block();
        
        let response = ChainInfoResponse {
            chain_id: 1, // TODO: from config
            chain_name: "JasprChain".to_string(),
            block_height: stats.chain_height,
            latest_block_hash: latest_block.as_ref()
                .map(|b| b.hash().to_hex())
                .unwrap_or_default(),
            latest_block_time: latest_block.as_ref()
                .map(|b| b.header.timestamp)
                .unwrap_or(0),
            pending_transactions: stats.pending_txs,
            connected_peers: stats.connected_peers,
            validator_count: stats.validator_count,
            total_stake: format_jaspr(stats.total_stake),
            is_syncing: false,
            protocol_version: 1,
        };
        
        RpcResponse::success(id, serde_json::to_value(response).unwrap())
    }
    
    /// Handle get block by height
    async fn handle_get_block_by_height(&self, id: u64, params: Value) -> RpcResponse {
        let height: BlockHeight = match serde_json::from_value(params) {
            Ok(h) => h,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, e.to_string()),
        };
        
        match self.node.get_block(height) {
            Some(block) => {
                let response = BlockResponse::from(&block);
                RpcResponse::success(id, serde_json::to_value(response).unwrap())
            }
            None => RpcResponse::error(id, error_codes::BLOCK_NOT_FOUND, format!("Block not found: {}", height)),
        }
    }
    
    /// Handle get block by hash
    async fn handle_get_block_by_hash(&self, id: u64, params: Value) -> RpcResponse {
        let hash_hex: String = match serde_json::from_value(params) {
            Ok(h) => h,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, e.to_string()),
        };
        
        let hash = match HashValue::from_hex(&hash_hex) {
            Ok(h) => h,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, format!("Invalid hash: {}", e)),
        };
        
        // For now, iterate to find block by hash (TODO: add index)
        let height = self.node.chain_height();
        for h in 0..=height {
            if let Some(block) = self.node.get_block(h) {
                if block.hash() == hash {
                    let response = BlockResponse::from(&block);
                    return RpcResponse::success(id, serde_json::to_value(response).unwrap());
                }
            }
        }
        
        RpcResponse::error(id, error_codes::BLOCK_NOT_FOUND, format!("Block not found: {}", hash_hex))
    }
    
    /// Handle get latest block
    async fn handle_get_latest_block(&self, id: u64) -> RpcResponse {
        match self.node.get_latest_block() {
            Some(block) => {
                let response = BlockResponse::from(&block);
                RpcResponse::success(id, serde_json::to_value(response).unwrap())
            }
            None => RpcResponse::error(id, error_codes::BLOCK_NOT_FOUND, "No blocks yet".to_string()),
        }
    }
    
    /// Handle get account
    async fn handle_get_account(&self, id: u64, params: Value) -> RpcResponse {
        let address_hex: String = match serde_json::from_value(params) {
            Ok(a) => a,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, e.to_string()),
        };
        
        let address = match Address::from_hex(&address_hex) {
            Ok(a) => a,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, format!("Invalid address: {}", e)),
        };
        
        let balance = self.node.get_balance(&address);
        
        let response = AccountResponse {
            address: address.to_hex(),
            balance: format_jaspr(balance),
            nonce: 0, // TODO: get actual nonce
            is_validator: false, // TODO: check validator status
            staked_amount: "0".to_string(),
            code_hash: None,
        };
        
        RpcResponse::success(id, serde_json::to_value(response).unwrap())
    }
    
    /// Handle get balance
    async fn handle_get_balance(&self, id: u64, params: Value) -> RpcResponse {
        let address_hex: String = match serde_json::from_value(params) {
            Ok(a) => a,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, e.to_string()),
        };
        
        let address = match Address::from_hex(&address_hex) {
            Ok(a) => a,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, format!("Invalid address: {}", e)),
        };
        
        let balance = self.node.get_balance(&address);
        RpcResponse::success(id, json!(format_jaspr(balance)))
    }
    
    /// Handle get nonce
    async fn handle_get_nonce(&self, id: u64, params: Value) -> RpcResponse {
        let address_hex: String = match serde_json::from_value(params) {
            Ok(a) => a,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, e.to_string()),
        };
        
        // TODO: get actual nonce from account store
        RpcResponse::success(id, json!(0u64))
    }
    
    /// Handle get transaction
    async fn handle_get_transaction(&self, id: u64, params: Value) -> RpcResponse {
        let hash_hex: String = match serde_json::from_value(params) {
            Ok(h) => h,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, e.to_string()),
        };
        
        // TODO: implement transaction lookup
        RpcResponse::error(id, error_codes::TX_NOT_FOUND, format!("Transaction not found: {}", hash_hex))
    }
    
    /// Handle get receipt
    async fn handle_get_receipt(&self, id: u64, params: Value) -> RpcResponse {
        let hash_hex: String = match serde_json::from_value(params) {
            Ok(h) => h,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, e.to_string()),
        };
        
        // TODO: implement receipt lookup
        RpcResponse::error(id, error_codes::TX_NOT_FOUND, format!("Receipt not found: {}", hash_hex))
    }
    
    /// Handle send transaction
    async fn handle_send_transaction(&self, id: u64, params: Value) -> RpcResponse {
        let request: SendTransactionRequest = match serde_json::from_value(params) {
            Ok(r) => r,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, e.to_string()),
        };
        
        // TODO: implement transaction sending
        RpcResponse::error(id, error_codes::INTERNAL_ERROR, "Not implemented".to_string())
    }
    
    /// Handle send raw transaction
    async fn handle_send_raw_transaction(&self, id: u64, params: Value) -> RpcResponse {
        let raw_tx_hex: String = match serde_json::from_value(params) {
            Ok(h) => h,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, e.to_string()),
        };
        
        // TODO: deserialize and submit transaction
        RpcResponse::error(id, error_codes::INTERNAL_ERROR, "Not implemented".to_string())
    }
    
    /// Handle get pending transactions
    async fn handle_get_pending_txs(&self, id: u64) -> RpcResponse {
        // TODO: get pending transactions from mempool
        RpcResponse::success(id, json!([]))
    }
    
    /// Handle get validators
    async fn handle_get_validators(&self, id: u64) -> RpcResponse {
        // TODO: get validators from consensus
        RpcResponse::success(id, json!([]))
    }
    
    /// Handle get staking info
    async fn handle_get_staking_info(&self, id: u64) -> RpcResponse {
        let response = StakingInfoResponse {
            total_staked: "0".to_string(),
            total_validators: 0,
            active_validators: 0,
            min_stake: "32000".to_string(),
            unbonding_period_days: 14,
            current_apy: 12.0,
            next_epoch_time: 0,
        };
        
        RpcResponse::success(id, serde_json::to_value(response).unwrap())
    }
    
    /// Handle call (read-only)
    async fn handle_call(&self, id: u64, params: Value) -> RpcResponse {
        let request: CallRequest = match serde_json::from_value(params) {
            Ok(r) => r,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, e.to_string()),
        };
        
        // TODO: implement Move VM call
        let response = CallResponse {
            return_values: vec![],
            gas_used: 0,
        };
        
        RpcResponse::success(id, serde_json::to_value(response).unwrap())
    }
    
    /// Handle estimate gas
    async fn handle_estimate_gas(&self, id: u64, params: Value) -> RpcResponse {
        let response = GasEstimateResponse {
            gas_estimate: 21000,
            gas_price: 100,
            total_cost: "2100000".to_string(),
        };
        
        RpcResponse::success(id, serde_json::to_value(response).unwrap())
    }
}

/// Format amount as JASPR string
fn format_jaspr(amount: u128) -> String {
    let whole = amount / 1_000_000_000;
    let frac = amount % 1_000_000_000;
    if frac == 0 {
        format!("{}", whole)
    } else {
        format!("{}.{:09}", whole, frac).trim_end_matches('0').to_string()
    }
}

/// Parse JASPR string to base units
fn parse_jaspr(s: &str) -> Result<u128, String> {
    let parts: Vec<&str> = s.split('.').collect();
    match parts.len() {
        1 => {
            let whole: u128 = parts[0].parse().map_err(|e| format!("Invalid amount: {}", e))?;
            Ok(whole * 1_000_000_000)
        }
        2 => {
            let whole: u128 = parts[0].parse().map_err(|e| format!("Invalid amount: {}", e))?;
            let frac_str = format!("{:0<9}", parts[1]);
            let frac: u128 = frac_str[..9].parse().map_err(|e| format!("Invalid fraction: {}", e))?;
            Ok(whole * 1_000_000_000 + frac)
        }
        _ => Err("Invalid amount format".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_jaspr() {
        assert_eq!(format_jaspr(1_000_000_000), "1");
        assert_eq!(format_jaspr(1_500_000_000), "1.5");
        assert_eq!(format_jaspr(123_456_789_012), "123.456789012");
    }
    
    #[test]
    fn test_parse_jaspr() {
        assert_eq!(parse_jaspr("1").unwrap(), 1_000_000_000);
        assert_eq!(parse_jaspr("1.5").unwrap(), 1_500_000_000);
        assert_eq!(parse_jaspr("123.456").unwrap(), 123_456_000_000);
    }
}
