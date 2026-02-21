//! Ed25519 key management for wallets

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use sha2::{Sha256, Digest};
use rand::rngs::OsRng;

/// Ed25519 keypair for wallet operations
pub struct Ed25519KeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    pub address: String,
}

impl Ed25519KeyPair {
    /// Generate new keypair
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        
        let address = Self::derive_address(&verifying_key);
        
        Self {
            signing_key,
            verifying_key,
            address,
        }
    }
    
    /// Generate from seed
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();
        
        let address = Self::derive_address(&verifying_key);
        
        Self {
            signing_key,
            verifying_key,
            address,
        }
    }
    
    fn derive_address(verifying_key: &VerifyingKey) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifying_key.as_bytes());
        let hash = hex::encode(hasher.finalize());
        format!("jaspr1{}", &hash[..40])
    }
    
    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signature: Signature = self.signing_key.sign(message);
        signature.to_bytes().to_vec()
    }
    
    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        if signature.len() != 64 {
            return false;
        }
        let sig_bytes: [u8; 64] = signature.try_into().unwrap();
        let signature = Signature::from_bytes(&sig_bytes);
        self.verifying_key.verify(message, &signature).is_ok()
    }
    
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.verifying_key.as_bytes().to_vec()
    }
}

/// Generate a new wallet
pub fn generate_wallet() -> Ed25519KeyPair {
    Ed25519KeyPair::generate()
}
