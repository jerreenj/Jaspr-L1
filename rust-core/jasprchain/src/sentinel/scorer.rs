//! AI Sentinel risk scoring
//!
//! In production, this would integrate with ML inference
//! Here we provide hooks and heuristic-based scoring

use crate::execution::SignedTransaction;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum GuardMode {
    Passive,
    Warning,
    Enforced,
}

#[derive(Clone, Debug)]
pub struct RiskScore {
    pub tx_hash: String,
    pub score: f64,
    pub action: String,
    pub factors: Vec<String>,
}

/// AI-powered transaction security sentinel
pub struct AISentinel {
    guard_mode: GuardMode,
    known_scam_addresses: Vec<String>,
    score_cache: HashMap<String, RiskScore>,
    total_scanned: u64,
    threats_detected: u64,
}

impl AISentinel {
    pub const WARN_THRESHOLD: f64 = 40.0;
    pub const BLOCK_THRESHOLD: f64 = 75.0;
    
    pub fn new() -> Self {
        Self {
            guard_mode: GuardMode::Enforced,
            known_scam_addresses: Vec::new(),
            score_cache: HashMap::new(),
            total_scanned: 0,
            threats_detected: 0,
        }
    }
    
    /// Scan a transaction for risks
    pub fn scan(&mut self, tx: &SignedTransaction) -> RiskScore {
        let tx_hash = tx.hash();
        
        // Check cache
        if let Some(cached) = self.score_cache.get(&tx_hash) {
            return cached.clone();
        }
        
        self.total_scanned += 1;
        
        // Heuristic scoring
        let mut score = 0.0;
        let mut factors = Vec::new();
        
        let inner = &tx.transaction;
        
        // Check known scam addresses
        if let Some(recipient) = &inner.recipient {
            if self.known_scam_addresses.contains(recipient) {
                score += 100.0;
                factors.push("Known scam address".to_string());
            }
        }
        
        // Large amount from low nonce account
        if inner.nonce < 5 && inner.amount > 1_000_000_000_000 {
            score += 30.0;
            factors.push("Large amount from new account".to_string());
        }
        
        // Unusual gas price
        if inner.gas_price > 50_000_000_000 {
            score += 15.0;
            factors.push("Unusual gas price".to_string());
        }
        
        // Determine action
        let action = match self.guard_mode {
            GuardMode::Passive => "allow".to_string(),
            GuardMode::Warning => {
                if score >= Self::BLOCK_THRESHOLD {
                    "warn".to_string()
                } else {
                    "allow".to_string()
                }
            }
            GuardMode::Enforced => {
                if score >= Self::BLOCK_THRESHOLD {
                    self.threats_detected += 1;
                    "block".to_string()
                } else if score >= Self::WARN_THRESHOLD {
                    "warn".to_string()
                } else {
                    "allow".to_string()
                }
            }
        };
        
        let result = RiskScore {
            tx_hash: tx_hash.clone(),
            score,
            action,
            factors,
        };
        
        self.score_cache.insert(tx_hash, result.clone());
        result
    }
    
    pub fn add_scam_address(&mut self, address: &str) {
        self.known_scam_addresses.push(address.to_string());
    }
    
    pub fn set_guard_mode(&mut self, mode: GuardMode) {
        self.guard_mode = mode;
    }
    
    pub fn stats(&self) -> (u64, u64) {
        (self.total_scanned, self.threats_detected)
    }
}

impl Default for AISentinel {
    fn default() -> Self {
        Self::new()
    }
}
