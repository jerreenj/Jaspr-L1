"""Transaction Types for JasprChain
Mirrors Rust transaction module
"""
from dataclasses import dataclass, field
from typing import Optional, Dict, Any, List
from enum import Enum
from datetime import datetime, timezone
import hashlib
import json
import base64


class TransactionType(Enum):
    """Types of transactions in JasprChain"""
    TRANSFER = "transfer"  # Native token transfer
    STAKE = "stake"  # Stake tokens to validator
    UNSTAKE = "unstake"  # Unstake tokens
    DEX_ORDER = "dex_order"  # Place order on DEX
    DEX_CANCEL = "dex_cancel"  # Cancel DEX order
    DEX_SWAP = "dex_swap"  # Instant swap
    CONTRACT_CALL = "contract_call"  # Smart contract interaction
    CONTRACT_DEPLOY = "contract_deploy"  # Deploy new contract


@dataclass
class Transaction:
    """Base transaction structure
    
    Mirrors Rust Transaction struct for easy porting
    """
    tx_type: TransactionType
    sender: str  # Sender address
    
    # Transaction-specific data
    recipient: Optional[str] = None
    amount: int = 0  # In base units
    
    # For contract calls
    contract_address: Optional[str] = None
    function_name: Optional[str] = None
    args: Dict[str, Any] = field(default_factory=dict)
    
    # Gas
    gas_limit: int = 21000
    gas_price: int = 1000000000  # 1 Gwei equivalent
    
    # Nonce (prevents replay)
    nonce: int = 0
    
    # Timestamps
    timestamp: int = field(
        default_factory=lambda: int(datetime.now(timezone.utc).timestamp() * 1000)
    )
    
    # Chain ID (prevents cross-chain replay)
    chain_id: int = 1  # JasprChain mainnet
    
    def hash(self) -> str:
        """Calculate transaction hash"""
        data = json.dumps({
            'type': self.tx_type.value,
            'sender': self.sender,
            'recipient': self.recipient,
            'amount': self.amount,
            'contract_address': self.contract_address,
            'function_name': self.function_name,
            'args': self.args,
            'gas_limit': self.gas_limit,
            'gas_price': self.gas_price,
            'nonce': self.nonce,
            'timestamp': self.timestamp,
            'chain_id': self.chain_id
        }, sort_keys=True).encode()
        return hashlib.sha256(data).hexdigest()
    
    def to_bytes(self) -> bytes:
        """Serialize for signing"""
        return self.hash().encode()
    
    def to_dict(self) -> dict:
        return {
            'hash': self.hash(),
            'type': self.tx_type.value,
            'sender': self.sender,
            'recipient': self.recipient,
            'amount': self.amount,
            'contract_address': self.contract_address,
            'function_name': self.function_name,
            'args': self.args,
            'gas_limit': self.gas_limit,
            'gas_price': self.gas_price,
            'nonce': self.nonce,
            'timestamp': self.timestamp,
            'chain_id': self.chain_id
        }


@dataclass
class SignedTransaction:
    """Transaction with signature
    
    Mirrors Rust SignedTransaction struct
    """
    transaction: Transaction
    signature: bytes
    public_key: bytes
    
    @property
    def hash(self) -> str:
        return self.transaction.hash()
    
    def to_dict(self) -> dict:
        tx_dict = self.transaction.to_dict()
        tx_dict['signature'] = base64.b64encode(self.signature).decode()
        tx_dict['public_key'] = base64.b64encode(self.public_key).decode()
        return tx_dict
    
    @classmethod
    def from_dict(cls, data: dict) -> 'SignedTransaction':
        """Deserialize from dict"""
        tx = Transaction(
            tx_type=TransactionType(data['type']),
            sender=data['sender'],
            recipient=data.get('recipient'),
            amount=data.get('amount', 0),
            contract_address=data.get('contract_address'),
            function_name=data.get('function_name'),
            args=data.get('args', {}),
            gas_limit=data.get('gas_limit', 21000),
            gas_price=data.get('gas_price', 1000000000),
            nonce=data.get('nonce', 0),
            timestamp=data.get('timestamp', 0),
            chain_id=data.get('chain_id', 1)
        )
        return cls(
            transaction=tx,
            signature=base64.b64decode(data.get('signature', '')),
            public_key=base64.b64decode(data.get('public_key', ''))
        )


@dataclass
class TransactionReceipt:
    """Receipt of executed transaction"""
    tx_hash: str
    block_height: int
    block_hash: str
    status: str  # 'success', 'failed', 'reverted'
    gas_used: int
    
    # Execution results
    logs: List[Dict[str, Any]] = field(default_factory=list)
    return_value: Optional[Any] = None
    error_message: Optional[str] = None
    
    # State changes
    state_changes: List[Dict[str, Any]] = field(default_factory=list)
    
    def to_dict(self) -> dict:
        return {
            'tx_hash': self.tx_hash,
            'block_height': self.block_height,
            'block_hash': self.block_hash,
            'status': self.status,
            'gas_used': self.gas_used,
            'logs': self.logs,
            'return_value': self.return_value,
            'error_message': self.error_message,
            'state_changes': self.state_changes
        }
