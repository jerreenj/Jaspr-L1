//! Faucet service for distributing testnet JASPR tokens
//!
//! Provides rate-limited token distribution for testing purposes.

use jaspr_types::{Address, Amount, Transaction, TransactionPayload, SignedTransaction};
use jaspr_crypto::{KeyPair, sign_transaction, Signer};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug};

/// Faucet configuration
#[derive(Clone, Debug)]
pub struct FaucetConfig {
    /// Amount to dispense per request (in base units)
    pub drip_amount: Amount,
    /// Cooldown period between requests from same address
    pub cooldown_period: Duration,
    /// Maximum drips per day per IP
    pub max_daily_drips: u32,
    /// Whether faucet is enabled
    pub enabled: bool,
    /// Chain ID
    pub chain_id: u64,
}

impl Default for FaucetConfig {
    fn default() -> Self {
        Self {
            drip_amount: 100_000_000_000, // 100 JASPR
            cooldown_period: Duration::from_secs(3600), // 1 hour
            max_daily_drips: 5,
            enabled: true,
            chain_id: 1,
        }
    }
}

/// Faucet request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FaucetRequest {
    /// Recipient address
    pub address: String,
    /// Optional captcha token
    pub captcha_token: Option<String>,
}

/// Faucet response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FaucetResponse {
    /// Success status
    pub success: bool,
    /// Transaction hash (if successful)
    pub tx_hash: Option<String>,
    /// Amount sent (if successful)
    pub amount: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Cooldown remaining (seconds)
    pub cooldown_remaining: Option<u64>,
}

impl FaucetResponse {
    pub fn success(tx_hash: String, amount: String) -> Self {
        Self {
            success: true,
            tx_hash: Some(tx_hash),
            amount: Some(amount),
            error: None,
            cooldown_remaining: None,
        }
    }
    
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            tx_hash: None,
            amount: None,
            error: Some(message),
            cooldown_remaining: None,
        }
    }
    
    pub fn cooldown(remaining_secs: u64) -> Self {
        Self {
            success: false,
            tx_hash: None,
            amount: None,
            error: Some(format!("Please wait {} seconds before requesting again", remaining_secs)),
            cooldown_remaining: Some(remaining_secs),
        }
    }
}

/// Rate limit entry
#[derive(Clone, Debug)]
struct RateLimitEntry {
    /// Last request time
    last_request: Instant,
    /// Daily request count
    daily_count: u32,
    /// Day start time
    day_start: Instant,
}

impl RateLimitEntry {
    fn new() -> Self {
        Self {
            last_request: Instant::now(),
            daily_count: 1,
            day_start: Instant::now(),
        }
    }
    
    fn update(&mut self) {
        let now = Instant::now();
        
        // Reset daily count if a day has passed
        if now.duration_since(self.day_start) > Duration::from_secs(86400) {
            self.daily_count = 0;
            self.day_start = now;
        }
        
        self.last_request = now;
        self.daily_count += 1;
    }
}

/// Faucet service
pub struct FaucetService {
    config: FaucetConfig,
    /// Faucet wallet keypair
    wallet: KeyPair,
    /// Rate limits by address
    address_limits: RwLock<HashMap<Address, RateLimitEntry>>,
    /// Rate limits by IP
    ip_limits: RwLock<HashMap<String, RateLimitEntry>>,
    /// Current nonce
    nonce: RwLock<u64>,
    /// Total dispensed
    total_dispensed: RwLock<Amount>,
    /// Total requests served
    total_requests: RwLock<u64>,
}

impl FaucetService {
    /// Create new faucet service
    pub fn new(config: FaucetConfig, wallet: KeyPair) -> Self {
        info!(
            address = %wallet.address(),
            drip_amount = config.drip_amount,
            "Faucet service initialized"
        );
        
        Self {
            config,
            wallet,
            address_limits: RwLock::new(HashMap::new()),
            ip_limits: RwLock::new(HashMap::new()),
            nonce: RwLock::new(0),
            total_dispensed: RwLock::new(0),
            total_requests: RwLock::new(0),
        }
    }
    
