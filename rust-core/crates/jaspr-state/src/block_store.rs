//! Block storage

use crate::db::{Database, DbResult, CF_BLOCKS, CF_TRANSACTIONS, CF_RECEIPTS, CF_METADATA};
use jaspr_types::{Block, BlockHeight, HashValue, SignedTransaction, TransactionReceipt};
use borsh::{BorshSerialize, BorshDeserialize};
use parking_lot::RwLock;

/// Key for latest block height
const KEY_LATEST_HEIGHT: &[u8] = b"latest_height";

/// Block store for managing blocks and transactions
pub struct BlockStore {
    db: Database,
    /// Cached latest height
    latest_height: RwLock<Option<BlockHeight>>,
}

impl BlockStore {
    /// Create new block store
    pub fn new(db: Database) -> Self {
        Self {
            db,
            latest_height: RwLock::new(None),
        }
    }
    
    /// Initialize genesis block if not exists
    pub fn init_genesis(&self, chain_id: u64) -> DbResult<Block> {
        if let Some(genesis) = self.get_block_by_height(0)? {
            return Ok(genesis);
        }
        
        let genesis = Block::genesis(chain_id);
        self.put_block(&genesis)?;
        Ok(genesis)
    }
    
    /// Get block by height
    pub fn get_block_by_height(&self, height: BlockHeight) -> DbResult<Option<Block>> {
        let key = height.to_le_bytes();
        match self.db.get_cf(CF_BLOCKS, &key)? {
            Some(data) => {
                let block: Block = borsh::from_slice(&data)
                    .map_err(|e| crate::db::DbError::Serialization(e.to_string()))?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }
    
    /// Get block by hash
    pub fn get_block_by_hash(&self, hash: &HashValue) -> DbResult<Option<Block>> {
        // First get height from hash index
        let hash_key = hash.as_bytes();
        match self.db.get_cf(CF_METADATA, hash_key)? {
            Some(height_bytes) => {
                if height_bytes.len() != 8 {
                    return Ok(None);
                }
                let height = u64::from_le_bytes(height_bytes.try_into().unwrap());
                self.get_block_by_height(height)
            }
            None => Ok(None),
        }
    }
    
    /// Put block
    pub fn put_block(&self, block: &Block) -> DbResult<()> {
        let height = block.height();
        let hash = block.hash();
        
        // Store block by height
        let height_key = height.to_le_bytes();
        let data = borsh::to_vec(block)
            .map_err(|e| crate::db::DbError::Serialization(e.to_string()))?;
        self.db.put_cf(CF_BLOCKS, &height_key, &data)?;
        
        // Index by hash
        let hash_key = hash.as_bytes();
        self.db.put_cf(CF_METADATA, hash_key, &height_key)?;
        
        // Store transactions
        for (idx, tx) in block.body.transactions.iter().enumerate() {
            self.put_transaction(tx, height, idx as u32)?;
        }
        
        // Update latest height
        self.update_latest_height(height)?;
        
        Ok(())
    }
    
    /// Get latest block height
    pub fn get_latest_height(&self) -> DbResult<Option<BlockHeight>> {
        // Check cache
        {
            let cache = self.latest_height.read();
            if cache.is_some() {
                return Ok(*cache);
            }
        }
        
        // Load from DB
        match self.db.get_cf(CF_METADATA, KEY_LATEST_HEIGHT)? {
            Some(bytes) => {
                if bytes.len() != 8 {
                    return Ok(None);
                }
                let height = u64::from_le_bytes(bytes.try_into().unwrap());
                *self.latest_height.write() = Some(height);
                Ok(Some(height))
            }
            None => Ok(None),
        }
    }
    
    /// Get latest block
    pub fn get_latest_block(&self) -> DbResult<Option<Block>> {
        match self.get_latest_height()? {
            Some(height) => self.get_block_by_height(height),
            None => Ok(None),
        }
    }
    
    /// Update latest height
    fn update_latest_height(&self, height: BlockHeight) -> DbResult<()> {
        let bytes = height.to_le_bytes();
        self.db.put_cf(CF_METADATA, KEY_LATEST_HEIGHT, &bytes)?;
        *self.latest_height.write() = Some(height);
        Ok(())
    }
    
    /// Get blocks in range (inclusive)
    pub fn get_blocks_range(&self, start: BlockHeight, end: BlockHeight) -> DbResult<Vec<Block>> {
        let mut blocks = Vec::new();
        for height in start..=end {
            if let Some(block) = self.get_block_by_height(height)? {
                blocks.push(block);
            }
        }
        Ok(blocks)
    }
    
    /// Put transaction
    fn put_transaction(&self, tx: &SignedTransaction, block_height: BlockHeight, tx_index: u32) -> DbResult<()> {
        let hash = tx.hash();
        let key = hash.as_bytes();
        
        #[derive(BorshSerialize, BorshDeserialize)]
        struct TxRecord {
            tx: SignedTransaction,
            block_height: BlockHeight,
            tx_index: u32,
        }
        
        let record = TxRecord {
            tx: tx.clone(),
            block_height,
            tx_index,
        };
        
        let data = borsh::to_vec(&record)
            .map_err(|e| crate::db::DbError::Serialization(e.to_string()))?;
        
        self.db.put_cf(CF_TRANSACTIONS, key, &data)?;
        Ok(())
    }
    
    /// Get transaction by hash
    pub fn get_transaction(&self, hash: &HashValue) -> DbResult<Option<(SignedTransaction, BlockHeight, u32)>> {
        let key = hash.as_bytes();
        
        #[derive(BorshSerialize, BorshDeserialize)]
        struct TxRecord {
            tx: SignedTransaction,
            block_height: BlockHeight,
            tx_index: u32,
        }
        
        match self.db.get_cf(CF_TRANSACTIONS, key)? {
            Some(data) => {
                let record: TxRecord = borsh::from_slice(&data)
                    .map_err(|e| crate::db::DbError::Serialization(e.to_string()))?;
                Ok(Some((record.tx, record.block_height, record.tx_index)))
            }
            None => Ok(None),
        }
    }
    
    /// Put receipt
    pub fn put_receipt(&self, receipt: &TransactionReceipt) -> DbResult<()> {
        let key = receipt.tx_hash.as_bytes();
        let data = borsh::to_vec(receipt)
            .map_err(|e| crate::db::DbError::Serialization(e.to_string()))?;
        self.db.put_cf(CF_RECEIPTS, key, &data)?;
        Ok(())
    }
    
    /// Get receipt by transaction hash
    pub fn get_receipt(&self, tx_hash: &HashValue) -> DbResult<Option<TransactionReceipt>> {
        let key = tx_hash.as_bytes();
        match self.db.get_cf(CF_RECEIPTS, key)? {
            Some(data) => {
                let receipt: TransactionReceipt = borsh::from_slice(&data)
                    .map_err(|e| crate::db::DbError::Serialization(e.to_string()))?;
                Ok(Some(receipt))
            }
            None => Ok(None),
        }
    }
    
    /// Get block count
    pub fn block_count(&self) -> DbResult<u64> {
        Ok(self.get_latest_height()?.map(|h| h + 1).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_store() -> BlockStore {
        let db = Database::open_in_memory().unwrap();
        BlockStore::new(db)
    }
    
    #[test]
    fn test_genesis() {
        let store = create_test_store();
        
        let genesis = store.init_genesis(1).unwrap();
        assert_eq!(genesis.height(), 0);
        assert!(genesis.finalized);
        
        // Should return same genesis on second call
        let genesis2 = store.init_genesis(1).unwrap();
        assert_eq!(genesis.hash(), genesis2.hash());
    }
    
    #[test]
    fn test_block_storage() {
        let store = create_test_store();
        
        let genesis = store.init_genesis(1).unwrap();
        
        // Get by height
        let loaded = store.get_block_by_height(0).unwrap().unwrap();
        assert_eq!(loaded.hash(), genesis.hash());
        
        // Get by hash
        let loaded = store.get_block_by_hash(&genesis.hash()).unwrap().unwrap();
        assert_eq!(loaded.height(), 0);
        
        // Latest height
        assert_eq!(store.get_latest_height().unwrap(), Some(0));
    }
    
    #[test]
    fn test_receipt() {
        let store = create_test_store();
        
        let receipt = TransactionReceipt::success(
            HashValue::sha256(b"tx1"),
            1,
            0,
            50000,
        );
        
        store.put_receipt(&receipt).unwrap();
        
        let loaded = store.get_receipt(&receipt.tx_hash).unwrap().unwrap();
        assert!(loaded.is_success());
        assert_eq!(loaded.gas_used, 50000);
    }
}
