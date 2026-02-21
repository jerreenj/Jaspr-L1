"""Account State Management
Mirrors Rust account types
"""
from dataclasses import dataclass, field
from typing import Dict, Any, Optional
from datetime import datetime, timezone


@dataclass
class Account:
    """Account state in JasprChain
    
    Mirrors Rust Account struct
    """
    address: str
    balance: int = 0  # Native token balance
    nonce: int = 0  # Transaction nonce
    
    # For smart contract accounts
    code_hash: Optional[str] = None
    storage_root: Optional[str] = None
    
    # Account type
    is_contract: bool = False
    
    # MPC/AA wallet specific
    is_aa_wallet: bool = False
    guardian_addresses: list = field(default_factory=list)
    session_keys: Dict[str, Any] = field(default_factory=dict)
    spending_limits: Dict[str, int] = field(default_factory=dict)
    
    # Metadata
    created_at: int = field(
        default_factory=lambda: int(datetime.now(timezone.utc).timestamp() * 1000)
    )
    
    def to_dict(self) -> dict:
        return {
            'address': self.address,
            'balance': self.balance,
            'nonce': self.nonce,
            'code_hash': self.code_hash,
            'storage_root': self.storage_root,
            'is_contract': self.is_contract,
            'is_aa_wallet': self.is_aa_wallet,
            'guardian_addresses': self.guardian_addresses,
            'session_keys': list(self.session_keys.keys()),
            'spending_limits': self.spending_limits,
            'created_at': self.created_at
        }


@dataclass
class AccountState:
    """Complete account state including storage"""
    account: Account
    storage: Dict[str, Any] = field(default_factory=dict)
    
    def get_storage(self, key: str, default: Any = None) -> Any:
        return self.storage.get(key, default)
    
    def set_storage(self, key: str, value: Any):
        self.storage[key] = value
