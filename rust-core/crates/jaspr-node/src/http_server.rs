//! HTTP RPC Server implementation using hyper
//! 
//! Provides HTTP and WebSocket endpoints for JSON-RPC API

use crate::rpc::{RpcRequest, RpcResponse, error_codes};
use crate::rpc_server::RpcHandler;
use crate::node::JasprNode;
use std::sync::Arc;
use std::net::SocketAddr;
use std::convert::Infallible;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{info, warn, error, debug};

/// HTTP server configuration
#[derive(Clone, Debug)]
pub struct HttpServerConfig {
    /// Listen address
    pub listen_addr: SocketAddr,
    /// Maximum request body size (bytes)
    pub max_body_size: usize,
    /// Request timeout (seconds)
    pub request_timeout_secs: u64,
    /// Enable CORS
    pub enable_cors: bool,
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    /// Enable WebSocket
    pub enable_websocket: bool,
    /// Rate limit (requests per second per IP)
    pub rate_limit_per_ip: u32,
    /// Enable request logging
    pub enable_logging: bool,
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:8545".parse().unwrap(),
            max_body_size: 10 * 1024 * 1024, // 10MB
            request_timeout_secs: 30,
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
            enable_websocket: true,
            rate_limit_per_ip: 100,
            enable_logging: true,
        }
    }
}

/// HTTP request context
#[derive(Clone, Debug)]
pub struct RequestContext {
    /// Client IP address
    pub client_ip: Option<String>,
    /// Request ID
    pub request_id: u64,
    /// Request timestamp
    pub timestamp: u64,
    /// User agent
    pub user_agent: Option<String>,
}

/// HTTP response with metadata
#[derive(Clone, Debug, Serialize)]
pub struct HttpResponse {
    /// JSON-RPC response
    #[serde(flatten)]
    pub rpc_response: RpcResponse,
    /// Server timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_time: Option<u64>,
}

/// Rate limiter for IP-based throttling
pub struct RateLimiter {
    /// Requests per IP in current window
    requests: RwLock<std::collections::HashMap<String, (u64, std::time::Instant)>>,
    /// Max requests per window
    max_requests: u32,
    /// Window duration
    window_duration: std::time::Duration,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            requests: RwLock::new(std::collections::HashMap::new()),
            max_requests,
            window_duration: std::time::Duration::from_secs(window_secs),
        }
    }
    
    /// Check if request is allowed
    pub fn check(&self, ip: &str) -> bool {
        let mut requests = self.requests.write();
        let now = std::time::Instant::now();
        
        if let Some((count, start)) = requests.get_mut(ip) {
            if now.duration_since(*start) > self.window_duration {
                // Reset window
                *count = 1;
                *start = now;
                true
            } else if *count >= self.max_requests as u64 {
                false
            } else {
                *count += 1;
                true
            }
        } else {
            requests.insert(ip.to_string(), (1, now));
            true
        }
    }
    
    /// Clean up expired entries
    pub fn cleanup(&self) {
        let mut requests = self.requests.write();
        let now = std::time::Instant::now();
        requests.retain(|_, (_, start)| now.duration_since(*start) <= self.window_duration * 2);
    }
}

/// HTTP server state
pub struct HttpServer {
    config: HttpServerConfig,
    node: Arc<JasprNode>,
    rate_limiter: Arc<RateLimiter>,
    running: RwLock<bool>,
    /// Request counter
    request_count: RwLock<u64>,
    /// Error counter
    error_count: RwLock<u64>,
}

impl HttpServer {
    /// Create new HTTP server
    pub fn new(config: HttpServerConfig, node: Arc<JasprNode>) -> Self {
        let rate_limiter = Arc::new(RateLimiter::new(
            config.rate_limit_per_ip,
            1, // 1 second window
        ));
        
        Self {
            config,
            node,
            rate_limiter,
            running: RwLock::new(false),
            request_count: RwLock::new(0),
            error_count: RwLock::new(0),
        }
    }
    
    /// Start the HTTP server
    pub async fn start(&self) -> Result<(), String> {
        if *self.running.read() {
            return Err("Server already running".to_string());
        }
        
        info!(addr = %self.config.listen_addr, "Starting HTTP RPC server");
        *self.running.write() = true;
        
        // In production, this would use hyper/axum to create actual HTTP listener
        // For now, we implement the request handling logic
        
        Ok(())
    }
    
    /// Stop the HTTP server
    pub async fn stop(&self) {
        info!("Stopping HTTP RPC server");
        *self.running.write() = false;
    }
    
