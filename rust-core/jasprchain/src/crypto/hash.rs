//! Hash functions

use sha2::{Sha256, Digest};

/// SHA-256 hash
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// SHA-256 hash as hex string
pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(sha256(data))
}

/// Calculate Merkle root from list of hashes
pub fn merkle_root(items: &[[u8; 32]]) -> [u8; 32] {
    if items.is_empty() {
        return sha256(b"");
    }
    
    if items.len() == 1 {
        return items[0];
    }
    
    let mut current: Vec<[u8; 32]> = items.to_vec();
    
    // Pad to even number
    if current.len() % 2 == 1 {
        current.push(*current.last().unwrap());
    }
    
    while current.len() > 1 {
        let mut next = Vec::new();
        for chunk in current.chunks(2) {
            let mut combined = Vec::new();
            combined.extend_from_slice(&chunk[0]);
            combined.extend_from_slice(&chunk[1]);
            next.push(sha256(&combined));
        }
        current = next;
        
        if current.len() > 1 && current.len() % 2 == 1 {
            current.push(*current.last().unwrap());
        }
    }
    
    current[0]
}