    /// Create with new random wallet
    pub fn with_random_wallet(config: FaucetConfig) -> Self {
        Self::new(config, KeyPair::generate())
    }
    
    /// Get faucet wallet address
    pub fn address(&self) -> Address {
        self.wallet.address()
    }
    
    /// Process faucet request
    pub fn process_request(
        &self,
        request: &FaucetRequest,
        client_ip: Option<&str>,
    ) -> Result<SignedTransaction, FaucetResponse> {
        if !self.config.enabled {
            return Err(FaucetResponse::error("Faucet is currently disabled".to_string()));
        }
        
        // Parse recipient address
        let recipient = match Address::from_hex(&request.address) {
            Ok(addr) => addr,
            Err(e) => return Err(FaucetResponse::error(format!("Invalid address: {}", e))),
        };
        
        // Check address rate limit
        if let Err(response) = self.check_address_limit(&recipient) {
            return Err(response);
        }
        
        // Check IP rate limit
        if let Some(ip) = client_ip {
            if let Err(response) = self.check_ip_limit(ip) {
                return Err(response);
            }
        }
        
        // Create and sign transaction
        let tx = self.create_drip_transaction(recipient);
        let signed = sign_transaction(&self.wallet, tx);
        
        // Update rate limits
        self.update_address_limit(&recipient);
        if let Some(ip) = client_ip {
            self.update_ip_limit(ip);
        }
        
        // Update stats
        *self.total_dispensed.write() += self.config.drip_amount;
        *self.total_requests.write() += 1;
        
        info!(
            recipient = %recipient,
            amount = self.config.drip_amount,
            tx_hash = %signed.hash(),
            "Faucet drip sent"
        );
        
        Ok(signed)
    }
    
    /// Check address rate limit
    fn check_address_limit(&self, address: &Address) -> Result<(), FaucetResponse> {
        let limits = self.address_limits.read();
        
        if let Some(entry) = limits.get(address) {
            let elapsed = entry.last_request.elapsed();
            
            if elapsed < self.config.cooldown_period {
                let remaining = self.config.cooldown_period - elapsed;
                return Err(FaucetResponse::cooldown(remaining.as_secs()));
            }
        }
        
        Ok(())
    }
    
    /// Check IP rate limit
    fn check_ip_limit(&self, ip: &str) -> Result<(), FaucetResponse> {
        let limits = self.ip_limits.read();
        
        if let Some(entry) = limits.get(ip) {
            // Check daily limit
            if entry.daily_count >= self.config.max_daily_drips {
                let remaining = Duration::from_secs(86400) - entry.day_start.elapsed();
                return Err(FaucetResponse::error(format!(
                    "Daily limit reached. Try again in {} hours",
                    remaining.as_secs() / 3600
                )));
            }
            
            // Check cooldown
            let elapsed = entry.last_request.elapsed();
            if elapsed < self.config.cooldown_period {
                let remaining = self.config.cooldown_period - elapsed;
                return Err(FaucetResponse::cooldown(remaining.as_secs()));
            }
        }
        
        Ok(())
    }
    
    /// Update address rate limit
    fn update_address_limit(&self, address: &Address) {
        let mut limits = self.address_limits.write();
        
        if let Some(entry) = limits.get_mut(address) {
            entry.update();
        } else {
            limits.insert(*address, RateLimitEntry::new());
        }
    }
    
    /// Update IP rate limit
    fn update_ip_limit(&self, ip: &str) {
        let mut limits = self.ip_limits.write();
        
        if let Some(entry) = limits.get_mut(ip) {
            entry.update();
        } else {
            limits.insert(ip.to_string(), RateLimitEntry::new());
        }
    }
    
