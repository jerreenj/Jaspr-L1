//! RPC API types and request/response definitions
//!
//! Provides JSON-RPC 2.0 compliant API types for:
//! - Account queries
//! - Transaction submission
//! - Block queries
//! - Network status
//! - Move VM interactions

use jaspr_types::{Address, HashValue, Amount, SignedTransaction, Block, BlockHeader};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JSON-RPC version
pub const JSONRPC_VERSION: &str = "2.0";

/// RPC request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcRequest {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Request ID
    pub id: RpcId,
    /// Method name
    pub method: String,
    /// Parameters
    #[serde(default)]
    pub params: serde_json::Value,
}

impl RpcRequest {
    pub fn new(id: u64, method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: RpcId::Number(id),
            method: method.to_string(),
            params,
        }
    }
}

/// RPC request ID
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RpcId {
    Number(u64),
    String(String),
    Null,
}

/// RPC response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcResponse {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Request ID
    pub id: RpcId,
    /// Result (on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error (on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl RpcResponse {
    pub fn success(id: RpcId, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }
    
    pub fn error(id: RpcId, code: i32, message: String) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(RpcError {
                code,
                message,
                data: None,
            }),
        }
    }
}

/// RPC error
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Standard JSON-RPC error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    
    // Custom error codes
    pub const RESOURCE_NOT_FOUND: i32 = -32000;
    pub const TRANSACTION_REJECTED: i32 = -32001;
    pub const INSUFFICIENT_FUNDS: i32 = -32002;
    pub const NONCE_TOO_LOW: i32 = -32003;
    pub const GAS_LIMIT_EXCEEDED: i32 = -32004;
    pub const CONTRACT_ERROR: i32 = -32005;
    pub const UNAUTHORIZED: i32 = -32006;
    pub const RATE_LIMITED: i32 = -32007;
}

// ============= Account API Types =============

/// Get account info request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetAccountRequest {
    pub address: String,
}

/// Account info response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountInfo {
    /// Account address
    pub address: String,
    /// Balance in base units
    pub balance: Amount,
    /// Balance formatted
    pub balance_formatted: String,
    /// Account nonce
    pub nonce: u64,
    /// Has code (is contract)
    pub has_code: bool,
    /// Code hash (if contract)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_hash: Option<String>,
    /// Account resources
    pub resources: Vec<ResourceInfo>,
}

/// Resource info
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceInfo {
    /// Resource type
    pub type_tag: String,
    /// Resource data (JSON)
    pub data: serde_json::Value,
}

/// Get balance request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetBalanceRequest {
    pub address: String,
    /// Coin type (default: native)
    #[serde(default)]
    pub coin_type: Option<String>,
}

/// Balance response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub address: String,
    pub balance: Amount,
    pub balance_formatted: String,
    pub coin_type: String,
}

/// Get nonce request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetNonceRequest {
    pub address: String,
}

/// Nonce response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NonceResponse {
    pub address: String,
    pub nonce: u64,
}

// ============= Transaction API Types =============

/// Submit transaction request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitTransactionRequest {
    /// Signed transaction (hex or base64)
    pub signed_transaction: String,
    /// Encoding format
    #[serde(default = "default_encoding")]
    pub encoding: String,
}

fn default_encoding() -> String {
    "hex".to_string()
}

/// Submit transaction response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitTransactionResponse {
    /// Transaction hash
    pub hash: String,
    /// Status
    pub status: String,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Get transaction request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetTransactionRequest {
    pub hash: String,
}

/// Transaction info response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionInfo {
    /// Transaction hash
    pub hash: String,
    /// Sender address
    pub sender: String,
    /// Recipient address (for transfers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient: Option<String>,
    /// Amount (for transfers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Amount>,
    /// Gas price
    pub gas_price: u64,
    /// Gas limit
    pub gas_limit: u64,
    /// Gas used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<u64>,
    /// Nonce
    pub nonce: u64,
    /// Transaction type
    pub tx_type: String,
    /// Status (pending, confirmed, failed)
    pub status: String,
    /// Block height (if confirmed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_height: Option<u64>,
    /// Block hash (if confirmed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<String>,
    /// Timestamp
    pub timestamp: u64,
    /// Events emitted
    pub events: Vec<EventInfo>,
}

