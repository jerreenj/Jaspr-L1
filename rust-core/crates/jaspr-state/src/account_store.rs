//! Account state storage

use crate::db::{Database, DbResult, CF_ACCOUNTS, CF_VALIDATORS};
use jaspr_types::{Address, Account, ValidatorInfo, Amount};
use borsh::BorshDeserialize;
use std::collections::HashMap;
use parking_lot::RwLock;

/// Account store for managing account state
pub struct AccountStore {
    db: Database,
    /// In-memory cache for hot accounts
    cache: RwLock<HashMap<Address, Account>>,
}

impl AccountStore {
    /// Create new account store
    pub fn new(db: Database) -> Self {
        Self {
            db,
            cache: RwLock::new(HashMap::new()),
        }
    }
    
    /// Get account by address
    pub fn get_account(&self, address: &Address) -> DbResult<Option<Account>> {
        // Check cache first
        {
            let cache = self.cache.read();
            if let Some(account) = cache.get(address) {
                return Ok(Some(account.clone()));
            }
        }
        
        // Load from DB
        let key = address.as_bytes();
        match self.db.get_cf(CF_ACCOUNTS, key)? {
            Some(data) => {
                let account: Account = borsh::from_slice(&data)
                    .map_err(|e| crate::db::DbError::Serialization(e.to_string()))?;
                
                // Cache it
                self.cache.write().insert(*address, account.clone());
                
                Ok(Some(account))
            }
            None => Ok(None),
        }
    }
    
    /// Get account or create empty one
    pub fn get_or_create(&self, address: &Address) -> DbResult<Account> {
        match self.get_account(address)? {
            Some(account) => Ok(account),
            None => Ok(Account::empty(*address)),
        }
    }
    
    /// Put account
    pub fn put_account(&self, account: &Account) -> DbResult<()> {
        let key = account.address.as_bytes();
        let data = borsh::to_vec(account)
            .map_err(|e| crate::db::DbError::Serialization(e.to_string()))?;
        
        self.db.put_cf(CF_ACCOUNTS, key, &data)?;
        
        // Update cache
        self.cache.write().insert(account.address, account.clone());
        
        Ok(())
    }
    
    /// Delete account
    pub fn delete_account(&self, address: &Address) -> DbResult<()> {
        let key = address.as_bytes();
        self.db.delete_cf(CF_ACCOUNTS, key)?;
        self.cache.write().remove(address);
        Ok(())
    }
    
    /// Get balance
    pub fn get_balance(&self, address: &Address) -> DbResult<Amount> {
        Ok(self.get_account(address)?
            .map(|a| a.balance())
            .unwrap_or(0))
    }
    
    /// Transfer tokens between accounts
    pub fn transfer(
        &self,
        from: &Address,
        to: &Address,
        amount: Amount,
    ) -> DbResult<()> {
        let mut sender = self.get_or_create(from)?;
        let mut recipient = self.get_or_create(to)?;
        
        sender.debit(amount)
            .map_err(|e| crate::db::DbError::Serialization(e))?;
        recipient.credit(amount);
        
        self.put_account(&sender)?;
        self.put_account(&recipient)?;
        
        Ok(())
    }
    
    /// Get validator info
    pub fn get_validator(&self, address: &Address) -> DbResult<Option<ValidatorInfo>> {
        let key = address.as_bytes();
        match self.db.get_cf(CF_VALIDATORS, key)? {
            Some(data) => {
                let info: ValidatorInfo = borsh::from_slice(&data)
                    .map_err(|e| crate::db::DbError::Serialization(e.to_string()))?;
                Ok(Some(info))
            }
            None => Ok(None),
        }
    }
    
    /// Put validator info
    pub fn put_validator(&self, validator: &ValidatorInfo) -> DbResult<()> {
        let key = validator.address.as_bytes();
        let data = borsh::to_vec(validator)
            .map_err(|e| crate::db::DbError::Serialization(e.to_string()))?;
        self.db.put_cf(CF_VALIDATORS, key, &data)?;
        Ok(())
    }
    
    /// Get all validators
    pub fn get_all_validators(&self) -> DbResult<Vec<ValidatorInfo>> {
        let mut validators = Vec::new();
        
        let cf = self.db.raw().cf_handle(CF_VALIDATORS)
            .ok_or_else(|| crate::db::DbError::ColumnFamilyNotFound(CF_VALIDATORS.to_string()))?;
        
        let iter = self.db.raw().iterator_cf(&cf, rocksdb::IteratorMode::Start);
        
        for item in iter {
            let (_, value) = item.map_err(crate::db::DbError::RocksDb)?;
            let validator: ValidatorInfo = borsh::from_slice(&value)
                .map_err(|e| crate::db::DbError::Serialization(e.to_string()))?;
            validators.push(validator);
        }
        
        Ok(validators)
    }
    
    /// Clear cache (for testing)
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_store() -> AccountStore {
        let db = Database::open_in_memory().unwrap();
        AccountStore::new(db)
    }
    
    #[test]
    fn test_account_crud() {
        let store = create_test_store();
        let address = Address::from_public_key(b"test");
        
        // Create account
        let mut account = Account::new(address, 1000);
        store.put_account(&account).unwrap();
        
        // Read account
        let loaded = store.get_account(&address).unwrap().unwrap();
        assert_eq!(loaded.balance(), 1000);
        
        // Update account
        account.credit(500);
        store.put_account(&account).unwrap();
        
        store.clear_cache();
        let loaded = store.get_account(&address).unwrap().unwrap();
        assert_eq!(loaded.balance(), 1500);
    }
    
    #[test]
    fn test_transfer() {
        let store = create_test_store();
        let addr1 = Address::from_public_key(b"user1");
        let addr2 = Address::from_public_key(b"user2");
        
        // Setup sender with balance
        let sender = Account::new(addr1, 1000);
        store.put_account(&sender).unwrap();
        
        // Transfer
        store.transfer(&addr1, &addr2, 300).unwrap();
        
        assert_eq!(store.get_balance(&addr1).unwrap(), 700);
        assert_eq!(store.get_balance(&addr2).unwrap(), 300);
    }
    
    #[test]
    fn test_validator() {
        let store = create_test_store();
        let addr = Address::from_public_key(b"validator1");
        
        let validator = ValidatorInfo::new(addr, "Test Validator".to_string(), 10_000_000_000_000);
        store.put_validator(&validator).unwrap();
        
        let loaded = store.get_validator(&addr).unwrap().unwrap();
        assert_eq!(loaded.name, "Test Validator");
        assert_eq!(loaded.voting_power(), 10000);
    }
}
