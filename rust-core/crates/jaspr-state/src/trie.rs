//! State Trie implementation using Sparse Merkle Tree
//!
//! Provides a cryptographically secure key-value store with:
//! - O(log n) proofs
//! - Efficient updates
//! - Historical state access
//! - Batch operations

use jaspr_types::{HashValue, Address, Amount};
use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn, debug};

/// State trie error
#[derive(Error, Debug)]
pub enum TrieError {
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Invalid proof")]
    InvalidProof,
    
    #[error("Corrupted node: {0}")]
    CorruptedNode(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Node in the sparse merkle tree
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TrieNode {
    /// Empty node
    Empty,
    /// Leaf node containing value
    Leaf {
        key: Vec<u8>,
        value: Vec<u8>,
        value_hash: HashValue,
    },
    /// Internal node with two children
    Internal {
        left: HashValue,
        right: HashValue,
    },
    /// Extension node (path compression)
    Extension {
        prefix: Vec<u8>,
        child: HashValue,
    },
}

impl TrieNode {
    /// Calculate node hash
    pub fn hash(&self) -> HashValue {
        match self {
            TrieNode::Empty => HashValue::zero(),
            TrieNode::Leaf { key, value_hash, .. } => {
                let mut data = key.clone();
                data.extend(value_hash.as_bytes());
                HashValue::sha256(&data)
            }
            TrieNode::Internal { left, right } => {
                let mut data = left.as_bytes().to_vec();
                data.extend(right.as_bytes());
                HashValue::sha256(&data)
            }
            TrieNode::Extension { prefix, child } => {
                let mut data = prefix.clone();
                data.extend(child.as_bytes());
                HashValue::sha256(&data)
            }
        }
    }
    
    /// Check if node is empty
    pub fn is_empty(&self) -> bool {
        matches!(self, TrieNode::Empty)
    }
}

/// Merkle proof for a key-value pair
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Key being proven
    pub key: Vec<u8>,
    /// Value (None if proving non-existence)
    pub value: Option<Vec<u8>>,
    /// Sibling hashes from leaf to root
    pub siblings: Vec<(bool, HashValue)>,
    /// Root hash
    pub root: HashValue,
}

impl MerkleProof {
    /// Verify the proof
    pub fn verify(&self) -> bool {
        let mut current_hash = if let Some(ref value) = self.value {
            let leaf = TrieNode::Leaf {
                key: self.key.clone(),
                value: value.clone(),
                value_hash: HashValue::sha256(value),
            };
            leaf.hash()
        } else {
            HashValue::zero()
        };
        
        for (is_right, sibling) in &self.siblings {
            let (left, right) = if *is_right {
                (sibling, &current_hash)
            } else {
                (&current_hash, sibling)
            };
            
            let internal = TrieNode::Internal {
                left: *left,
                right: *right,
            };
            current_hash = internal.hash();
        }
        
        current_hash == self.root
    }
}

/// Node storage trait
pub trait NodeStorage: Send + Sync {
    fn get(&self, hash: &HashValue) -> Option<TrieNode>;
    fn put(&self, hash: HashValue, node: TrieNode);
    fn delete(&self, hash: &HashValue);
}

/// In-memory node storage
#[derive(Default)]
pub struct InMemoryNodeStorage {
    nodes: RwLock<HashMap<HashValue, TrieNode>>,
}

impl InMemoryNodeStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl NodeStorage for InMemoryNodeStorage {
    fn get(&self, hash: &HashValue) -> Option<TrieNode> {
        self.nodes.read().get(hash).cloned()
    }
    
    fn put(&self, hash: HashValue, node: TrieNode) {
        self.nodes.write().insert(hash, node);
    }
    
    fn delete(&self, hash: &HashValue) {
        self.nodes.write().remove(hash);
    }
}

/// Sparse Merkle Trie
pub struct SparseMerkleTrie {
    /// Root hash
    root: RwLock<HashValue>,
    /// Node storage
    storage: Arc<dyn NodeStorage>,
    /// Cache of recent nodes
    cache: RwLock<HashMap<HashValue, TrieNode>>,
    /// Pending changes (not yet committed)
    pending: RwLock<HashMap<Vec<u8>, Option<Vec<u8>>>>,
    /// Tree height (256 for SHA256)
    height: usize,
}

impl SparseMerkleTrie {
    /// Create new empty trie
    pub fn new(storage: Arc<dyn NodeStorage>) -> Self {
        Self {
            root: RwLock::new(HashValue::zero()),
            storage,
            cache: RwLock::new(HashMap::new()),
            pending: RwLock::new(HashMap::new()),
            height: 256,
        }
    }
    