    /// Create drip transaction
    fn create_drip_transaction(&self, recipient: Address) -> Transaction {
        let nonce = {
            let mut n = self.nonce.write();
            let current = *n;
            *n += 1;
            current
        };
        
        Transaction::new(
            self.wallet.address(),
            TransactionPayload::Transfer {
                recipient,
                amount: self.config.drip_amount,
            },
            nonce,
            50000, // gas limit
            100,   // gas price
            self.config.chain_id,
        )
    }
    
    /// Get faucet statistics
    pub fn stats(&self) -> FaucetStats {
        FaucetStats {
            enabled: self.config.enabled,
            drip_amount: self.config.drip_amount,
            cooldown_seconds: self.config.cooldown_period.as_secs(),
            total_dispensed: *self.total_dispensed.read(),
            total_requests: *self.total_requests.read(),
            unique_addresses: self.address_limits.read().len(),
            unique_ips: self.ip_limits.read().len(),
        }
    }
    
    /// Clean up expired rate limit entries
    pub fn cleanup_expired(&self) {
        let expiry = Duration::from_secs(86400 * 2); // 2 days
        
        {
            let mut limits = self.address_limits.write();
            limits.retain(|_, entry| entry.last_request.elapsed() < expiry);
        }
        
        {
            let mut limits = self.ip_limits.write();
            limits.retain(|_, entry| entry.last_request.elapsed() < expiry);
        }
    }
    
    /// Set faucet enabled/disabled
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
        info!(enabled = enabled, "Faucet status changed");
    }
    
    /// Update drip amount
    pub fn set_drip_amount(&mut self, amount: Amount) {
        self.config.drip_amount = amount;
        info!(amount = amount, "Faucet drip amount updated");
    }
}

/// Faucet statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FaucetStats {
    pub enabled: bool,
    pub drip_amount: Amount,
    pub cooldown_seconds: u64,
    pub total_dispensed: Amount,
    pub total_requests: u64,
    pub unique_addresses: usize,
    pub unique_ips: usize,
}

/// Faucet fund request (for admin)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FaucetFundRequest {
    pub amount: String,
}

/// Multi-drip request for batch operations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiDripRequest {
    pub addresses: Vec<String>,
    pub amount_each: Option<String>,
}

/// Multi-drip response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiDripResponse {
    pub successful: Vec<String>,
    pub failed: Vec<(String, String)>,
    pub total_sent: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_faucet() -> FaucetService {
        let config = FaucetConfig {
            cooldown_period: Duration::from_secs(1),
            ..Default::default()
        };
        FaucetService::with_random_wallet(config)
    }
    
    #[test]
    fn test_faucet_drip() {
        let faucet = create_test_faucet();
        
        let request = FaucetRequest {
            address: "0x1234567890123456789012345678901234567890123456789012345678901234".to_string(),
            captcha_token: None,
        };
        
        let result = faucet.process_request(&request, Some("127.0.0.1"));
        assert!(result.is_ok());
        
        let stats = faucet.stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.total_dispensed, faucet.config.drip_amount);
    }
    
    #[test]
    fn test_rate_limiting() {
        let faucet = create_test_faucet();
        
        let request = FaucetRequest {
            address: "0x1234567890123456789012345678901234567890123456789012345678901234".to_string(),
            captcha_token: None,
        };
        
        // First request should succeed
        assert!(faucet.process_request(&request, Some("127.0.0.1")).is_ok());
        
        // Second immediate request should fail (cooldown)
        let result = faucet.process_request(&request, Some("127.0.0.1"));
        assert!(result.is_err());
    }
    
    #[test]
    fn test_invalid_address() {
        let faucet = create_test_faucet();
        
        let request = FaucetRequest {
            address: "invalid_address".to_string(),
            captcha_token: None,
        };
        
        let result = faucet.process_request(&request, None);
        assert!(result.is_err());
    }
}