/// Event info
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventInfo {
    /// Event key
    pub key: String,
    /// Sequence number
    pub sequence_number: u64,
    /// Event type
    pub type_tag: String,
    /// Event data (JSON)
    pub data: serde_json::Value,
}

/// Simulate transaction request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulateTransactionRequest {
    /// Transaction (unsigned)
    pub transaction: String,
    /// Sender for simulation
    pub sender: String,
}

/// Simulation result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Would succeed
    pub success: bool,
    /// Gas estimate
    pub gas_used: u64,
    /// Return value (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_value: Option<serde_json::Value>,
    /// Events that would be emitted
    pub events: Vec<EventInfo>,
    /// Error (if would fail)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Estimate gas request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EstimateGasRequest {
    pub sender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Gas estimate response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GasEstimateResponse {
    pub gas_estimate: u64,
    pub gas_unit_price: u64,
    pub total_cost: Amount,
}

// ============= Block API Types =============

/// Get block request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetBlockRequest {
    /// Block height or "latest"
    pub height_or_hash: String,
    /// Include full transactions
    #[serde(default)]
    pub full_transactions: bool,
}

/// Block info response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block hash
    pub hash: String,
    /// Block height
    pub height: u64,
    /// Parent hash
    pub parent_hash: String,
    /// Proposer address
    pub proposer: String,
    /// State root
    pub state_root: String,
    /// Transactions root
    pub transactions_root: String,
    /// Receipts root
    pub receipts_root: String,
    /// Timestamp
    pub timestamp: u64,
    /// Gas used
    pub gas_used: u64,
    /// Gas limit
    pub gas_limit: u64,
    /// Transaction count
    pub tx_count: usize,
    /// Is finalized
    pub finalized: bool,
    /// Finality time (ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finality_time_ms: Option<u64>,
    /// Transactions (hashes or full)
    pub transactions: Vec<serde_json::Value>,
}

/// Get blocks request (for range)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetBlocksRequest {
    pub from_height: u64,
    pub to_height: u64,
    #[serde(default)]
    pub full_transactions: bool,
}

/// Block header response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockHeaderInfo {
    pub hash: String,
    pub height: u64,
    pub parent_hash: String,
    pub proposer: String,
    pub timestamp: u64,
    pub tx_count: usize,
    pub finalized: bool,
}

// ============= Network API Types =============

/// Network info response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Chain ID
    pub chain_id: u64,
    /// Network name
    pub network: String,
    /// Current block height
    pub height: u64,
    /// Latest block hash
    pub latest_block_hash: String,
    /// Current TPS
    pub tps: f64,
    /// Target TPS
    pub target_tps: u64,
    /// Total transactions
    pub total_transactions: u64,
    /// Node version
    pub version: String,
    /// Is syncing
    pub syncing: bool,
    /// Peer count
    pub peer_count: usize,
}

/// Peer info
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerInfoResponse {
    /// Peer ID
    pub id: String,
    /// Address
    pub address: String,
    /// Port
    pub port: u16,
    /// Latency (ms)
    pub latency_ms: u64,
    /// Peer version
    pub version: String,
    /// Is validator
    pub is_validator: bool,
    /// Last seen timestamp
    pub last_seen: u64,
}

/// Gas price response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GasPriceResponse {
    /// Recommended gas price
    pub gas_price: u64,
    /// Minimum gas price
    pub min_gas_price: u64,
    /// Fast gas price (priority)
    pub fast_gas_price: u64,
}

// ============= Validator API Types =============

/// Validator info
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorInfo {
    /// Validator address
    pub address: String,
    /// Name/moniker
    pub name: String,
    /// Total stake
    pub stake: Amount,
    /// Voting power percentage
    pub voting_power_percent: f64,
    /// Commission rate (percentage)
    pub commission_rate: f64,
    /// Is active
    pub active: bool,
    /// Is jailed
    pub jailed: bool,
    /// Uptime percentage
    pub uptime: f64,
    /// Blocks proposed
    pub blocks_proposed: u64,
    /// Estimated APY
    pub apy: f64,
}

/// Get validators response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorsResponse {
    pub validators: Vec<ValidatorInfo>,
    pub total_stake: Amount,
    pub active_count: usize,
    pub total_count: usize,
}

