"""Account Abstraction Implementation
Mirrors Rust AA module

Features from spec:
- Smart account with programmed safety
- Session keys for dApps (revokable)
- Gas abstraction
- Spending limits and 2FA
"""
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any
from datetime import datetime, timezone
from enum import Enum
import hashlib
import secrets


class SessionKeyPermission(Enum):
    """Permissions for session keys"""
    TRANSFER = "transfer"
    DEX_TRADE = "dex_trade"
    CONTRACT_CALL = "contract_call"
    STAKE = "stake"
    FULL = "full"


@dataclass
class SessionKey:
    """Session key for dApp access
    
    Allows dApps limited access without full wallet control
    """
    key_id: str
    public_key: bytes
    permissions: List[SessionKeyPermission]
    spending_limit: int  # Max amount per session
    spent: int = 0
    expires_at: int = 0  # Unix timestamp
    created_at: int = field(
        default_factory=lambda: int(datetime.now(timezone.utc).timestamp() * 1000)
    )
    revoked: bool = False
    dapp_name: str = ""
    
    def is_valid(self) -> bool:
        """Check if session key is still valid"""
        if self.revoked:
            return False
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        if self.expires_at > 0 and now > self.expires_at:
            return False
        return True
    
    def can_spend(self, amount: int) -> bool:
        """Check if session key can spend amount"""
        return self.is_valid() and (self.spent + amount) <= self.spending_limit
    
    def to_dict(self) -> dict:
        return {
            'key_id': self.key_id,
            'permissions': [p.value for p in self.permissions],
            'spending_limit': self.spending_limit,
            'spent': self.spent,
            'remaining': self.spending_limit - self.spent,
            'expires_at': self.expires_at,
            'created_at': self.created_at,
            'revoked': self.revoked,
            'is_valid': self.is_valid(),
            'dapp_name': self.dapp_name
        }


