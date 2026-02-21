//! Wallet and account management utilities
//!
//! Provides wallet creation, key management, and transaction signing

use jaspr_types::{Address, Transaction, SignedTransaction, TransactionPayload, Amount};
use jaspr_crypto::{KeyPair, PrivateKey, PublicKey, sign_transaction, Signer};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug};

/// Wallet configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletConfig {
    /// Wallet name
    pub name: String,
    /// Key derivation path
    pub derivation_path: String,
    /// Network (mainnet, testnet, devnet)
    pub network: String,
    /// Chain ID
    pub chain_id: u64,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            derivation_path: "m/44'/637'/0'/0'/0'".to_string(),
            network: "mainnet".to_string(),
            chain_id: 1,
        }
    }
}

/// Account in wallet
#[derive(Clone)]
pub struct WalletAccount {
    /// Account name/label
    pub name: String,
    /// Address
    pub address: Address,
    /// Public key
    pub public_key: PublicKey,
    /// Private key (encrypted in production)
    keypair: KeyPair,
    /// Account index
    pub index: u32,
    /// Is default account
    pub is_default: bool,
}

impl std::fmt::Debug for WalletAccount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalletAccount")
            .field("name", &self.name)
            .field("address", &self.address)
            .field("public_key", &self.public_key)
            .field("index", &self.index)
            .field("is_default", &self.is_default)
            .field("keypair", &"[HIDDEN]")
            .finish()
    }
}

impl WalletAccount {
    /// Create new account from keypair
    pub fn new(name: String, keypair: KeyPair, index: u32) -> Self {
        Self {
            name,
            address: keypair.address(),
            public_key: *keypair.public_key(),
            keypair,
            index,
            is_default: false,
        }
    }
    
    /// Sign a transaction
    pub fn sign_transaction(&self, tx: Transaction) -> SignedTransaction {
        sign_transaction(&self.keypair, tx)
    }
    
    /// Sign arbitrary message
    pub fn sign_message(&self, message: &[u8]) -> Vec<u8> {
        self.keypair.sign(message).to_vec()
    }
    
    /// Get private key hex (careful!)
    pub fn export_private_key(&self) -> String {
        self.keypair.export_private_hex()
    }
}

/// HD Wallet for managing multiple accounts
pub struct HdWallet {
    config: WalletConfig,
    /// Accounts by address
    accounts: RwLock<HashMap<Address, WalletAccount>>,
    /// Account order
    account_order: RwLock<Vec<Address>>,
    /// Next account index
    next_index: RwLock<u32>,
}

impl HdWallet {
    /// Create new HD wallet
    pub fn new(config: WalletConfig) -> Self {
        Self {
            config,
            accounts: RwLock::new(HashMap::new()),
            account_order: RwLock::new(Vec::new()),
            next_index: RwLock::new(0),
        }
    }
    
    /// Create wallet with initial account
    pub fn create() -> Self {
        let mut wallet = Self::new(WalletConfig::default());
        wallet.create_account("Default".to_string());
        wallet
    }
    
    /// Create new account
    pub fn create_account(&mut self, name: String) -> Address {
        let keypair = KeyPair::generate();
        let index = {
            let mut idx = self.next_index.write();
            let current = *idx;
            *idx += 1;
            current
        };
        
        let mut account = WalletAccount::new(name.clone(), keypair, index);
        
        // First account is default
        if index == 0 {
            account.is_default = true;
        }
        
        let address = account.address;
        
        self.accounts.write().insert(address, account);
        self.account_order.write().push(address);
        
        info!(address = %address, name = %name, "Created new account");
        
        address
    }
    
    /// Import account from private key
    pub fn import_account(&mut self, name: String, private_key_hex: &str) -> Result<Address, String> {
        let private_key = PrivateKey::from_hex(private_key_hex)
            .map_err(|e| format!("Invalid private key: {}", e))?;
        
        let keypair = KeyPair::from_private_key(private_key);
        let address = keypair.address();
        
        // Check if already exists
        if self.accounts.read().contains_key(&address) {
            return Err("Account already exists".to_string());
        }
        
        let index = {
            let mut idx = self.next_index.write();
            let current = *idx;
            *idx += 1;
            current
        };
        
        let account = WalletAccount::new(name.clone(), keypair, index);
        
        self.accounts.write().insert(address, account);
        self.account_order.write().push(address);
        
        info!(address = %address, name = %name, "Imported account");
        
        Ok(address)
    }
    
