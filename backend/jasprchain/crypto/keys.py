"""Ed25519 Key Management for JasprChain Wallets
Mirrors Rust: ed25519_dalek crate
"""
import nacl.signing
import nacl.encoding
import hashlib
import secrets
from dataclasses import dataclass
from typing import Optional
import base64


@dataclass
class Ed25519KeyPair:
    """Ed25519 keypair for wallet operations"""
    private_key: bytes
    public_key: bytes
    address: str
    
    def sign(self, message: bytes) -> bytes:
        """Sign a message with the private key"""
        signing_key = nacl.signing.SigningKey(self.private_key)
        signed = signing_key.sign(message)
        return signed.signature
    
    def verify(self, message: bytes, signature: bytes) -> bool:
        """Verify a signature"""
        try:
            verify_key = nacl.signing.VerifyKey(self.public_key)
            verify_key.verify(message, signature)
            return True
        except nacl.exceptions.BadSignature:
            return False
    
    def to_dict(self) -> dict:
        return {
            'public_key': base64.b64encode(self.public_key).decode(),
            'address': self.address
        }


def generate_wallet(seed: Optional[bytes] = None) -> Ed25519KeyPair:
    """Generate a new wallet keypair"""
    if seed:
        # Deterministic key generation from seed
        seed_hash = hashlib.sha256(seed).digest()
        signing_key = nacl.signing.SigningKey(seed_hash)
    else:
        # Random key generation
        signing_key = nacl.signing.SigningKey.generate()
    
    verify_key = signing_key.verify_key
    
    # Create address from public key (first 20 bytes of hash)
    address_hash = hashlib.sha256(bytes(verify_key)).hexdigest()[:40]
    address = f"jaspr1{address_hash}"
    
    return Ed25519KeyPair(
        private_key=bytes(signing_key),
        public_key=bytes(verify_key),
        address=address
    )


def address_from_public_key(public_key: bytes) -> str:
    """Derive address from public key"""
    address_hash = hashlib.sha256(public_key).hexdigest()[:40]
    return f"jaspr1{address_hash}"


def verify_signature(public_key: bytes, message: bytes, signature: bytes) -> bool:
    """Standalone signature verification"""
    try:
        verify_key = nacl.signing.VerifyKey(public_key)
        verify_key.verify(message, signature)
        return True
    except:
        return False
