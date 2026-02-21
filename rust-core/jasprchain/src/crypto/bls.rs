//! BLS12-381 signatures for consensus
//!
//! Uses blst crate for production-grade BLS operations

use blst::min_pk::{SecretKey, PublicKey, Signature, AggregateSignature};
use blst::BLST_ERROR;
use rand::RngCore;

const DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";

/// BLS12-381 keypair
pub struct BLSKeyPair {
    secret_key: SecretKey,
    public_key: PublicKey,
}

impl BLSKeyPair {
    /// Generate new BLS keypair
    pub fn generate() -> Self {
        let mut ikm = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut ikm);
        
        let secret_key = SecretKey::key_gen(&ikm, &[]).expect("Key generation failed");
        let public_key = secret_key.sk_to_pk();
        
        Self {
            secret_key,
            public_key,
        }
    }
    
    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> BLSSignature {
        let sig = self.secret_key.sign(message, DST, &[]);
        BLSSignature {
            signature: sig,
            public_key: self.public_key.clone(),
        }
    }
    
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public_key.to_bytes().to_vec()
    }
}

/// BLS signature with associated public key
pub struct BLSSignature {
    pub signature: Signature,
    pub public_key: PublicKey,
}

impl BLSSignature {
    /// Verify the signature
    pub fn verify(&self, message: &[u8]) -> bool {
        let result = self.signature.verify(true, message, DST, &[], &self.public_key, true);
        result == BLST_ERROR::BLST_SUCCESS
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        self.signature.to_bytes().to_vec()
    }
}

/// Aggregate multiple signatures
pub fn aggregate_signatures(signatures: &[BLSSignature]) -> Vec<u8> {
    if signatures.is_empty() {
        return Vec::new();
    }
    
    let mut agg = AggregateSignature::from_signature(&signatures[0].signature);
    for sig in signatures.iter().skip(1) {
        agg.add_signature(&sig.signature, true).ok();
    }
    
    agg.to_signature().to_bytes().to_vec()
}

/// Verify aggregated signature
pub fn verify_aggregate(
    aggregate_sig: &[u8],
    public_keys: &[PublicKey],
    message: &[u8],
) -> bool {
    if aggregate_sig.len() != 96 || public_keys.is_empty() {
        return false;
    }
    
    let sig_bytes: [u8; 96] = aggregate_sig.try_into().unwrap();
    let sig = match Signature::from_bytes(&sig_bytes) {
        Ok(s) => s,
        Err(_) => return false,
    };
    
    // Aggregate public keys
    let pks: Vec<&PublicKey> = public_keys.iter().collect();
    let result = sig.aggregate_verify(true, &[message], DST, &pks, true);
    result == BLST_ERROR::BLST_SUCCESS
}