    /// Create trie from existing root
    pub fn from_root(root: HashValue, storage: Arc<dyn NodeStorage>) -> Self {
        Self {
            root: RwLock::new(root),
            storage,
            cache: RwLock::new(HashMap::new()),
            pending: RwLock::new(HashMap::new()),
            height: 256,
        }
    }
    
    /// Get current root hash
    pub fn root_hash(&self) -> HashValue {
        *self.root.read()
    }
    
    /// Get value by key
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        // Check pending changes first
        if let Some(value) = self.pending.read().get(key) {
            return value.clone();
        }
        
        let key_hash = HashValue::sha256(key);
        let mut current_hash = *self.root.read();
        
        if current_hash.is_zero() {
            return None;
        }
        
        // Traverse the trie
        for i in 0..self.height {
            let node = self.get_node(&current_hash)?;
            
            match node {
                TrieNode::Empty => return None,
                TrieNode::Leaf { key: leaf_key, value, .. } => {
                    if leaf_key == key {
                        return Some(value);
                    } else {
                        return None;
                    }
                }
                TrieNode::Internal { left, right } => {
                    let bit = self.get_bit(&key_hash, i);
                    current_hash = if bit { right } else { left };
                }
                TrieNode::Extension { prefix, child } => {
                    // Check if prefix matches - convert prefix bytes to bits for comparison
                    let key_bits = self.key_to_bits(&key_hash);
                    let prefix_bits: Vec<bool> = prefix.iter().flat_map(|&b| (0..8).map(move |i| (b >> (7 - i)) & 1 == 1)).take(prefix.len() * 8).collect();
                    if i + prefix_bits.len() <= key_bits.len() && key_bits[i..i+prefix_bits.len()] == prefix_bits[..] {
                        current_hash = child;
                    } else {
                        return None;
                    }
                }
            }
        }
        
        None
    }
    
    /// Set value for key
    pub fn set(&self, key: Vec<u8>, value: Vec<u8>) {
        self.pending.write().insert(key, Some(value));
    }
    
    /// Delete key
    pub fn delete(&self, key: &[u8]) {
        self.pending.write().insert(key.to_vec(), None);
    }
    
    /// Commit pending changes and return new root
    pub fn commit(&self) -> HashValue {
        let pending = std::mem::take(&mut *self.pending.write());
        
        if pending.is_empty() {
            return *self.root.read();
        }
        
        let mut current_root = *self.root.read();
        
        for (key, value) in pending {
            current_root = self.update_single(current_root, &key, value);
        }
        
        *self.root.write() = current_root;
        current_root
    }
    
    /// Update a single key-value pair
    fn update_single(&self, root: HashValue, key: &[u8], value: Option<Vec<u8>>) -> HashValue {
        let key_hash = HashValue::sha256(key);
        
        match value {
            Some(v) => {
                let leaf = TrieNode::Leaf {
                    key: key.to_vec(),
                    value: v.clone(),
                    value_hash: HashValue::sha256(&v),
                };
                let leaf_hash = leaf.hash();
                self.put_node(leaf_hash, leaf);
                
                if root.is_zero() {
                    leaf_hash
                } else {
                    self.insert_at(root, &key_hash, leaf_hash, 0)
                }
            }
            None => {
                if root.is_zero() {
                    HashValue::zero()
                } else {
                    self.delete_at(root, &key_hash, 0)
                }
            }
        }
    }
    
