//! Hashing utilities for JasprChain

use sha2::{Sha256, Digest as Sha2Digest};
use sha3::{Sha3_256, Keccak256};
use jaspr_types::HashValue;

/// Available hash functions
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashFunction {
    /// SHA-256 (default)
    Sha256,
    /// SHA3-256
    Sha3_256,
    /// Keccak-256 (Ethereum compatible)
    Keccak256,
    /// Blake3 (fastest)
    Blake3,
}

impl Default for HashFunction {
    fn default() -> Self {
        Self::Sha256
    }
}

/// Hasher trait for computing hashes
pub trait Hasher {
    /// Hash arbitrary data
    fn hash(&self, data: &[u8]) -> HashValue;
    
    /// Hash multiple items
    fn hash_all(&self, items: &[&[u8]]) -> HashValue {
        let mut combined = Vec::new();
        for item in items {
            combined.extend_from_slice(item);
        }
        self.hash(&combined)
    }
}

/// SHA-256 hasher
pub struct Sha256Hasher;

impl Hasher for Sha256Hasher {
    fn hash(&self, data: &[u8]) -> HashValue {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        HashValue::new(bytes)
    }
}

/// SHA3-256 hasher
pub struct Sha3Hasher;

impl Hasher for Sha3Hasher {
    fn hash(&self, data: &[u8]) -> HashValue {
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        HashValue::new(bytes)
    }
}

/// Keccak-256 hasher (Ethereum compatible)
pub struct KeccakHasher;

impl Hasher for KeccakHasher {
    fn hash(&self, data: &[u8]) -> HashValue {
        let mut hasher = Keccak256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        HashValue::new(bytes)
    }
}

/// Blake3 hasher (fastest)
pub struct Blake3Hasher;

impl Hasher for Blake3Hasher {
    fn hash(&self, data: &[u8]) -> HashValue {
        let hash = blake3::hash(data);
        HashValue::new(*hash.as_bytes())
    }
}

/// Get hasher for specified function
pub fn get_hasher(func: HashFunction) -> Box<dyn Hasher + Send + Sync> {
    match func {
        HashFunction::Sha256 => Box::new(Sha256Hasher),
        HashFunction::Sha3_256 => Box::new(Sha3Hasher),
        HashFunction::Keccak256 => Box::new(KeccakHasher),
        HashFunction::Blake3 => Box::new(Blake3Hasher),
    }
}

/// Quick hash functions
pub fn sha256(data: &[u8]) -> HashValue {
    Sha256Hasher.hash(data)
}

pub fn sha3(data: &[u8]) -> HashValue {
    Sha3Hasher.hash(data)
}

pub fn keccak256(data: &[u8]) -> HashValue {
    KeccakHasher.hash(data)
}

pub fn blake3(data: &[u8]) -> HashValue {
    Blake3Hasher.hash(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let hash = sha256(b"hello world");
        assert!(!hash.is_zero());
        
        // Deterministic
        assert_eq!(hash, sha256(b"hello world"));
        
        // Different input = different hash
        assert_ne!(hash, sha256(b"hello"));
    }

    #[test]
    fn test_different_hashers() {
        let data = b"test data";
        
        let h1 = sha256(data);
        let h2 = sha3(data);
        let h3 = keccak256(data);
        let h4 = blake3(data);
        
        // All should be different
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_ne!(h3, h4);
    }

    #[test]
    fn test_hash_all() {
        let hasher = Sha256Hasher;
        let hash1 = hasher.hash_all(&[b"hello", b"world"]);
        let hash2 = hasher.hash(b"helloworld");
        
        assert_eq!(hash1, hash2);
    }
}
