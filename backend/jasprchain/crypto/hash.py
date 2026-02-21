"""Cryptographic Hash Functions for JasprChain
Mirrors Rust: sha2, blake3 crates
"""
import hashlib
from typing import List, Any
import json


def sha256_hash(data: bytes) -> bytes:
    """SHA-256 hash - used for transactions and addresses"""
    return hashlib.sha256(data).digest()


def sha256_hex(data) -> str:
    """SHA-256 hash as hex string"""
    if isinstance(data, str):
        data = data.encode('utf-8')
    return hashlib.sha256(data).hexdigest()


def double_sha256(data: bytes) -> bytes:
    """Double SHA-256 - used for block hashes"""
    return sha256_hash(sha256_hash(data))


def blake3_hash(data: bytes) -> bytes:
    """BLAKE3 hash - faster alternative for state hashing
    
    In production Rust: use blake3 crate
    Here we simulate with SHA-256 for compatibility
    """
    # BLAKE3 would be: blake3.blake3(data).digest()
    return hashlib.sha256(data).digest()


def merkle_root(items: List[bytes]) -> bytes:
    """Calculate Merkle root from list of hashes
    
    Used for:
    - Transaction root in blocks
    - State root in Sparse Merkle Tree
    
    Mirrors Rust: merkle crate pattern
    """
    if not items:
        return sha256_hash(b'')
    
    if len(items) == 1:
        return items[0]
    
    # Pad to even number
    if len(items) % 2 == 1:
        items = items + [items[-1]]
    
    # Build tree bottom-up
    while len(items) > 1:
        next_level = []
        for i in range(0, len(items), 2):
            combined = items[i] + items[i + 1]
            next_level.append(sha256_hash(combined))
        items = next_level
    
    return items[0]


def hash_object(obj: Any) -> bytes:
    """Hash a Python object (serializes to JSON first)"""
    data = json.dumps(obj, sort_keys=True, default=str).encode()
    return sha256_hash(data)


def hash_hex(obj: Any) -> str:
    """Hash object and return hex string"""
    return hash_object(obj).hex()