    /// Get account by address
    pub fn get_account(&self, address: &Address) -> Option<WalletAccount> {
        self.accounts.read().get(address).cloned()
    }
    
    /// Get default account
    pub fn default_account(&self) -> Option<WalletAccount> {
        self.accounts.read()
            .values()
            .find(|a| a.is_default)
            .cloned()
    }
    
    /// Set default account
    pub fn set_default(&self, address: &Address) -> Result<(), String> {
        let mut accounts = self.accounts.write();
        
        if !accounts.contains_key(address) {
            return Err("Account not found".to_string());
        }
        
        // Clear previous default
        for account in accounts.values_mut() {
            account.is_default = false;
        }
        
        // Set new default
        if let Some(account) = accounts.get_mut(address) {
            account.is_default = true;
        }
        
        Ok(())
    }
    
    /// List all accounts
    pub fn list_accounts(&self) -> Vec<WalletAccount> {
        let accounts = self.accounts.read();
        let order = self.account_order.read();
        
        order.iter()
            .filter_map(|addr| accounts.get(addr).cloned())
            .collect()
    }
    
    /// Remove account
    pub fn remove_account(&self, address: &Address) -> Result<(), String> {
        let mut accounts = self.accounts.write();
        
        if !accounts.contains_key(address) {
            return Err("Account not found".to_string());
        }
        
        let was_default = accounts.get(address).map(|a| a.is_default).unwrap_or(false);
        
        accounts.remove(address);
        self.account_order.write().retain(|a| a != address);
        
        // If removed default, set new default
        if was_default {
            if let Some(first) = accounts.values_mut().next() {
                first.is_default = true;
            }
        }
        
        Ok(())
    }
    
    /// Sign transaction with account
    pub fn sign_transaction(
        &self,
        address: &Address,
        tx: Transaction,
    ) -> Result<SignedTransaction, String> {
        let accounts = self.accounts.read();
        
        let account = accounts.get(address)
            .ok_or_else(|| "Account not found".to_string())?;
        
        Ok(account.sign_transaction(tx))
    }
    
    /// Create and sign transfer transaction
    pub fn create_transfer(
        &self,
        from: &Address,
        to: &Address,
        amount: Amount,
        nonce: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<SignedTransaction, String> {
        let tx = Transaction::new(
            *from,
            TransactionPayload::Transfer {
                recipient: *to,
                amount,
            },
            nonce,
            gas_limit,
            gas_price,
            self.config.chain_id,
        );
        
        self.sign_transaction(from, tx)
    }
    
    /// Account count
    pub fn account_count(&self) -> usize {
        self.accounts.read().len()
    }
    
    /// Get wallet config
    pub fn config(&self) -> &WalletConfig {
        &self.config
    }
}

/// Transaction builder for constructing complex transactions
pub struct TransactionBuilder {
    sender: Option<Address>,
    payload: Option<TransactionPayload>,
    nonce: Option<u64>,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u64,
}

impl TransactionBuilder {
    /// Create new transaction builder
    pub fn new() -> Self {
        Self {
            sender: None,
            payload: None,
            nonce: None,
            gas_limit: 100_000,
            gas_price: 100,
            chain_id: 1,
        }
    }
    
    /// Set sender
    pub fn sender(mut self, address: Address) -> Self {
        self.sender = Some(address);
        self
    }
    
    /// Set transfer payload
    pub fn transfer(mut self, recipient: Address, amount: Amount) -> Self {
        self.payload = Some(TransactionPayload::Transfer { recipient, amount });
        self
    }
    
    /// Set stake payload
    pub fn stake(mut self, validator: Address, amount: Amount) -> Self {
        self.payload = Some(TransactionPayload::Stake { validator, amount });
        self
    }
    
    /// Set unstake payload
    pub fn unstake(mut self, validator: Address, amount: Amount) -> Self {
        self.payload = Some(TransactionPayload::Unstake { validator, amount });
        self
    }
    
    /// Set module deploy payload
    pub fn deploy_module(mut self, bytecode: Vec<u8>) -> Self {
        self.payload = Some(TransactionPayload::ModuleDeploy {
            bytecode,
            abi: String::new(),
        });
        self
    }
    
    /// Set script call payload
    pub fn call(
        mut self,
        module_address: Address,
        module_name: String,
        function_name: String,
        type_args: Vec<String>,
        args: Vec<Vec<u8>>,
    ) -> Self {
        self.payload = Some(TransactionPayload::ScriptCall {
            module_address,
            module_name,
            function_name,
            type_args,
            args,
        });
        self
    }
    
