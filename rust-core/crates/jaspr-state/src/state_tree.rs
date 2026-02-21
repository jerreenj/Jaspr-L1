//! Merkle Patricia Trie for state management

use jaspr_types::{HashValue, Address};
use std::collections::HashMap;
use parking_lot::RwLock;

/// State tree node
#[derive(Clone, Debug)]
pub enum StateNode {
    /// Leaf node with value
    Leaf {
        key_end: Vec<u8>,
        value: Vec<u8>,
    },
    /// Branch node with children
    Branch {
        children: [Option<Box<StateNode>>; 16],
        value: Option<Vec<u8>>,
    },
    /// Extension node (compressed path)
    Extension {
        key_part: Vec<u8>,
        child: Box<StateNode>,
    },
}

/// Simplified state tree (in-memory)
/// For production, this would be a full Merkle Patricia Trie
pub struct StateTree {
    /// Root hash
    root: RwLock<HashValue>,
    /// State data (key -> value)
    data: RwLock<HashMap<Vec<u8>, Vec<u8>>>,
    /// Version
    version: RwLock<u64>,
}

impl StateTree {
    /// Create new empty state tree
    pub fn new() -> Self {
        Self {
            root: RwLock::new(HashValue::zero()),
            data: RwLock::new(HashMap::new()),
            version: RwLock::new(0),
        }
    }
    
    /// Get current root hash
    pub fn root_hash(&self) -> HashValue {
        *self.root.read()
    }
    
    /// Get current version
    pub fn version(&self) -> u64 {
        *self.version.read()
    }
    
    /// Get value by key
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.read().get(key).cloned()
    }
    
    /// Set value
    pub fn set(&self, key: Vec<u8>, value: Vec<u8>) {
        self.data.write().insert(key, value);
        self.update_root();
    }
    
    /// Delete key
    pub fn delete(&self, key: &[u8]) -> Option<Vec<u8>> {
        let result = self.data.write().remove(key);
        self.update_root();
        result
    }
    
    /// Get account state
    pub fn get_account_state(&self, address: &Address) -> Option<Vec<u8>> {
        let key = address.as_bytes().to_vec();
        self.get(&key)
    }
    
    /// Set account state
    pub fn set_account_state(&self, address: &Address, data: Vec<u8>) {
        let key = address.as_bytes().to_vec();
        self.set(key, data);
    }
    
    /// Get resource
    pub fn get_resource(&self, address: &Address, type_tag: &str) -> Option<Vec<u8>> {
        let key = format!("{}:{}", address.to_hex(), type_tag);
        self.get(key.as_bytes())
    }
    
    /// Set resource
    pub fn set_resource(&self, address: &Address, type_tag: &str, data: Vec<u8>) {
        let key = format!("{}:{}", address.to_hex(), type_tag);
        self.set(key.into_bytes(), data);
    }
    
    /// Delete resource
    pub fn delete_resource(&self, address: &Address, type_tag: &str) -> Option<Vec<u8>> {
        let key = format!("{}:{}", address.to_hex(), type_tag);
        self.delete(key.as_bytes())
    }
    
    /// Commit changes and increment version
    pub fn commit(&self) -> HashValue {
        let mut version = self.version.write();
        *version += 1;
        self.root_hash()
    }
    
    /// Update root hash based on current state
    fn update_root(&self) {
        let data = self.data.read();
        let mut hasher_input = Vec::new();
        
        // Sort keys for deterministic hashing
        let mut keys: Vec<_> = data.keys().collect();
        keys.sort();
        
        for key in keys {
            if let Some(value) = data.get(key) {
                hasher_input.extend_from_slice(key);
                hasher_input.extend_from_slice(value);
            }
        }
        
        let new_root = HashValue::sha256(&hasher_input);
        *self.root.write() = new_root;
    }
    
    /// Get all keys with prefix
    pub fn get_keys_with_prefix(&self, prefix: &[u8]) -> Vec<Vec<u8>> {
        self.data.read()
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect()
    }
    
    /// Clear all state
    pub fn clear(&self) {
        self.data.write().clear();
        *self.root.write() = HashValue::zero();
        *self.version.write() = 0;
    }
    
    /// Get state size
    pub fn size(&self) -> usize {
        self.data.read().len()
    }
}

impl Default for StateTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_operations() {
        let tree = StateTree::new();
        
        tree.set(b"key1".to_vec(), b"value1".to_vec());
        assert_eq!(tree.get(b"key1"), Some(b"value1".to_vec()));
        
        tree.delete(b"key1");
        assert_eq!(tree.get(b"key1"), None);
    }
    
    #[test]
    fn test_root_changes() {
        let tree = StateTree::new();
        
        let root1 = tree.root_hash();
        assert!(root1.is_zero());
        
        tree.set(b"key".to_vec(), b"value".to_vec());
        let root2 = tree.root_hash();
        assert!(!root2.is_zero());
        assert_ne!(root1, root2);
        
        tree.set(b"key2".to_vec(), b"value2".to_vec());
        let root3 = tree.root_hash();
        assert_ne!(root2, root3);
    }
    
    #[test]
    fn test_commit() {
        let tree = StateTree::new();
        assert_eq!(tree.version(), 0);
        
        tree.commit();
        assert_eq!(tree.version(), 1);
        
        tree.commit();
        assert_eq!(tree.version(), 2);
    }
    
    #[test]
    fn test_account_state() {
        let tree = StateTree::new();
        let addr = Address::from_public_key(b"test");
        
        tree.set_account_state(&addr, b"account_data".to_vec());
        let data = tree.get_account_state(&addr).unwrap();
        assert_eq!(data, b"account_data".to_vec());
    }
    
    #[test]
    fn test_resources() {
        let tree = StateTree::new();
        let addr = Address::from_public_key(b"test");
        
        tree.set_resource(&addr, "0x1::Coin::Balance", b"balance_data".to_vec());
        let data = tree.get_resource(&addr, "0x1::Coin::Balance").unwrap();
        assert_eq!(data, b"balance_data".to_vec());
        
        tree.delete_resource(&addr, "0x1::Coin::Balance");
        assert!(tree.get_resource(&addr, "0x1::Coin::Balance").is_none());
    }
}
