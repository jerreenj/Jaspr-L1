//! Hash types and utilities for JasprChain

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::fmt;

/// 32-byte hash value
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, BorshSerialize, BorshDeserialize, Serialize, Deserialize, Default)]
pub struct HashValue([u8; 32]);

impl HashValue {
    /// Create from bytes
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create from slice (must be 32 bytes)
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Some(Self(bytes))
    }

    /// Hash arbitrary data with SHA256
    pub fn sha256(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        Self(bytes)
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Create from hex string
    pub fn from_hex(hex_str: &str) -> Result<Self, hex::FromHexError> {
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        let bytes = hex::decode(hex_str)?;
        Self::from_slice(&bytes).ok_or(hex::FromHexError::InvalidStringLength)
    }

    /// Zero hash
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    /// Check if zero
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 32]
    }
}

impl fmt::Debug for HashValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", &self.to_hex()[..16])
    }
}

impl fmt::Display for HashValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl From<[u8; 32]> for HashValue {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for HashValue {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Type alias for cleaner code
pub type Hash = HashValue;

/// Hasher trait for hashable types
pub trait Hasher {
    /// Compute hash of this object
    fn hash(&self) -> HashValue;
}

/// Merkle tree utilities
pub struct MerkleTree;

impl MerkleTree {
    /// Compute merkle root from list of hashes
    pub fn compute_root(hashes: &[HashValue]) -> HashValue {
        if hashes.is_empty() {
            return HashValue::zero();
        }
        if hashes.len() == 1 {
            return hashes[0];
        }

        let mut current_level: Vec<HashValue> = hashes.to_vec();
        
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in current_level.chunks(2) {
                let combined = if chunk.len() == 2 {
                    let mut data = Vec::with_capacity(64);
                    data.extend_from_slice(chunk[0].as_bytes());
                    data.extend_from_slice(chunk[1].as_bytes());
                    HashValue::sha256(&data)
                } else {
                    // Odd number - duplicate last element
                    let mut data = Vec::with_capacity(64);
                    data.extend_from_slice(chunk[0].as_bytes());
                    data.extend_from_slice(chunk[0].as_bytes());
                    HashValue::sha256(&data)
                };
                next_level.push(combined);
            }
            
            current_level = next_level;
        }
        
        current_level[0]
    }

    /// Verify merkle proof
    pub fn verify_proof(
        leaf: &HashValue,
        proof: &[HashValue],
        root: &HashValue,
        index: usize,
    ) -> bool {
        let mut current = *leaf;
        let mut idx = index;

        for sibling in proof {
            let mut data = Vec::with_capacity(64);
            if idx % 2 == 0 {
                data.extend_from_slice(current.as_bytes());
                data.extend_from_slice(sibling.as_bytes());
            } else {
                data.extend_from_slice(sibling.as_bytes());
                data.extend_from_slice(current.as_bytes());
            }
            current = HashValue::sha256(&data);
            idx /= 2;
        }

        current == *root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_sha256() {
        let hash = HashValue::sha256(b"hello world");
        assert!(!hash.is_zero());
        
        // Same input should produce same hash
        let hash2 = HashValue::sha256(b"hello world");
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_merkle_root() {
        let hashes = vec![
            HashValue::sha256(b"leaf1"),
            HashValue::sha256(b"leaf2"),
            HashValue::sha256(b"leaf3"),
            HashValue::sha256(b"leaf4"),
        ];
        
        let root = MerkleTree::compute_root(&hashes);
        assert!(!root.is_zero());
        
        // Root should be deterministic
        let root2 = MerkleTree::compute_root(&hashes);
        assert_eq!(root, root2);
    }

    #[test]
    fn test_empty_merkle() {
        let root = MerkleTree::compute_root(&[]);
        assert!(root.is_zero());
    }
}