    /// Set nonce
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }
    
    /// Set gas limit
    pub fn gas_limit(mut self, limit: u64) -> Self {
        self.gas_limit = limit;
        self
    }
    
    /// Set gas price
    pub fn gas_price(mut self, price: u64) -> Self {
        self.gas_price = price;
        self
    }
    
    /// Set chain ID
    pub fn chain_id(mut self, id: u64) -> Self {
        self.chain_id = id;
        self
    }
    
    /// Build transaction
    pub fn build(self) -> Result<Transaction, String> {
        let sender = self.sender.ok_or("Sender not set")?;
        let payload = self.payload.ok_or("Payload not set")?;
        let nonce = self.nonce.ok_or("Nonce not set")?;
        
        Ok(Transaction::new(
            sender,
            payload,
            nonce,
            self.gas_limit,
            self.gas_price,
            self.chain_id,
        ))
    }
    
    /// Build and sign transaction
    pub fn build_and_sign(self, wallet: &HdWallet) -> Result<SignedTransaction, String> {
        let sender = self.sender.ok_or("Sender not set")?;
        let tx = self.build()?;
        wallet.sign_transaction(&sender, tx)
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Account balance tracker
pub struct BalanceTracker {
    /// Cached balances
    balances: RwLock<HashMap<Address, Amount>>,
    /// Last update times
    last_updated: RwLock<HashMap<Address, u64>>,
}

impl BalanceTracker {
    /// Create new balance tracker
    pub fn new() -> Self {
        Self {
            balances: RwLock::new(HashMap::new()),
            last_updated: RwLock::new(HashMap::new()),
        }
    }
    
    /// Update balance
    pub fn update(&self, address: Address, balance: Amount) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.balances.write().insert(address, balance);
        self.last_updated.write().insert(address, now);
    }
    
    /// Get cached balance
    pub fn get(&self, address: &Address) -> Option<Amount> {
        self.balances.read().get(address).copied()
    }
    
    /// Get last update time
    pub fn last_updated(&self, address: &Address) -> Option<u64> {
        self.last_updated.read().get(address).copied()
    }
    
    /// Check if balance is stale (older than given seconds)
    pub fn is_stale(&self, address: &Address, max_age_secs: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.last_updated.read()
            .get(address)
            .map(|t| now - t > max_age_secs)
            .unwrap_or(true)
    }
    
    /// Clear all cached balances
    pub fn clear(&self) {
        self.balances.write().clear();
        self.last_updated.write().clear();
    }
}

impl Default for BalanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_wallet() {
        let wallet = HdWallet::create();
        assert_eq!(wallet.account_count(), 1);
        
        let default = wallet.default_account().unwrap();
        assert!(default.is_default);
    }
    
    #[test]
    fn test_create_accounts() {
        let mut wallet = HdWallet::new(WalletConfig::default());
        
        let addr1 = wallet.create_account("Account 1".to_string());
        let addr2 = wallet.create_account("Account 2".to_string());
        
        assert_eq!(wallet.account_count(), 2);
        assert_ne!(addr1, addr2);
    }
    
    #[test]
    fn test_import_account() {
        let mut wallet = HdWallet::new(WalletConfig::default());
        
        // Generate a key to import
        let keypair = KeyPair::generate();
        let private_hex = keypair.export_private_hex();
        let expected_addr = keypair.address();
        
        let imported_addr = wallet.import_account("Imported".to_string(), &private_hex).unwrap();
        
        assert_eq!(imported_addr, expected_addr);
    }
    
    #[test]
    fn test_transaction_builder() {
        let mut wallet = HdWallet::create();
        let sender = wallet.default_account().unwrap().address;
        let recipient = Address::from_public_key(b"recipient");
        
        let tx = TransactionBuilder::new()
            .sender(sender)
            .transfer(recipient, 1000)
            .nonce(0)
            .gas_limit(50000)
            .gas_price(100)
            .chain_id(1)
            .build()
            .unwrap();
        
        assert_eq!(tx.sender, sender);
        assert_eq!(tx.nonce, 0);
    }
    
    #[test]
    fn test_balance_tracker() {
        let tracker = BalanceTracker::new();
        let addr = Address::from_public_key(b"test");
        
        tracker.update(addr, 1000);
        
        assert_eq!(tracker.get(&addr), Some(1000));
        assert!(!tracker.is_stale(&addr, 60));
    }
}
