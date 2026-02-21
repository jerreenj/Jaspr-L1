//! Account Abstraction implementation

use std::collections::HashMap;

/// Account Abstraction manager
pub struct AccountAbstraction {
    wallets: HashMap<String, AAWallet>,
}

impl AccountAbstraction {
    pub fn new() -> Self {
        Self {
            wallets: HashMap::new(),
        }
    }
    
    pub fn get_or_create(&mut self, address: &str) -> &AAWallet {
        self.wallets.entry(address.to_string())
            .or_insert_with(|| AAWallet::new(address))
    }
    
    pub fn validate_operation(
        &self,
        sender: &str,
        to: &str,
        amount: u64,
    ) -> (bool, String) {
        if let Some(wallet) = self.wallets.get(sender) {
            wallet.validate_transaction(amount, to)
        } else {
            (true, "OK".to_string())
        }
    }
}

impl Default for AccountAbstraction {
    fn default() -> Self {
        Self::new()
    }
}

/// AA Wallet with spending limits
pub struct AAWallet {
    pub address: String,
    pub daily_limit: u64,
    pub transaction_limit: u64,
    pub daily_spent: u64,
    pub blocked_addresses: Vec<String>,
}

impl AAWallet {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
            daily_limit: 10_000_000_000_000,
            transaction_limit: 1_000_000_000_000,
            daily_spent: 0,
            blocked_addresses: Vec::new(),
        }
    }
    
    pub fn validate_transaction(&self, amount: u64, recipient: &str) -> (bool, String) {
        if self.blocked_addresses.contains(&recipient.to_string()) {
            return (false, "Recipient blocked".to_string());
        }
        
        if amount > self.transaction_limit {
            return (false, "Transaction limit exceeded".to_string());
        }
        
        if self.daily_spent + amount > self.daily_limit {
            return (false, "Daily limit exceeded".to_string());
        }
        
        (true, "OK".to_string())
    }
}
