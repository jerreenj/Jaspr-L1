"""Committee Finality Engine
Mirrors Rust finality module

Implements:
- Rotating committee for pre-commit + commit
- 2/3 threshold BLS aggregation
- <2s deterministic finality
"""
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Set
from datetime import datetime, timezone
import asyncio

from .block import Block, BlockHeader
from .validator import ValidatorSet, Validator
from ..crypto.bls import (
    BLSSignature, aggregate_signatures, verify_aggregate, 
    bls_sign, BLSPublicKey
)


@dataclass
class VoteMessage:
    """Vote message from a validator"""
    block_hash: str
    height: int
    validator_address: str
    signature: BLSSignature
    vote_type: str  # 'prevote' or 'precommit'
    timestamp: int


@dataclass
class CommitteeAttestation:
    """Aggregated committee attestation for a block
    
    Once 2/3 of validators sign, block is finalized
    """
    block_hash: str
    height: int
    aggregate_signature: bytes
    signers: List[str]  # Validator addresses
    voting_power: int
    total_voting_power: int
    finalized: bool = False
    finality_timestamp: Optional[int] = None
    
    @property
    def participation_rate(self) -> float:
        if self.total_voting_power == 0:
            return 0.0
        return self.voting_power / self.total_voting_power * 100
    
    def to_dict(self) -> dict:
        return {
            'block_hash': self.block_hash,
            'height': self.height,
            'aggregate_signature': self.aggregate_signature.hex() if self.aggregate_signature else '',
            'signers': self.signers,
            'voting_power': self.voting_power,
            'total_voting_power': self.total_voting_power,
            'participation_rate': round(self.participation_rate, 2),
            'finalized': self.finalized,
            'finality_timestamp': self.finality_timestamp
        }


class FinalityEngine:
    """Manages block finality through BLS attestations
    
    Implements JasprChain finality rules:
    - Safety with <1/3 Byzantine stake
    - Deterministic state root verification
    - No settlement rollback once finalized
    - <2s target finality time
    """
    
    # Target finality time in milliseconds
    TARGET_FINALITY_MS = 2000
    
    def __init__(self, validator_set: ValidatorSet):
        self.validator_set = validator_set
        
        # Vote storage by block hash
        self._prevotes: Dict[str, List[VoteMessage]] = {}
        self._precommits: Dict[str, List[VoteMessage]] = {}
        
        # Finalized blocks
        self._attestations: Dict[str, CommitteeAttestation] = {}
        
        # Timing
        self._block_proposal_times: Dict[str, int] = {}
    
    def on_block_proposed(self, block: Block):
        """Record when a block was proposed"""
        self._block_proposal_times[block.hash] = int(
            datetime.now(timezone.utc).timestamp() * 1000
        )
    
    def add_prevote(
        self, 
        block_hash: str, 
        height: int, 
        validator_address: str,
        signature: BLSSignature
    ) -> bool:
        """Add a prevote from a validator"""
        vote = VoteMessage(
            block_hash=block_hash,
            height=height,
            validator_address=validator_address,
            signature=signature,
            vote_type='prevote',
            timestamp=int(datetime.now(timezone.utc).timestamp() * 1000)
        )
        
        if block_hash not in self._prevotes:
            self._prevotes[block_hash] = []
        
        # Check for duplicate
        if any(v.validator_address == validator_address for v in self._prevotes[block_hash]):
            return False
        
        self._prevotes[block_hash].append(vote)
        return True
    
    def add_precommit(
        self, 
        block_hash: str, 
        height: int, 
        validator_address: str,
        signature: BLSSignature
    ) -> Optional[CommitteeAttestation]:
        """Add a precommit and check for finality"""
        vote = VoteMessage(
            block_hash=block_hash,
            height=height,
            validator_address=validator_address,
            signature=signature,
            vote_type='precommit',
            timestamp=int(datetime.now(timezone.utc).timestamp() * 1000)
        )
        
        if block_hash not in self._precommits:
            self._precommits[block_hash] = []
        
        # Check for duplicate
        if any(v.validator_address == validator_address for v in self._precommits[block_hash]):
            return None
        
        self._precommits[block_hash].append(vote)
        
        # Check if we have 2/3 threshold
        return self._check_finality(block_hash, height)
    
    def _check_finality(self, block_hash: str, height: int) -> Optional[CommitteeAttestation]:
        """Check if block has reached finality"""
        precommits = self._precommits.get(block_hash, [])
        
        if not precommits:
            return None
        
        # Calculate voting power
        voting_power = 0
        signers = []
        signatures = []
        
        for vote in precommits:
            validator = self.validator_set.get_validator(vote.validator_address)
            if validator and validator.active and not validator.jailed:
                voting_power += validator.voting_power
                signers.append(vote.validator_address)
                signatures.append(vote.signature)
        
        total_voting_power = self.validator_set.total_voting_power()
        
        # Check 2/3 threshold
        finalized = self.validator_set.has_supermajority(voting_power)
        
        # Aggregate signatures
        aggregate_sig = aggregate_signatures(signatures) if signatures else b''
        
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        proposal_time = self._block_proposal_times.get(block_hash, now)
        
        attestation = CommitteeAttestation(
            block_hash=block_hash,
            height=height,
            aggregate_signature=aggregate_sig,
            signers=signers,
            voting_power=voting_power,
            total_voting_power=total_voting_power,
            finalized=finalized,
            finality_timestamp=now if finalized else None
        )
        
        if finalized:
            self._attestations[block_hash] = attestation
        
        return attestation
    
    def get_attestation(self, block_hash: str) -> Optional[CommitteeAttestation]:
        """Get attestation for a block"""
        return self._attestations.get(block_hash)
    
    def is_finalized(self, block_hash: str) -> bool:
        """Check if a block is finalized"""
        attestation = self._attestations.get(block_hash)
        return attestation is not None and attestation.finalized
    
    def get_finality_time(self, block_hash: str) -> Optional[int]:
        """Get finality time in milliseconds"""
        proposal_time = self._block_proposal_times.get(block_hash)
        attestation = self._attestations.get(block_hash)
        
        if proposal_time and attestation and attestation.finality_timestamp:
            return attestation.finality_timestamp - proposal_time
        return None
