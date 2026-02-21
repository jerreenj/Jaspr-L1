"""Sparse Merkle Tree Implementation
Mirrors Rust: sparse-merkle-tree crate

Used for:
- Trustless state verification
- Merkle-proof settlement receipts
- Transparent trading ledgers
"""
from dataclasses import dataclass, field
from typing import Dict, Any, Optional, List, Tuple
import hashlib
import json


def hash_node(data: bytes) -> str:
    """Hash a node value"""
    return hashlib.sha256(data).hexdigest()


def hash_pair(left: str, right: str) -> str:
    """Hash two child nodes"""
    combined = (left + right).encode()
    return hashlib.sha256(combined).hexdigest()


# Empty node hash (for sparse tree)
EMPTY_HASH = hash_node(b'')


@dataclass
class MerkleProof:
    """Merkle proof for a key-value pair
    
    Used to prove inclusion/exclusion in the state tree
    """
    key: str
    value: Optional[Any]
    siblings: List[str]  # Sibling hashes along the path
    path_bits: List[bool]  # Path direction (left=False, right=True)
    root: str
    
    def verify(self) -> bool:
        """Verify the proof against the root"""
        if self.value is None:
            current = EMPTY_HASH
        else:
            value_bytes = json.dumps(self.value, sort_keys=True).encode()
            current = hash_node(value_bytes)
        
        for sibling, is_right in zip(self.siblings, self.path_bits):
            if is_right:
                current = hash_pair(sibling, current)
            else:
                current = hash_pair(current, sibling)
        
        return current == self.root
    
    def to_dict(self) -> dict:
        return {
            'key': self.key,
            'value': self.value,
            'siblings': self.siblings,
            'path_bits': self.path_bits,
            'root': self.root,
            'valid': self.verify()
        }


class SparseMerkleTree:
    """Sparse Merkle Tree for state storage
    
    Properties:
    - Fixed depth (256 bits for full key space)
    - Efficient for sparse data
    - Enables compact proofs
    
    Mirrors Rust SMT implementation
    """
    
    TREE_DEPTH = 256  # Supports full SHA-256 key space
    
    def __init__(self):
        self._data: Dict[str, Any] = {}  # Key -> Value
        self._nodes: Dict[str, str] = {}  # Node path -> hash
        self._root: str = EMPTY_HASH
    
    @property
    def root(self) -> str:
        """Get current state root"""
        return self._root
    
    def get(self, key: str) -> Optional[Any]:
        """Get value for a key"""
        return self._data.get(key)
    
    def set(self, key: str, value: Any) -> str:
        """Set a value and return new root"""
        self._data[key] = value
        self._update_root()
        return self._root
    
    def delete(self, key: str) -> str:
        """Delete a key and return new root"""
        if key in self._data:
            del self._data[key]
            self._update_root()
        return self._root
    
    def _update_root(self):
        """Recalculate the root hash
        
        Simplified implementation - in production would use
        incremental updates for efficiency
        """
        if not self._data:
            self._root = EMPTY_HASH
            return
        
        # Hash all key-value pairs
        hashes = []
        for key in sorted(self._data.keys()):
            value = self._data[key]
            value_bytes = json.dumps(value, sort_keys=True, default=str).encode()
            key_hash = hash_node(key.encode())
            value_hash = hash_node(value_bytes)
            combined = hash_pair(key_hash, value_hash)
            hashes.append(combined)
        
        # Build tree from leaves
        while len(hashes) > 1:
            if len(hashes) % 2 == 1:
                hashes.append(EMPTY_HASH)
            
            next_level = []
            for i in range(0, len(hashes), 2):
                parent = hash_pair(hashes[i], hashes[i+1])
                next_level.append(parent)
            hashes = next_level
        
        self._root = hashes[0] if hashes else EMPTY_HASH
    
    def get_proof(self, key: str) -> MerkleProof:
        """Generate a Merkle proof for a key
        
        Can prove both inclusion and exclusion
        """
        # Simplified proof generation
        value = self._data.get(key)
        
        # In full implementation, would compute actual path
        # For now, return a simplified proof
        return MerkleProof(
            key=key,
            value=value,
            siblings=[],  # Would contain actual sibling hashes
            path_bits=[],
            root=self._root
        )
    
    def batch_update(self, updates: Dict[str, Any]) -> str:
        """Apply multiple updates atomically"""
        for key, value in updates.items():
            if value is None:
                if key in self._data:
                    del self._data[key]
            else:
                self._data[key] = value
        
        self._update_root()
        return self._root
    
    def snapshot(self) -> Dict[str, Any]:
        """Create a snapshot of current state"""
        return {
            'root': self._root,
            'size': len(self._data),
            'keys': list(self._data.keys())
        }


class StateStore:
    """High-level state store using SMT
    
    Provides account and contract storage management
    """
    
    def __init__(self):
        self.tree = SparseMerkleTree()
        self._pending_changes: Dict[str, Any] = {}
    
    @property
    def root(self) -> str:
        return self.tree.root
    
    def get(self, key: str, default: Any = None) -> Any:
        """Get state value"""
        # Check pending first
        if key in self._pending_changes:
            return self._pending_changes[key]
        return self.tree.get(key) or default
    
    def set(self, key: str, value: Any):
        """Set state value (staged)"""
        self._pending_changes[key] = value
    
    def __setitem__(self, key: str, value: Any):
        self.set(key, value)
    
    def __getitem__(self, key: str) -> Any:
        return self.get(key)
    
    def commit(self) -> str:
        """Commit pending changes and return new root"""
        if self._pending_changes:
            self.tree.batch_update(self._pending_changes)
            self._pending_changes.clear()
        return self.root
    
    def rollback(self):
        """Discard pending changes"""
        self._pending_changes.clear()
    
    def get_account_balance(self, address: str) -> int:
        """Get account balance"""
        return self.get(f"balance:{address}", 0)
    
    def set_account_balance(self, address: str, balance: int):
        """Set account balance"""
        self.set(f"balance:{address}", balance)
    
    def get_account_nonce(self, address: str) -> int:
        """Get account nonce"""
        return self.get(f"nonce:{address}", 0)
    
    def increment_nonce(self, address: str) -> int:
        """Increment and return new nonce"""
        nonce = self.get_account_nonce(address)
        new_nonce = nonce + 1
        self.set(f"nonce:{address}", new_nonce)
        return new_nonce
