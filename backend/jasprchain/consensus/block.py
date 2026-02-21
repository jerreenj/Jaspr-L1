"""Block Structure for JasprChain
Mirrors Rust consensus block types
"""
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Any
from datetime import datetime, timezone
import json
import hashlib


@dataclass
class BlockHeader:
    """Block header containing consensus-critical data
    
    Mirrors Rust struct for easy porting
    """
    height: int
    previous_hash: str
    timestamp: int  # Unix timestamp in milliseconds
    proposer: str  # Validator address
    state_root: str  # Sparse Merkle Tree root
    transactions_root: str  # Merkle root of transactions
    receipts_root: str  # Merkle root of execution receipts
    
    # Consensus fields
    committee_signatures: str = ""  # Aggregated BLS signature
    attestation_count: int = 0  # Number of validators who attested
    
    def hash(self) -> str:
        """Calculate block hash"""
        data = json.dumps({
            'height': self.height,
            'previous_hash': self.previous_hash,
            'timestamp': self.timestamp,
            'proposer': self.proposer,
            'state_root': self.state_root,
            'transactions_root': self.transactions_root,
            'receipts_root': self.receipts_root
        }, sort_keys=True).encode()
        return hashlib.sha256(data).hexdigest()
    
    def to_dict(self) -> dict:
        return {
            'height': self.height,
            'previous_hash': self.previous_hash,
            'timestamp': self.timestamp,
            'proposer': self.proposer,
            'state_root': self.state_root,
            'transactions_root': self.transactions_root,
            'receipts_root': self.receipts_root,
            'committee_signatures': self.committee_signatures,
            'attestation_count': self.attestation_count
        }


@dataclass
class Block:
    """Complete block with header and transactions
    
    Mirrors Rust Block struct
    """
    header: BlockHeader
    transactions: List[Dict[str, Any]] = field(default_factory=list)
    
    # Execution results
    receipts: List[Dict[str, Any]] = field(default_factory=list)
    
    # Finality status
    finalized: bool = False
    finality_time_ms: Optional[int] = None
    
    @property
    def hash(self) -> str:
        return self.header.hash()
    
    @property
    def height(self) -> int:
        return self.header.height
    
    def to_dict(self) -> dict:
        return {
            'hash': self.hash,
            'header': self.header.to_dict(),
            'transactions': self.transactions,
            'transaction_count': len(self.transactions),
            'receipts': self.receipts,
            'finalized': self.finalized,
            'finality_time_ms': self.finality_time_ms
        }
    
    def to_summary(self) -> dict:
        """Compact summary for API responses"""
        return {
            'hash': self.hash,
            'height': self.height,
            'timestamp': self.header.timestamp,
            'proposer': self.header.proposer,
            'tx_count': len(self.transactions),
            'finalized': self.finalized,
            'state_root': self.header.state_root[:16] + '...'
        }


def create_genesis_block() -> Block:
    """Create the genesis block for JasprChain"""
    now = int(datetime.now(timezone.utc).timestamp() * 1000)
    
    header = BlockHeader(
        height=0,
        previous_hash="0" * 64,
        timestamp=now,
        proposer="jaspr1genesis",
        state_root=hashlib.sha256(b'genesis_state').hexdigest(),
        transactions_root=hashlib.sha256(b'').hexdigest(),
        receipts_root=hashlib.sha256(b'').hexdigest(),
        committee_signatures="",
        attestation_count=0
    )
    
    genesis = Block(
        header=header,
        transactions=[],
        receipts=[],
        finalized=True,
        finality_time_ms=0
    )
    
    return genesis
