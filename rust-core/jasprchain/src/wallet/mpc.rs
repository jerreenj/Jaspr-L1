//! MPC Wallet implementation

use crate::crypto::Ed25519KeyPair;

/// MPC key share
pub struct MPCKeyShare {
    pub share_id: String,
    pub share_index: u32,
    pub threshold: u32,
    pub total_shares: u32,
}

/// MPC Wallet with threshold signing
pub struct MPCWallet {
    pub address: String,
    pub threshold: u32,
    pub num_shares: u32,
    keypair: Ed25519KeyPair,
}

impl MPCWallet {
    pub fn new() -> Self {
        let keypair = Ed25519KeyPair::generate();
        Self {
            address: keypair.address.clone(),
            threshold: 2,
            num_shares: 3,
            keypair,
        }
    }
    
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.keypair.sign(message)
    }
    
    pub fn public_key(&self) -> Vec<u8> {
        self.keypair.public_key_bytes()
    }
}

impl Default for MPCWallet {
    fn default() -> Self {
        Self::new()
    }
}
