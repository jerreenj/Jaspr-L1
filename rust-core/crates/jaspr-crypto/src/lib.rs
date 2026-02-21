//! Cryptographic primitives for JasprChain
//!
//! Provides:
//! - Ed25519 key generation, signing, and verification
//! - Multiple hash functions (SHA256, SHA3, Blake3)
//! - BLS signatures for consensus (placeholder)

pub mod keys;
pub mod signing;
pub mod hashing;

pub use keys::{KeyPair, PrivateKey, PublicKey};
pub use signing::{Signature, Signer, Verifier};
pub use hashing::{HashFunction, Hasher};
