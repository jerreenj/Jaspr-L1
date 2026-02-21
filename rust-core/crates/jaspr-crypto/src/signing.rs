//! Digital signature operations

use ed25519_dalek::{Signer as DalekSigner, Verifier as DalekVerifier};
use serde::{Deserialize, Serialize, Deserializer, Serializer};
use crate::keys::{KeyPair, PrivateKey, PublicKey};
use jaspr_types::{Transaction, SignedTransaction, HashValue};

/// Ed25519 signature (64 bytes)
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Signature([u8; 64]);

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

impl Signature {
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    /// Create from slice
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != 64 {
            return None;
        }
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(slice);
        Some(Self(bytes))
    }

    /// Get raw bytes
    pub fn to_bytes(&self) -> [u8; 64] {
        self.0
    }

    /// Convert to vec
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
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
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sig({}...)", &self.to_hex()[..16])
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Trait for types that can sign messages
pub trait Signer {
    /// Sign a message
    fn sign(&self, message: &[u8]) -> Signature;
    
    /// Sign a hash
    fn sign_hash(&self, hash: &HashValue) -> Signature {
        self.sign(hash.as_bytes())
    }
}

/// Trait for types that can verify signatures
pub trait Verifier {
    /// Verify a signature
    fn verify(&self, message: &[u8], signature: &Signature) -> bool;
    
    /// Verify signature on hash
    fn verify_hash(&self, hash: &HashValue, signature: &Signature) -> bool {
        self.verify(hash.as_bytes(), signature)
    }
}

impl Signer for PrivateKey {
    fn sign(&self, message: &[u8]) -> Signature {
        let sig = self.signing_key().sign(message);
        Signature::from_bytes(sig.to_bytes())
    }
}

impl Signer for KeyPair {
    fn sign(&self, message: &[u8]) -> Signature {
        self.private_key().sign(message)
    }
}

impl Verifier for PublicKey {
    fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        let sig = ed25519_dalek::Signature::from_bytes(&signature.to_bytes());
        self.verifying_key().verify(message, &sig).is_ok()
    }
}

/// Sign a transaction
pub fn sign_transaction(keypair: &KeyPair, tx: Transaction) -> SignedTransaction {
    let hash = tx.signing_hash();
    let signature = keypair.sign(hash.as_bytes());
    
    SignedTransaction::new(
        tx,
        signature.to_vec(),
        keypair.public_key().to_bytes().to_vec(),
    )
}

/// Verify a signed transaction
pub fn verify_transaction(signed_tx: &SignedTransaction) -> bool {
    // Get public key
    if signed_tx.public_key.len() != 32 {
        return false;
    }
    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&signed_tx.public_key);
    
    let pubkey = match PublicKey::from_bytes(&pk_bytes) {
        Ok(pk) => pk,
        Err(_) => return false,
    };
    
    // Verify sender matches public key
    if pubkey.to_address() != signed_tx.transaction.sender {
        return false;
    }
    
    // Get signature
    let signature = match Signature::from_slice(&signed_tx.signature) {
        Some(s) => s,
        None => return false,
    };
    
    // Verify signature
    let hash = signed_tx.transaction.signing_hash();
    pubkey.verify_hash(&hash, &signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jaspr_types::{TransactionPayload, Address};

    #[test]
    fn test_sign_verify() {
        let keypair = KeyPair::generate();
        let message = b"test message";
        
        let signature = keypair.sign(message);
        assert!(keypair.public_key().verify(message, &signature));
        
        // Wrong message should fail
        assert!(!keypair.public_key().verify(b"wrong message", &signature));
    }

    #[test]
    fn test_sign_transaction() {
        let keypair = KeyPair::generate();
        
        let tx = Transaction::new(
            keypair.address(),
            TransactionPayload::Transfer {
                recipient: Address::zero(),
                amount: 1000,
            },
            0,
            100000,
            100,
            1,
        );
        
        let signed = sign_transaction(&keypair, tx);
        assert!(verify_transaction(&signed));
    }

    #[test]
    fn test_tampered_transaction_fails() {
        let keypair = KeyPair::generate();
        
        let tx = Transaction::new(
            keypair.address(),
            TransactionPayload::Transfer {
                recipient: Address::zero(),
                amount: 1000,
            },
            0,
            100000,
            100,
            1,
        );
        
        let mut signed = sign_transaction(&keypair, tx);
        
        // Tamper with the transaction
        signed.transaction.nonce = 999;
        
        // Should now fail verification
        assert!(!verify_transaction(&signed));
    }
}
