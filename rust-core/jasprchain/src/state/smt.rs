//! Sparse Merkle Tree implementation

use std::collections::HashMap;
use crate::crypto::{sha256, sha256_hex};

/// Sparse Merkle Tree for state storage
pub struct SparseMerkleTree {
    data: HashMap<String, u64>,
    root: [u8; 32],
}

impl SparseMerkleTree {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            root: sha256(b""),
        }
    }
    
    pub fn root(&self) -> String {
        hex::encode(self.root)
    }
    
    pub fn get(&self, key: &str) -> Option<u64> {
        self.data.get(key).copied()
    }
    
    pub fn set(&mut self, key: &str, value: u64) {
        self.data.insert(key.to_string(), value);
        self.update_root();
    }
    
    fn update_root(&mut self) {
        if self.data.is_empty() {
            self.root = sha256(b"");
            return;
        }
        
        let mut hashes: Vec<[u8; 32]> = self.data
            .iter()
            .map(|(k, v)| {
                let combined = format!("{}:{}", k, v);
                sha256(combined.as_bytes())
            })
            .collect();
        
        while hashes.len() > 1 {
            if hashes.len() % 2 == 1 {
                hashes.push(*hashes.last().unwrap());
            }
            
            let mut next = Vec::new();
            for chunk in hashes.chunks(2) {
                let mut combined = Vec::new();
                combined.extend_from_slice(&chunk[0]);
                combined.extend_from_slice(&chunk[1]);
                next.push(sha256(&combined));
            }
            hashes = next;
        }
        
        self.root = hashes[0];
    }
}

impl Default for SparseMerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level state store
pub struct StateStore {
    tree: SparseMerkleTree,
    pending: HashMap<String, u64>,
}

impl StateStore {
    pub fn new() -> Self {
        Self {
            tree: SparseMerkleTree::new(),
            pending: HashMap::new(),
        }
    }
    
    pub fn root(&self) -> String {
        self.tree.root()
    }
    
    pub fn get(&self, key: &str) -> Option<u64> {
        self.pending.get(key).copied().or_else(|| self.tree.get(key))
    }
    
    pub fn set(&mut self, key: &str, value: u64) {
        self.pending.insert(key.to_string(), value);
    }
    
    pub fn commit(&mut self) -> String {
        for (key, value) in self.pending.drain() {
            self.tree.set(&key, value);
        }
        self.tree.root()
    }
    
    pub fn rollback(&mut self) {
        self.pending.clear();
    }
    
    pub fn get_balance(&self, address: &str) -> u64 {
        self.get(&format!("balance:{}", address)).unwrap_or(0)
    }
    
    pub fn set_balance(&mut self, address: &str, balance: u64) {
        self.set(&format!("balance:{}", address), balance);
    }
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}