/// Staking info for address
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StakingInfo {
    pub address: String,
    pub total_staked: Amount,
    pub stakes: Vec<StakeEntry>,
    pub unbonding: Vec<UnbondingEntry>,
    pub rewards_earned: Amount,
    pub claimable_rewards: Amount,
}

/// Stake entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StakeEntry {
    pub validator: String,
    pub validator_name: String,
    pub amount: Amount,
    pub apy: f64,
    pub estimated_daily_reward: Amount,
}

/// Unbonding entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnbondingEntry {
    pub validator: String,
    pub amount: Amount,
    pub unlock_time: u64,
    pub remaining_days: f64,
    pub is_claimable: bool,
}

// ============= Move VM API Types =============

/// Get module request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetModuleRequest {
    pub address: String,
    pub module_name: String,
}

/// Module info response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleInfoResponse {
    pub address: String,
    pub name: String,
    pub bytecode_hash: String,
    pub bytecode_size: usize,
    pub abi: ModuleAbi,
}

/// Module ABI
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleAbi {
    pub structs: Vec<StructAbi>,
    pub functions: Vec<FunctionAbi>,
    pub friends: Vec<String>,
}

/// Struct ABI
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructAbi {
    pub name: String,
    pub type_parameters: Vec<String>,
    pub abilities: Vec<String>,
    pub fields: Vec<FieldAbi>,
}

/// Field ABI
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldAbi {
    pub name: String,
    pub type_signature: String,
}

/// Function ABI
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionAbi {
    pub name: String,
    pub visibility: String,
    pub is_entry: bool,
    pub type_parameters: Vec<String>,
    pub parameters: Vec<String>,
    pub returns: Vec<String>,
}

/// View function request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewFunctionRequest {
    pub module_address: String,
    pub module_name: String,
    pub function_name: String,
    #[serde(default)]
    pub type_args: Vec<String>,
    #[serde(default)]
    pub args: Vec<serde_json::Value>,
}

/// View function response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewFunctionResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Get resource request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetResourceRequest {
    pub address: String,
    pub resource_type: String,
}

/// Resource response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceResponse {
    pub address: String,
    pub type_tag: String,
    pub data: serde_json::Value,
}

// ============= Mempool API Types =============

/// Mempool stats
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MempoolStats {
    pub pending_count: usize,
    pub queued_count: usize,
    pub total_count: usize,
    pub pending_gas: u64,
    pub sender_count: usize,
}

/// Pending transactions response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingTransactionsResponse {
    pub transactions: Vec<TransactionInfo>,
    pub count: usize,
}

// ============= Batch API Types =============

/// Batch request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchRequest {
    pub requests: Vec<RpcRequest>,
}

/// Batch response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchResponse {
    pub responses: Vec<RpcResponse>,
}

// ============= Subscription Types =============

/// Subscription request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscribeRequest {
    /// Subscription type
    pub subscription_type: SubscriptionType,
    /// Filter (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<SubscriptionFilter>,
}

/// Subscription type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SubscriptionType {
    #[serde(rename = "newHeads")]
    NewBlocks,
    #[serde(rename = "newPendingTransactions")]
    PendingTransactions,
    #[serde(rename = "logs")]
    Logs,
    #[serde(rename = "staking")]
    Staking,
}

/// Subscription filter
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscriptionFilter {
    /// Address filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Vec<String>>,
    /// Topics filter (for logs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics: Option<Vec<String>>,
}

/// Subscription response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscribeResponse {
    /// Subscription ID
    pub subscription_id: String,
}

/// Subscription notification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscriptionNotification {
    /// Subscription ID
    pub subscription: String,
    /// Notification data
    pub result: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rpc_request_serialization() {
        let req = RpcRequest::new(1, "getBalance", serde_json::json!({"address": "0x123"}));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("getBalance"));
        assert!(json.contains("2.0"));
    }
    
    #[test]
    fn test_rpc_response_success() {
        let resp = RpcResponse::success(
            RpcId::Number(1),
            serde_json::json!({"balance": 1000}),
        );
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }
    
    #[test]
    fn test_rpc_response_error() {
        let resp = RpcResponse::error(
            RpcId::Number(1),
            error_codes::INVALID_PARAMS,
            "Invalid address".to_string(),
        );
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, error_codes::INVALID_PARAMS);
    }
}