    /// Insert leaf at position in trie
    fn insert_at(&self, node_hash: HashValue, key_hash: &HashValue, leaf_hash: HashValue, depth: usize) -> HashValue {
        if depth >= self.height {
            return leaf_hash;
        }
        
        if node_hash.is_zero() {
            return leaf_hash;
        }
        
        let node = match self.get_node(&node_hash) {
            Some(n) => n,
            None => return leaf_hash,
        };
        
        match node {
            TrieNode::Empty => leaf_hash,
            TrieNode::Leaf { key: existing_key, value, value_hash } => {
                let existing_hash = HashValue::sha256(&existing_key);
                
                if existing_hash == *key_hash {
                    // Replace existing leaf
                    leaf_hash
                } else {
                    // Split into internal node
                    let existing_leaf = TrieNode::Leaf {
                        key: existing_key,
                        value,
                        value_hash,
                    };
                    let existing_leaf_hash = existing_leaf.hash();
                    self.put_node(existing_leaf_hash, existing_leaf);
                    
                    self.create_internal_path(&existing_hash, existing_leaf_hash, key_hash, leaf_hash, depth)
                }
            }
            TrieNode::Internal { left, right } => {
                let bit = self.get_bit(key_hash, depth);
                let (new_left, new_right) = if bit {
                    (left, self.insert_at(right, key_hash, leaf_hash, depth + 1))
                } else {
                    (self.insert_at(left, key_hash, leaf_hash, depth + 1), right)
                };
                
                let internal = TrieNode::Internal {
                    left: new_left,
                    right: new_right,
                };
                let internal_hash = internal.hash();
                self.put_node(internal_hash, internal);
                internal_hash
            }
            TrieNode::Extension { prefix, child } => {
                // Handle extension node
                let child_hash = self.insert_at(child, key_hash, leaf_hash, depth + prefix.len());
                let extension = TrieNode::Extension {
                    prefix,
                    child: child_hash,
                };
                let extension_hash = extension.hash();
                self.put_node(extension_hash, extension);
                extension_hash
            }
        }
    }
    
    /// Delete leaf from trie
    fn delete_at(&self, node_hash: HashValue, key_hash: &HashValue, depth: usize) -> HashValue {
        if depth >= self.height || node_hash.is_zero() {
            return HashValue::zero();
        }
        
        let node = match self.get_node(&node_hash) {
            Some(n) => n,
            None => return HashValue::zero(),
        };
        
        match node {
            TrieNode::Empty => HashValue::zero(),
            TrieNode::Leaf { key, .. } => {
                let existing_hash = HashValue::sha256(&key);
                if existing_hash == *key_hash {
                    HashValue::zero()
                } else {
                    node_hash
                }
            }
            TrieNode::Internal { left, right } => {
                let bit = self.get_bit(key_hash, depth);
                let (new_left, new_right) = if bit {
                    (left, self.delete_at(right, key_hash, depth + 1))
                } else {
                    (self.delete_at(left, key_hash, depth + 1), right)
                };
                
                // Collapse if possible
                if new_left.is_zero() && new_right.is_zero() {
                    HashValue::zero()
                } else if new_left.is_zero() {
                    new_right
                } else if new_right.is_zero() {
                    new_left
                } else {
                    let internal = TrieNode::Internal {
                        left: new_left,
                        right: new_right,
                    };
                    let internal_hash = internal.hash();
                    self.put_node(internal_hash, internal);
                    internal_hash
                }
            }
            TrieNode::Extension { prefix, child } => {
                let child_hash = self.delete_at(child, key_hash, depth + prefix.len());
                if child_hash.is_zero() {
                    HashValue::zero()
                } else {
                    let extension = TrieNode::Extension {
                        prefix,
                        child: child_hash,
                    };
                    let extension_hash = extension.hash();
                    self.put_node(extension_hash, extension);
                    extension_hash
                }
            }
        }
    }
    
    /// Create internal path between two leaves
    fn create_internal_path(
        &self,
        key1: &HashValue,
        leaf1: HashValue,
        key2: &HashValue,
        leaf2: HashValue,
        depth: usize,
    ) -> HashValue {
        if depth >= self.height {
            return leaf2; // Collision (shouldn't happen with proper hashing)
        }
        
        let bit1 = self.get_bit(key1, depth);
        let bit2 = self.get_bit(key2, depth);
        
        if bit1 == bit2 {
            // Same direction, recurse
            let child = self.create_internal_path(key1, leaf1, key2, leaf2, depth + 1);
            let internal = if bit1 {
                TrieNode::Internal {
                    left: HashValue::zero(),
                    right: child,
                }
            } else {
                TrieNode::Internal {
                    left: child,
                    right: HashValue::zero(),
                }
            };
            let internal_hash = internal.hash();
            self.put_node(internal_hash, internal);
            internal_hash
        } else {
            // Different directions
            let (left, right) = if bit1 {
                (leaf2, leaf1)
            } else {
                (leaf1, leaf2)
            };
            let internal = TrieNode::Internal { left, right };
            let internal_hash = internal.hash();
            self.put_node(internal_hash, internal);
            internal_hash
        }
    }
    
