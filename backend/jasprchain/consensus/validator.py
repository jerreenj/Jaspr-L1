"""Validator Management for JasprChain
Mirrors Rust validator module

Follows HyperLiquid model: Start with ~4 validators, scale up
"""
from dataclasses import dataclass, field
from typing import Dict, List, Optional
from datetime import datetime, timezone
import hashlib
import secrets

from ..crypto.bls import ValidatorBLSKeys, BLSPublicKey


@dataclass
class ValidatorStats:
    """Performance statistics for a validator"""
    blocks_proposed: int = 0
    blocks_attested: int = 0
    blocks_missed: int = 0
    uptime_percentage: float = 100.0
    last_active: int = 0  # Unix timestamp
    
    def to_dict(self) -> dict:
        return {
            'blocks_proposed': self.blocks_proposed,
            'blocks_attested': self.blocks_attested,
            'blocks_missed': self.blocks_missed,
            'uptime_percentage': round(self.uptime_percentage, 2),
            'last_active': self.last_active
        }


@dataclass
class Validator:
    """Represents a validator node in JasprChain
    
    Mirrors Rust Validator struct
    """
    address: str
    bls_public_key: str  # Hex encoded
    stake: int  # In base units (like wei)
    
    # Status
    active: bool = True
    jailed: bool = False
    
    # Performance
    stats: ValidatorStats = field(default_factory=ValidatorStats)
    
    # Commission (basis points, 100 = 1%)
    commission_rate: int = 500  # 5% default
    
    # Metadata
    name: str = ""
    website: str = ""
    
    def __post_init__(self):
        if not self.stats:
            self.stats = ValidatorStats()
    
    @property
    def voting_power(self) -> int:
        """Voting power based on stake"""
        if not self.active or self.jailed:
            return 0
        return self.stake
    
    def to_dict(self) -> dict:
        return {
            'address': self.address,
            'bls_public_key': self.bls_public_key,
            'stake': self.stake,
            'voting_power': self.voting_power,
            'active': self.active,
            'jailed': self.jailed,
            'stats': self.stats.to_dict(),
            'commission_rate': self.commission_rate,
            'name': self.name,
            'website': self.website
        }


class ValidatorSet:
    """Manages the set of active validators
    
    Implements HyperLiquid-style validator model:
    - Start with 4 validators
    - Weighted by stake for proposer selection
    - 2/3 threshold for finality
    """
    
    def __init__(self):
        self.validators: Dict[str, Validator] = {}
        self._bls_keys: Dict[str, ValidatorBLSKeys] = {}  # Internal key storage
    
    def add_validator(
        self, 
        address: str, 
        stake: int, 
        name: str = "",
        commission_rate: int = 500
    ) -> Validator:
        """Add a new validator to the set"""
        # Generate BLS keys for this validator
        bls_keys = ValidatorBLSKeys()
        
        validator = Validator(
            address=address,
            bls_public_key=bls_keys.public_key.to_hex(),
            stake=stake,
            name=name,
            commission_rate=commission_rate,
            active=True,
            stats=ValidatorStats(
                last_active=int(datetime.now(timezone.utc).timestamp() * 1000)
            )
        )
        
        self.validators[address] = validator
        self._bls_keys[address] = bls_keys
        
        return validator
    
    def get_validator(self, address: str) -> Optional[Validator]:
        return self.validators.get(address)
    
    def get_active_validators(self) -> List[Validator]:
        """Get all active, non-jailed validators"""
        return [
            v for v in self.validators.values() 
            if v.active and not v.jailed
        ]
    
    def total_stake(self) -> int:
        """Total stake of active validators"""
        return sum(v.stake for v in self.get_active_validators())
    
    def total_voting_power(self) -> int:
        """Total voting power for consensus"""
        return sum(v.voting_power for v in self.get_active_validators())
    
    def has_supermajority(self, voting_power: int) -> bool:
        """Check if voting power meets 2/3 threshold"""
        total = self.total_voting_power()
        if total == 0:
            return False
        return voting_power >= (total * 2) // 3
    
    def get_bls_keys(self, address: str) -> Optional[ValidatorBLSKeys]:
        """Get BLS keys for signing (internal use)"""
        return self._bls_keys.get(address)
    
    def slash_validator(self, address: str, amount: int, reason: str):
        """Slash a validator's stake"""
        validator = self.validators.get(address)
        if validator:
            validator.stake = max(0, validator.stake - amount)
            if validator.stake == 0:
                validator.jailed = True
    
    def jail_validator(self, address: str):
        """Jail a validator (remove from active set)"""
        validator = self.validators.get(address)
        if validator:
            validator.jailed = True
    
    def unjail_validator(self, address: str):
        """Unjail a validator"""
        validator = self.validators.get(address)
        if validator:
            validator.jailed = False
    
    def to_dict(self) -> dict:
        return {
            'validators': [v.to_dict() for v in self.validators.values()],
            'total_validators': len(self.validators),
            'active_validators': len(self.get_active_validators()),
            'total_stake': self.total_stake(),
            'total_voting_power': self.total_voting_power()
        }
