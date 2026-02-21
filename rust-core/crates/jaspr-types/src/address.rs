//! Address types for JasprChain
//! 
//! Addresses are 32-byte identifiers derived from public keys

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::fmt;

/// 32-byte address
#[derive(Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Address([u8; 32]);

impl Address {
    /// Create address from bytes
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create address from public key (SHA256 hash)
    pub fn from_public_key(public_key: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(public_key);
        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        Self(bytes)
    }

    /// Create address from hex string
    pub fn from_hex(hex_str: &str) -> Result<Self, hex::FromHexError> {
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        let bytes = hex::decode(hex_str)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string with 0x prefix
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }

    /// Zero address (used for contract creation)
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    /// Check if zero address
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 32]
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address({})", self.to_hex())
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display as jaspr1... format
        let hex_part = hex::encode(&self.0[..16]);
        write!(f, "jaspr1{}", hex_part)
    }
}

impl Default for Address {
    fn default() -> Self {
        Self::zero()
    }
}

impl From<[u8; 32]> for Address {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Account address (alias for Move compatibility)
pub type AccountAddress = Address;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_from_public_key() {
        let pubkey = b"test_public_key_bytes_here_32b!";
        let addr = Address::from_public_key(pubkey);
        assert!(!addr.is_zero());
    }

    #[test]
    fn test_address_hex_roundtrip() {
        let original = Address::from_public_key(b"test");
        let hex_str = original.to_hex();
        let recovered = Address::from_hex(&hex_str).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_display_format() {
        let addr = Address::from_public_key(b"test");
        let display = format!("{}", addr);
        assert!(display.starts_with("jaspr1"));
    }
}