    /// Get bit at position in hash
    fn get_bit(&self, hash: &HashValue, pos: usize) -> bool {
        let byte_pos = pos / 8;
        let bit_pos = 7 - (pos % 8);
        let bytes = hash.as_bytes();
        if byte_pos >= bytes.len() {
            return false;
        }
        (bytes[byte_pos] >> bit_pos) & 1 == 1
    }
    
    /// Convert key to bits
    fn key_to_bits(&self, hash: &HashValue) -> Vec<bool> {
        let mut bits = Vec::with_capacity(self.height);
        for i in 0..self.height {
            bits.push(self.get_bit(hash, i));
        }
        bits
    }
    
    /// Get node from cache or storage
    fn get_node(&self, hash: &HashValue) -> Option<TrieNode> {
        // Check cache first
        if let Some(node) = self.cache.read().get(hash) {
            return Some(node.clone());
        }
        
        // Load from storage
        let node = self.storage.get(hash)?;
        self.cache.write().insert(*hash, node.clone());
        Some(node)
    }
    
    /// Put node to cache and storage
    fn put_node(&self, hash: HashValue, node: TrieNode) {
        self.cache.write().insert(hash, node.clone());
        self.storage.put(hash, node);
    }
    
    /// Generate merkle proof for key
    pub fn prove(&self, key: &[u8]) -> MerkleProof {
        let key_hash = HashValue::sha256(key);
        let mut siblings = Vec::new();
        let mut current_hash = *self.root.read();
        let mut value = None;
        
        for i in 0..self.height {
            if current_hash.is_zero() {
                break;
            }
            
            let node = match self.get_node(&current_hash) {
                Some(n) => n,
                None => break,
            };
            
            match node {
                TrieNode::Empty => break,
                TrieNode::Leaf { key: leaf_key, value: leaf_value, .. } => {
                    if leaf_key == key {
                        value = Some(leaf_value);
                    }
                    break;
                }
                TrieNode::Internal { left, right } => {
                    let bit = self.get_bit(&key_hash, i);
                    if bit {
                        siblings.push((false, left));
                        current_hash = right;
                    } else {
                        siblings.push((true, right));
                        current_hash = left;
                    }
                }
                TrieNode::Extension { child, .. } => {
                    current_hash = child;
                }
            }
        }
        
        MerkleProof {
            key: key.to_vec(),
            value,
            siblings,
            root: *self.root.read(),
        }
    }
    
    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
    
    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.read().len()
    }
}

/// Account state stored in trie
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountState {
    /// Account balance
    pub balance: Amount,
    /// Account nonce
    pub nonce: u64,
    /// Code hash (for contracts)
    pub code_hash: Option<HashValue>,
    /// Storage root hash
    pub storage_root: HashValue,
}

impl Default for AccountState {
    fn default() -> Self {
        Self {
            balance: 0,
            nonce: 0,
            code_hash: None,
            storage_root: HashValue::zero(),
        }
    }
}

impl AccountState {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }
    
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        bincode::deserialize(data).ok()
    }
}

/// World state manager
pub struct WorldState {
    /// State trie
    trie: SparseMerkleTrie,
    /// Code storage
    code: RwLock<HashMap<HashValue, Vec<u8>>>,
    /// Account storage tries
    storage_tries: RwLock<HashMap<Address, SparseMerkleTrie>>,
}

impl WorldState {
    /// Create new world state
    pub fn new(storage: Arc<dyn NodeStorage>) -> Self {
        Self {
            trie: SparseMerkleTrie::new(storage),
            code: RwLock::new(HashMap::new()),
            storage_tries: RwLock::new(HashMap::new()),
        }
    }
    
    /// Get account state
    pub fn get_account(&self, address: &Address) -> AccountState {
        let key = address.as_bytes();
        self.trie.get(key)
            .and_then(|data| AccountState::deserialize(&data))
            .unwrap_or_default()
    }
    
