//! RocksDB database wrapper

use rocksdb::{DB, Options, ColumnFamilyDescriptor, WriteBatch};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Database errors
#[derive(Error, Debug)]
pub enum DbError {
    #[error("RocksDB error: {0}")]
    RocksDb(#[from] rocksdb::Error),
    
    #[error("Key not found: {0}")]
    NotFound(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Column family not found: {0}")]
    ColumnFamilyNotFound(String),
}

pub type DbResult<T> = Result<T, DbError>;

/// Column families for organizing data
pub const CF_DEFAULT: &str = "default";
pub const CF_BLOCKS: &str = "blocks";
pub const CF_TRANSACTIONS: &str = "transactions";
pub const CF_ACCOUNTS: &str = "accounts";
pub const CF_RECEIPTS: &str = "receipts";
pub const CF_STATE: &str = "state";
pub const CF_VALIDATORS: &str = "validators";
pub const CF_METADATA: &str = "metadata";

/// Database configuration
#[derive(Clone, Debug)]
pub struct DatabaseConfig {
    pub path: String,
    pub create_if_missing: bool,
    pub max_open_files: i32,
    pub write_buffer_size: usize,
    pub max_write_buffer_number: i32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "./data/jasprchain".to_string(),
            create_if_missing: true,
            max_open_files: 1000,
            write_buffer_size: 64 * 1024 * 1024, // 64MB
            max_write_buffer_number: 3,
        }
    }
}

/// Main database wrapper
pub struct Database {
    db: Arc<DB>,
}

impl Database {
    /// Open database with configuration
    pub fn open(config: &DatabaseConfig) -> DbResult<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(config.create_if_missing);
        opts.create_missing_column_families(true);
        opts.set_max_open_files(config.max_open_files);
        opts.set_write_buffer_size(config.write_buffer_size);
        opts.set_max_write_buffer_number(config.max_write_buffer_number);
        
        let cf_names = vec![
            CF_BLOCKS,
            CF_TRANSACTIONS,
            CF_ACCOUNTS,
            CF_RECEIPTS,
            CF_STATE,
            CF_VALIDATORS,
            CF_METADATA,
        ];
        
        let cfs: Vec<ColumnFamilyDescriptor> = cf_names
            .iter()
            .map(|name| {
                let cf_opts = Options::default();
                ColumnFamilyDescriptor::new(*name, cf_opts)
            })
            .collect();
        
        let db = DB::open_cf_descriptors(&opts, &config.path, cfs)?;
        
        Ok(Self { db: Arc::new(db) })
    }
    
    /// Open in-memory database for testing
    pub fn open_in_memory() -> DbResult<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        
        let temp_dir = std::env::temp_dir().join(format!("jaspr_test_{}", rand::random::<u64>()));
        
        let cf_names = vec![
            CF_BLOCKS,
            CF_TRANSACTIONS,
            CF_ACCOUNTS,
            CF_RECEIPTS,
            CF_STATE,
            CF_VALIDATORS,
            CF_METADATA,
        ];
        
        let cfs: Vec<ColumnFamilyDescriptor> = cf_names
            .iter()
            .map(|name| ColumnFamilyDescriptor::new(*name, Options::default()))
            .collect();
        
        let db = DB::open_cf_descriptors(&opts, &temp_dir, cfs)?;
        
        Ok(Self { db: Arc::new(db) })
    }
    
    /// Get value from default column family
    pub fn get(&self, key: &[u8]) -> DbResult<Option<Vec<u8>>> {
        Ok(self.db.get(key)?)
    }
    
    /// Get value from specific column family
    pub fn get_cf(&self, cf: &str, key: &[u8]) -> DbResult<Option<Vec<u8>>> {
        let cf_handle = self.db.cf_handle(cf)
            .ok_or_else(|| DbError::ColumnFamilyNotFound(cf.to_string()))?;
        Ok(self.db.get_cf(&cf_handle, key)?)
    }
    
    /// Put value in default column family
    pub fn put(&self, key: &[u8], value: &[u8]) -> DbResult<()> {
        Ok(self.db.put(key, value)?)
    }
    
    /// Put value in specific column family
    pub fn put_cf(&self, cf: &str, key: &[u8], value: &[u8]) -> DbResult<()> {
        let cf_handle = self.db.cf_handle(cf)
            .ok_or_else(|| DbError::ColumnFamilyNotFound(cf.to_string()))?;
        Ok(self.db.put_cf(&cf_handle, key, value)?)
    }
    
    /// Delete key from default column family
    pub fn delete(&self, key: &[u8]) -> DbResult<()> {
        Ok(self.db.delete(key)?)
    }
    
    /// Delete key from specific column family
    pub fn delete_cf(&self, cf: &str, key: &[u8]) -> DbResult<()> {
        let cf_handle = self.db.cf_handle(cf)
            .ok_or_else(|| DbError::ColumnFamilyNotFound(cf.to_string()))?;
        Ok(self.db.delete_cf(&cf_handle, key)?)
    }
    
    /// Execute batch write
    pub fn write_batch(&self, batch: WriteBatch) -> DbResult<()> {
        Ok(self.db.write(batch)?)
    }
    
    /// Create new write batch
    pub fn new_batch(&self) -> WriteBatch {
        WriteBatch::default()
    }
    
    /// Get raw DB reference for advanced operations
    pub fn raw(&self) -> &DB {
        &self.db
    }
    
    /// Clone the Arc reference
    pub fn clone_arc(&self) -> Arc<DB> {
        Arc::clone(&self.db)
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_operations() {
        let db = Database::open_in_memory().unwrap();
        
        // Put and get
        db.put(b"key1", b"value1").unwrap();
        let val = db.get(b"key1").unwrap();
        assert_eq!(val, Some(b"value1".to_vec()));
        
        // Delete
        db.delete(b"key1").unwrap();
        let val = db.get(b"key1").unwrap();
        assert_eq!(val, None);
    }
    
    #[test]
    fn test_column_families() {
        let db = Database::open_in_memory().unwrap();
        
        // Put in accounts CF
        db.put_cf(CF_ACCOUNTS, b"addr1", b"account_data").unwrap();
        let val = db.get_cf(CF_ACCOUNTS, b"addr1").unwrap();
        assert_eq!(val, Some(b"account_data".to_vec()));
        
        // Different CF should not have the key
        let val = db.get_cf(CF_BLOCKS, b"addr1").unwrap();
        assert_eq!(val, None);
    }
}