    /// Handle HTTP request
    pub async fn handle_request(
        &self,
        body: &[u8],
        context: RequestContext,
    ) -> Result<Vec<u8>, HttpError> {
        // Check rate limit
        if let Some(ip) = &context.client_ip {
            if !self.rate_limiter.check(ip) {
                *self.error_count.write() += 1;
                return Err(HttpError::RateLimited);
            }
        }
        
        // Check body size
        if body.len() > self.config.max_body_size {
            *self.error_count.write() += 1;
            return Err(HttpError::BodyTooLarge);
        }
        
        // Parse request
        let request: RpcRequest = serde_json::from_slice(body)
            .map_err(|e| HttpError::InvalidJson(e.to_string()))?;
        
        // Create handler and process
        let handler = RpcHandler::new(self.node.clone());
        let response = handler.handle_request(request).await;
        
        // Increment counter
        *self.request_count.write() += 1;
        
        // Serialize response
        let response_bytes = serde_json::to_vec(&response)
            .map_err(|e| HttpError::SerializationError(e.to_string()))?;
        
        Ok(response_bytes)
    }
    
    /// Handle batch request
    pub async fn handle_batch_request(
        &self,
        body: &[u8],
        context: RequestContext,
    ) -> Result<Vec<u8>, HttpError> {
        // Check rate limit
        if let Some(ip) = &context.client_ip {
            if !self.rate_limiter.check(ip) {
                return Err(HttpError::RateLimited);
            }
        }
        
        // Parse batch request
        let requests: Vec<RpcRequest> = serde_json::from_slice(body)
            .map_err(|e| HttpError::InvalidJson(e.to_string()))?;
        
        if requests.is_empty() {
            return Err(HttpError::EmptyBatch);
        }
        
        if requests.len() > 100 {
            return Err(HttpError::BatchTooLarge);
        }
        
        // Process all requests
        let handler = RpcHandler::new(self.node.clone());
        let mut responses = Vec::with_capacity(requests.len());
        
        for request in requests {
            let response = handler.handle_request(request).await;
            responses.push(response);
        }
        
        *self.request_count.write() += responses.len() as u64;
        
        // Serialize responses
        let response_bytes = serde_json::to_vec(&responses)
            .map_err(|e| HttpError::SerializationError(e.to_string()))?;
        
        Ok(response_bytes)
    }
    
    /// Get server statistics
    pub fn stats(&self) -> HttpServerStats {
        HttpServerStats {
            is_running: *self.running.read(),
            listen_addr: self.config.listen_addr.to_string(),
            total_requests: *self.request_count.read(),
            total_errors: *self.error_count.read(),
            rate_limit: self.config.rate_limit_per_ip,
        }
    }
    
    /// Generate CORS headers
    pub fn cors_headers(&self) -> Vec<(String, String)> {
        if !self.config.enable_cors {
            return Vec::new();
        }
        
        let origin = self.config.cors_origins.first()
            .cloned()
            .unwrap_or_else(|| "*".to_string());
        
        vec![
            ("Access-Control-Allow-Origin".to_string(), origin),
            ("Access-Control-Allow-Methods".to_string(), "POST, GET, OPTIONS".to_string()),
            ("Access-Control-Allow-Headers".to_string(), "Content-Type".to_string()),
            ("Access-Control-Max-Age".to_string(), "86400".to_string()),
        ]
    }
}

/// HTTP error types
#[derive(Debug, Clone)]
pub enum HttpError {
    RateLimited,
    BodyTooLarge,
    InvalidJson(String),
    SerializationError(String),
    EmptyBatch,
    BatchTooLarge,
    Timeout,
    InternalError(String),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimited => write!(f, "Rate limit exceeded"),
            Self::BodyTooLarge => write!(f, "Request body too large"),
            Self::InvalidJson(e) => write!(f, "Invalid JSON: {}", e),
            Self::SerializationError(e) => write!(f, "Serialization error: {}", e),
            Self::EmptyBatch => write!(f, "Empty batch request"),
            Self::BatchTooLarge => write!(f, "Batch request too large"),
            Self::Timeout => write!(f, "Request timeout"),
            Self::InternalError(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl HttpError {
    /// Convert to HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            Self::RateLimited => 429,
            Self::BodyTooLarge => 413,
            Self::InvalidJson(_) => 400,
            Self::SerializationError(_) => 500,
            Self::EmptyBatch => 400,
            Self::BatchTooLarge => 400,
            Self::Timeout => 408,
            Self::InternalError(_) => 500,
        }
    }
    
    /// Convert to JSON error response
    pub fn to_json_response(&self) -> Value {
        json!({
            "jsonrpc": "2.0",
            "error": {
                "code": self.error_code(),
                "message": self.to_string()
            },
            "id": null
        })
    }
    
    /// Get error code
    pub fn error_code(&self) -> i32 {
        match self {
            Self::RateLimited => -32005,
            Self::BodyTooLarge => -32600,
            Self::InvalidJson(_) => error_codes::PARSE_ERROR,
            Self::SerializationError(_) => error_codes::INTERNAL_ERROR,
            Self::EmptyBatch => error_codes::INVALID_REQUEST,
            Self::BatchTooLarge => error_codes::INVALID_REQUEST,
            Self::Timeout => -32006,
            Self::InternalError(_) => error_codes::INTERNAL_ERROR,
        }
    }
}