@dataclass
class SpendingLimit:
    """Spending limit configuration"""
    daily_limit: int
    transaction_limit: int
    daily_spent: int = 0
    last_reset: int = 0
    
    def can_spend(self, amount: int) -> bool:
        self._maybe_reset()
        return (
            amount <= self.transaction_limit and
            (self.daily_spent + amount) <= self.daily_limit
        )
    
    def record_spend(self, amount: int):
        self._maybe_reset()
        self.daily_spent += amount
    
    def _maybe_reset(self):
        """Reset daily spent if new day"""
        now = int(datetime.now(timezone.utc).timestamp())
        day_start = (now // 86400) * 86400
        if self.last_reset < day_start:
            self.daily_spent = 0
            self.last_reset = day_start


@dataclass
class TwoFactorConfig:
    """2FA configuration for high-value transactions"""
    enabled: bool = False
    threshold: int = 0  # Amount above which 2FA is required
    method: str = "totp"  # totp, email, sms
    secret_hash: str = ""  # Hashed TOTP secret


class AAWallet:
    """Account Abstraction Wallet
    
    Implements programmable wallet logic:
    - Spending limits
    - Session keys for dApps
    - 2FA for large transactions
    - Guardian-based recovery
    - Gas abstraction
    """
    
    def __init__(self, address: str):
        self.address = address
        self.session_keys: Dict[str, SessionKey] = {}
        self.spending_limits: Dict[str, SpendingLimit] = {}
        self.two_factor = TwoFactorConfig()
        self.guardians: List[str] = []
        self.blocked_addresses: List[str] = []
        
        # Default spending limits
        self.spending_limits['default'] = SpendingLimit(
            daily_limit=10_000_000_000_000,  # 10k tokens
            transaction_limit=1_000_000_000_000  # 1k tokens
        )
    
    def create_session_key(
        self,
        public_key: bytes,
        permissions: List[SessionKeyPermission],
        spending_limit: int,
        duration_hours: int = 24,
        dapp_name: str = ""
    ) -> SessionKey:
        """Create a new session key for a dApp"""
        key_id = hashlib.sha256(
            f"{self.address}:{public_key.hex()}:{datetime.now(timezone.utc).timestamp()}".encode()
        ).hexdigest()[:16]
        
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        expires = now + (duration_hours * 3600 * 1000)
        
        session_key = SessionKey(
            key_id=key_id,
            public_key=public_key,
            permissions=permissions,
            spending_limit=spending_limit,
            expires_at=expires,
            dapp_name=dapp_name
        )
        
        self.session_keys[key_id] = session_key
        return session_key
    
    def revoke_session_key(self, key_id: str) -> bool:
        """Revoke a session key immediately"""
        if key_id in self.session_keys:
            self.session_keys[key_id].revoked = True
            return True
        return False
    
    def revoke_all_session_keys(self):
        """Emergency revoke all session keys"""
        for key in self.session_keys.values():
            key.revoked = True
    
    def validate_transaction(
        self,
        amount: int,
        recipient: str,
        session_key_id: Optional[str] = None
    ) -> Tuple[bool, str]:
        """Validate if transaction is allowed by AA rules
        
        Returns (allowed, reason)
        """
        # Check blocked addresses
        if recipient in self.blocked_addresses:
            return False, "Recipient is blocked"
        
        # Check spending limits
        limits = self.spending_limits.get('default')
        if limits and not limits.can_spend(amount):
            return False, "Spending limit exceeded"
        
        # Check session key if provided
        if session_key_id:
            session_key = self.session_keys.get(session_key_id)
            if not session_key:
                return False, "Invalid session key"
            if not session_key.is_valid():
                return False, "Session key expired or revoked"
            if not session_key.can_spend(amount):
                return False, "Session key spending limit exceeded"
        
        # Check 2FA requirement
        if self.two_factor.enabled and amount > self.two_factor.threshold:
            return False, "2FA_REQUIRED"
        
        return True, "OK"
    
    def record_transaction(self, amount: int, session_key_id: Optional[str] = None):
        """Record a completed transaction"""
        limits = self.spending_limits.get('default')
        if limits:
            limits.record_spend(amount)
        
        if session_key_id and session_key_id in self.session_keys:
            self.session_keys[session_key_id].spent += amount
    
    def add_guardian(self, guardian_address: str):
        """Add a guardian for recovery"""
        if guardian_address not in self.guardians:
            self.guardians.append(guardian_address)
    
    def remove_guardian(self, guardian_address: str):
        """Remove a guardian"""
        if guardian_address in self.guardians:
            self.guardians.remove(guardian_address)
    
    def block_address(self, address: str):
        """Block an address from receiving funds"""
        if address not in self.blocked_addresses:
            self.blocked_addresses.append(address)
    
    def unblock_address(self, address: str):
        """Unblock an address"""
        if address in self.blocked_addresses:
            self.blocked_addresses.remove(address)
    
    def to_dict(self) -> dict:
        return {
            'address': self.address,
            'session_keys': [sk.to_dict() for sk in self.session_keys.values()],
            'active_session_keys': len([sk for sk in self.session_keys.values() if sk.is_valid()]),
            'guardians': self.guardians,
            'blocked_addresses': self.blocked_addresses,
            'two_factor_enabled': self.two_factor.enabled,
            'spending_limits': {
                k: {
                    'daily_limit': v.daily_limit,
                    'transaction_limit': v.transaction_limit,
                    'daily_spent': v.daily_spent
                }
                for k, v in self.spending_limits.items()
            }
        }


class AccountAbstraction:
    """Account Abstraction manager
    
    Handles gas abstraction and transaction validation
    """
    
    def __init__(self):
        self.wallets: Dict[str, AAWallet] = {}
    
    def get_or_create_wallet(self, address: str) -> AAWallet:
        """Get or create AA wallet for address"""
        if address not in self.wallets:
            self.wallets[address] = AAWallet(address)
        return self.wallets[address]
    
    def validate_user_operation(
        self,
        sender: str,
        to: str,
        amount: int,
        session_key_id: Optional[str] = None
    ) -> Tuple[bool, str]:
        """Validate a user operation (AA-style transaction)"""
        wallet = self.wallets.get(sender)
        if not wallet:
            return True, "OK"  # No AA rules configured
        
        return wallet.validate_transaction(amount, to, session_key_id)
    
    def estimate_gas_in_tokens(self, gas_amount: int, token_price: float) -> int:
        """Estimate gas cost in $JJ tokens
        
        Enables gas abstraction - users pay gas in native token
        """
        # Gas price in base units
        gas_cost_wei = gas_amount * 1_000_000_000  # 1 Gwei
        # Convert to token amount based on current price
        token_amount = int(gas_cost_wei / token_price) if token_price > 0 else gas_cost_wei
        return token_amount


from typing import Tuple