    /// Set account state
    pub fn set_account(&self, address: &Address, state: AccountState) {
        let key = address.as_bytes().to_vec();
        let value = state.serialize();
        self.trie.set(key, value);
    }
    
    /// Get balance
    pub fn get_balance(&self, address: &Address) -> Amount {
        self.get_account(address).balance
    }
    
    /// Set balance
    pub fn set_balance(&self, address: &Address, balance: Amount) {
        let mut state = self.get_account(address);
        state.balance = balance;
        self.set_account(address, state);
    }
    
    /// Get nonce
    pub fn get_nonce(&self, address: &Address) -> u64 {
        self.get_account(address).nonce
    }
    
    /// Increment nonce
    pub fn increment_nonce(&self, address: &Address) {
        let mut state = self.get_account(address);
        state.nonce += 1;
        self.set_account(address, state);
    }
    
    /// Transfer balance between accounts
    pub fn transfer(&self, from: &Address, to: &Address, amount: Amount) -> Result<(), TrieError> {
        let mut from_state = self.get_account(from);
        let mut to_state = self.get_account(to);
        
        if from_state.balance < amount {
            return Err(TrieError::StorageError("Insufficient balance".to_string()));
        }
        
        from_state.balance -= amount;
        to_state.balance += amount;
        
        self.set_account(from, from_state);
        self.set_account(to, to_state);
        
        Ok(())
    }
    
    /// Set code for address
    pub fn set_code(&self, address: &Address, code: Vec<u8>) {
        let code_hash = HashValue::sha256(&code);
        self.code.write().insert(code_hash, code);
        
        let mut state = self.get_account(address);
        state.code_hash = Some(code_hash);
        self.set_account(address, state);
    }
    
    /// Get code for address
    pub fn get_code(&self, address: &Address) -> Option<Vec<u8>> {
        let state = self.get_account(address);
        let code_hash = state.code_hash?;
        self.code.read().get(&code_hash).cloned()
    }
    
    /// Commit state changes
    pub fn commit(&self) -> HashValue {
        self.trie.commit()
    }
    
    /// Get state root hash
    pub fn root_hash(&self) -> HashValue {
        self.trie.root_hash()
    }
    
    /// Generate proof for account
    pub fn prove_account(&self, address: &Address) -> MerkleProof {
        self.trie.prove(address.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_trie_basic() {
        let storage = Arc::new(InMemoryNodeStorage::new());
        let trie = SparseMerkleTrie::new(storage);
        
        trie.set(b"key1".to_vec(), b"value1".to_vec());
        trie.set(b"key2".to_vec(), b"value2".to_vec());
        
        let root = trie.commit();
        assert!(!root.is_zero());
        
        assert_eq!(trie.get(b"key1"), Some(b"value1".to_vec()));
        assert_eq!(trie.get(b"key2"), Some(b"value2".to_vec()));
        assert_eq!(trie.get(b"key3"), None);
    }
    
    #[test]
    fn test_trie_delete() {
        let storage = Arc::new(InMemoryNodeStorage::new());
        let trie = SparseMerkleTrie::new(storage);
        
        trie.set(b"key1".to_vec(), b"value1".to_vec());
        trie.commit();
        
        trie.delete(b"key1");
        trie.commit();
        
        assert_eq!(trie.get(b"key1"), None);
    }
    
    #[test]
    fn test_merkle_proof() {
        let storage = Arc::new(InMemoryNodeStorage::new());
        let trie = SparseMerkleTrie::new(storage);
        
        trie.set(b"key1".to_vec(), b"value1".to_vec());
        trie.commit();
        
        let proof = trie.prove(b"key1");
        assert!(proof.verify());
        assert_eq!(proof.value, Some(b"value1".to_vec()));
    }
    
    #[test]
    fn test_world_state() {
        let storage = Arc::new(InMemoryNodeStorage::new());
        let state = WorldState::new(storage);
        
        let addr1 = Address::from_public_key(b"user1");
        let addr2 = Address::from_public_key(b"user2");
        
        state.set_balance(&addr1, 1000);
        state.set_balance(&addr2, 500);
        
        state.transfer(&addr1, &addr2, 300).unwrap();
        
        assert_eq!(state.get_balance(&addr1), 700);
        assert_eq!(state.get_balance(&addr2), 800);
    }
}