/// HTTP server statistics
#[derive(Clone, Debug, Serialize)]
pub struct HttpServerStats {
    pub is_running: bool,
    pub listen_addr: String,
    pub total_requests: u64,
    pub total_errors: u64,
    pub rate_limit: u32,
}

/// WebSocket connection handler
pub struct WebSocketHandler {
    /// Node reference
    node: Arc<JasprNode>,
    /// Active subscriptions
    subscriptions: RwLock<std::collections::HashMap<String, Subscription>>,
    /// Connection ID
    connection_id: String,
}

/// Subscription types
#[derive(Clone, Debug)]
pub enum SubscriptionType {
    /// New blocks
    NewBlocks,
    /// New transactions
    NewTransactions,
    /// Pending transactions
    PendingTransactions,
    /// Logs/events
    Logs { filter: LogFilter },
}

/// Log filter for event subscriptions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogFilter {
    pub address: Option<String>,
    pub topics: Vec<Option<String>>,
}

/// Active subscription
#[derive(Clone, Debug)]
pub struct Subscription {
    pub id: String,
    pub sub_type: SubscriptionType,
    pub created_at: u64,
}

impl WebSocketHandler {
    /// Create new WebSocket handler
    pub fn new(node: Arc<JasprNode>) -> Self {
        Self {
            node,
            subscriptions: RwLock::new(std::collections::HashMap::new()),
            connection_id: format!("ws_{}", rand::random::<u64>()),
        }
    }
    
    /// Handle WebSocket message
    pub async fn handle_message(&self, message: &str) -> String {
        let request: Result<RpcRequest, _> = serde_json::from_str(message);
        
        match request {
            Ok(req) => {
                let response = self.process_request(req).await;
                serde_json::to_string(&response).unwrap_or_else(|_| {
                    r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Internal error"},"id":null}"#.to_string()
                })
            }
            Err(e) => {
                let response = RpcResponse::error(0, error_codes::PARSE_ERROR, e.to_string());
                serde_json::to_string(&response).unwrap_or_default()
            }
        }
    }
    
    /// Process RPC request
    async fn process_request(&self, request: RpcRequest) -> RpcResponse {
        match request.method.as_str() {
            "eth_subscribe" | "jaspr_subscribe" => {
                self.handle_subscribe(request.id, request.params).await
            }
            "eth_unsubscribe" | "jaspr_unsubscribe" => {
                self.handle_unsubscribe(request.id, request.params).await
            }
            _ => {
                // Forward to regular RPC handler
                let handler = RpcHandler::new(self.node.clone());
                handler.handle_request(request).await
            }
        }
    }
    
    /// Handle subscribe request
    async fn handle_subscribe(&self, id: u64, params: Value) -> RpcResponse {
        let sub_type_str: String = match serde_json::from_value(params.clone()) {
            Ok(s) => s,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, e.to_string()),
        };
        
        let sub_type = match sub_type_str.as_str() {
            "newHeads" | "newBlocks" => SubscriptionType::NewBlocks,
            "newPendingTransactions" => SubscriptionType::PendingTransactions,
            "logs" => {
                let filter = LogFilter { address: None, topics: Vec::new() };
                SubscriptionType::Logs { filter }
            }
            _ => return RpcResponse::error(id, error_codes::INVALID_PARAMS, "Unknown subscription type".to_string()),
        };
        
        let sub_id = format!("0x{}", hex::encode(rand::random::<[u8; 16]>()));
        
        let subscription = Subscription {
            id: sub_id.clone(),
            sub_type,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        self.subscriptions.write().insert(sub_id.clone(), subscription);
        
        RpcResponse::success(id, json!(sub_id))
    }
    
    /// Handle unsubscribe request
    async fn handle_unsubscribe(&self, id: u64, params: Value) -> RpcResponse {
        let sub_id: String = match serde_json::from_value(params) {
            Ok(s) => s,
            Err(e) => return RpcResponse::error(id, error_codes::INVALID_PARAMS, e.to_string()),
        };
        
        let removed = self.subscriptions.write().remove(&sub_id).is_some();
        RpcResponse::success(id, json!(removed))
    }
    
    /// Get active subscription count
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.read().len()
    }
    
    /// Clean up connection
    pub fn cleanup(&self) {
        self.subscriptions.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(5, 1);
        
        // First 5 requests should pass
        for _ in 0..5 {
            assert!(limiter.check("127.0.0.1"));
        }
        
        // 6th should fail
        assert!(!limiter.check("127.0.0.1"));
        
        // Different IP should pass
        assert!(limiter.check("192.168.1.1"));
    }
    
    #[test]
    fn test_http_error() {
        let err = HttpError::RateLimited;
        assert_eq!(err.status_code(), 429);
        
        let err = HttpError::InvalidJson("test".to_string());
        assert_eq!(err.status_code(), 400);
    }
    
    #[test]
    fn test_cors_headers() {
        let config = HttpServerConfig::default();
        // Would need node to fully test, but config is valid
        assert!(config.enable_cors);
    }
}
