//! Key management for JasprChain

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use jaspr_types::Address;

/// Private key (32 bytes)
#[derive(Clone)]
pub struct PrivateKey(SigningKey);

impl PrivateKey {
    /// Generate new random private key
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        Self(SigningKey::generate(&mut csprng))
    }

    /// Create from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, ed25519_dalek::SignatureError> {
        Ok(Self(SigningKey::from_bytes(bytes)))
    }

    /// Get raw bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Get public key
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key())
    }

    /// Get signing key reference
    pub fn signing_key(&self) -> &SigningKey {
        &self.0
    }

    /// Create from hex string
    pub fn from_hex(hex_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        let bytes = hex::decode(hex_str)?;
        if bytes.len() != 32 {
            return Err("Invalid key length".into());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self::from_bytes(&arr)?)
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }
}

/// Public key (32 bytes)
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKey(#[serde(with = "verifying_key_serde")] VerifyingKey);

mod verifying_key_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(key: &VerifyingKey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(key.to_bytes()))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<VerifyingKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom("Invalid key length"));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        VerifyingKey::from_bytes(&arr).map_err(serde::de::Error::custom)
    }
}

impl PublicKey {
    /// Create from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, ed25519_dalek::SignatureError> {
        Ok(Self(VerifyingKey::from_bytes(bytes)?))
    }

    /// Get raw bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Get verifying key reference
    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.0
    }

    /// Derive address from public key
    pub fn to_address(&self) -> Address {
        Address::from_public_key(&self.to_bytes())
    }

    /// Create from hex string
    pub fn from_hex(hex_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        let bytes = hex::decode(hex_str)?;
        if bytes.len() != 32 {
            return Err("Invalid key length".into());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self::from_bytes(&arr)?)
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PublicKey({})", &self.to_hex()[..16])
    }
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Key pair for signing
#[derive(Clone)]
pub struct KeyPair {
    private: PrivateKey,
    public: PublicKey,
}

impl KeyPair {
    /// Generate new random key pair
    pub fn generate() -> Self {
        let private = PrivateKey::generate();
        let public = private.public_key();
        Self { private, public }
    }

    /// Create from private key
    pub fn from_private_key(private: PrivateKey) -> Self {
        let public = private.public_key();
        Self { private, public }
    }

    /// Get private key
    pub fn private_key(&self) -> &PrivateKey {
        &self.private
    }

    /// Get public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public
    }

    /// Get address derived from public key
    pub fn address(&self) -> Address {
        self.public.to_address()
    }

    /// Export private key hex
    pub fn export_private_hex(&self) -> String {
        self.private.to_hex()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        
        // Different keypairs should have different addresses
        assert_ne!(kp1.address(), kp2.address());
    }

    #[test]
    fn test_private_key_roundtrip() {
        let kp = KeyPair::generate();
        let hex = kp.export_private_hex();
        let recovered = PrivateKey::from_hex(&hex).unwrap();
        
        assert_eq!(kp.private_key().to_bytes(), recovered.to_bytes());
    }

    #[test]
    fn test_public_key_address() {
        let kp = KeyPair::generate();
        let addr = kp.address();
        
        // Address should be deterministic
        assert_eq!(addr, kp.public_key().to_address());
    }
}
