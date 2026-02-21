"""Slashing Module for JasprChain
Handles validator misbehavior penalties

Slashing Conditions:
1. Double Signing - Signing two different blocks at same height (SEVERE)
2. Downtime - Missing too many blocks (MODERATE)
3. Invalid Attestation - Attesting to invalid block (SEVERE)

Penalties:
- Double Sign: 5% of stake slashed, 7-day jail
- Downtime: 0.1% of stake slashed per missed window
- Invalid Attestation: 3% of stake slashed, 3-day jail
"""
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Set
from datetime import datetime, timezone, timedelta
from enum import Enum
import hashlib


class SlashingType(Enum):
    """Types of slashable offenses"""
    DOUBLE_SIGN = "double_sign"
    DOWNTIME = "downtime"
    INVALID_ATTESTATION = "invalid_attestation"


@dataclass
class SlashingEvidence:
    """Evidence of slashable offense"""
    evidence_type: SlashingType
    validator: str
    height: int
    timestamp: int
    evidence_hash: str
    details: Dict
    
    @classmethod
    def create_double_sign_evidence(cls, validator: str, height: int, 
                                     block_hash_1: str, block_hash_2: str,
                                     signature_1: str, signature_2: str) -> 'SlashingEvidence':
        """Create evidence for double signing"""
        evidence_data = f"{validator}:{height}:{block_hash_1}:{block_hash_2}"
        evidence_hash = hashlib.sha256(evidence_data.encode()).hexdigest()
        
        return cls(
            evidence_type=SlashingType.DOUBLE_SIGN,
            validator=validator,
            height=height,
            timestamp=int(datetime.now(timezone.utc).timestamp() * 1000),
            evidence_hash=evidence_hash,
            details={
                "block_hash_1": block_hash_1,
                "block_hash_2": block_hash_2,
                "signature_1": signature_1,
                "signature_2": signature_2
            }
        )
    
    @classmethod
    def create_downtime_evidence(cls, validator: str, missed_blocks: List[int],
                                  window_start: int, window_end: int) -> 'SlashingEvidence':
        """Create evidence for downtime"""
        evidence_data = f"{validator}:{window_start}:{window_end}:{len(missed_blocks)}"
        evidence_hash = hashlib.sha256(evidence_data.encode()).hexdigest()
        
        return cls(
            evidence_type=SlashingType.DOWNTIME,
            validator=validator,
            height=window_end,
            timestamp=int(datetime.now(timezone.utc).timestamp() * 1000),
            evidence_hash=evidence_hash,
            details={
                "missed_blocks": missed_blocks,
                "window_start": window_start,
                "window_end": window_end,
                "miss_rate": len(missed_blocks) / (window_end - window_start + 1)
            }
        )
    
    def to_dict(self) -> dict:
        return {
            "evidence_type": self.evidence_type.value,
            "validator": self.validator,
            "height": self.height,
            "timestamp": self.timestamp,
            "evidence_hash": self.evidence_hash,
            "details": self.details
        }


@dataclass
class SlashingRecord:
    """Record of a slashing event"""
    validator: str
    slash_type: SlashingType
    amount_slashed: int
    jail_until: Optional[int]
    height: int
    timestamp: int
    evidence_hash: str
    
    def to_dict(self) -> dict:
        return {
            "validator": self.validator,
            "slash_type": self.slash_type.value,
            "amount_slashed": self.amount_slashed,
            "jail_until": self.jail_until,
            "height": self.height,
            "timestamp": self.timestamp,
            "evidence_hash": self.evidence_hash
        }


@dataclass
class ValidatorSigningInfo:
    """Track validator signing behavior"""
    validator: str
    start_height: int
    index_offset: int = 0
    jailed_until: int = 0
    tombstoned: bool = False  # Permanent ban
    missed_blocks_counter: int = 0
    signed_blocks: Set[int] = field(default_factory=set)
    
    # Track block signatures for double-sign detection
    block_signatures: Dict[int, str] = field(default_factory=dict)  # height -> block_hash


class SlashingModule:
    """Slashing mechanism for validator misbehavior"""
    
    # Slashing parameters
    DOUBLE_SIGN_SLASH_FRACTION = 0.05  # 5% stake slashed
    DOWNTIME_SLASH_FRACTION = 0.001    # 0.1% per window
    INVALID_ATTESTATION_SLASH_FRACTION = 0.03  # 3% stake slashed
    
    # Jail durations (in milliseconds)
    DOUBLE_SIGN_JAIL_DURATION = 7 * 24 * 60 * 60 * 1000  # 7 days
    INVALID_ATTESTATION_JAIL_DURATION = 3 * 24 * 60 * 60 * 1000  # 3 days
    DOWNTIME_JAIL_DURATION = 1 * 24 * 60 * 60 * 1000  # 1 day
    
    # Downtime parameters
    SIGNED_BLOCKS_WINDOW = 100  # Check last 100 blocks
    MIN_SIGNED_PER_WINDOW = 50  # Must sign at least 50%
    
    def __init__(self, persistence=None):
        self.persistence = persistence
        self.signing_info: Dict[str, ValidatorSigningInfo] = {}
        self.slashing_records: List[SlashingRecord] = []
        self.pending_evidence: List[SlashingEvidence] = []
        self._processed_evidence: Set[str] = set()
        
        # Statistics
        self.stats = {
            "total_slashed_amount": 0,
            "double_sign_events": 0,
            "downtime_events": 0,
            "invalid_attestation_events": 0,
            "validators_jailed": 0,
            "validators_tombstoned": 0
        }
    
    def register_validator(self, validator: str, start_height: int):
        """Register a validator for signing tracking"""
        if validator not in self.signing_info:
            self.signing_info[validator] = ValidatorSigningInfo(
                validator=validator,
                start_height=start_height
            )
    
    def record_block_signature(self, validator: str, height: int, block_hash: str):
        """Record that a validator signed a block"""
        if validator not in self.signing_info:
            self.register_validator(validator, height)
        
        info = self.signing_info[validator]
        
        # Check for double signing
        if height in info.block_signatures:
            existing_hash = info.block_signatures[height]
            if existing_hash != block_hash:
                # DOUBLE SIGN DETECTED!
                evidence = SlashingEvidence.create_double_sign_evidence(
                    validator=validator,
                    height=height,
                    block_hash_1=existing_hash,
                    block_hash_2=block_hash,
                    signature_1="sig1",  # In real impl, actual signatures
                    signature_2="sig2"
                )
                self.submit_evidence(evidence)
                return False, "Double signing detected!"
        
        # Record signature
        info.block_signatures[height] = block_hash
        info.signed_blocks.add(height)
        info.missed_blocks_counter = max(0, info.missed_blocks_counter - 1)
        
        return True, "Signature recorded"
    
    def record_missed_block(self, validator: str, height: int):
        """Record that a validator missed signing a block"""
        if validator not in self.signing_info:
            self.register_validator(validator, height)
        
        info = self.signing_info[validator]
        info.missed_blocks_counter += 1
        
        # Check if exceeded downtime threshold
        if info.missed_blocks_counter >= (self.SIGNED_BLOCKS_WINDOW - self.MIN_SIGNED_PER_WINDOW):
            # Calculate missed blocks in window
            window_start = max(info.start_height, height - self.SIGNED_BLOCKS_WINDOW)
            missed_in_window = []
            for h in range(window_start, height + 1):
                if h not in info.signed_blocks:
                    missed_in_window.append(h)
            
            if len(missed_in_window) > (self.SIGNED_BLOCKS_WINDOW - self.MIN_SIGNED_PER_WINDOW):
                evidence = SlashingEvidence.create_downtime_evidence(
                    validator=validator,
                    missed_blocks=missed_in_window[-20:],  # Last 20 missed
                    window_start=window_start,
                    window_end=height
                )
                self.submit_evidence(evidence)
    
    def submit_evidence(self, evidence: SlashingEvidence) -> tuple:
        """Submit slashing evidence for processing"""
        # Check if already processed
        if evidence.evidence_hash in self._processed_evidence:
            return False, "Evidence already processed"
        
        # Validate evidence
        valid, msg = self._validate_evidence(evidence)
        if not valid:
            return False, msg
        
        self.pending_evidence.append(evidence)
        return True, "Evidence submitted for processing"
    
    def _validate_evidence(self, evidence: SlashingEvidence) -> tuple:
        """Validate slashing evidence"""
        # Check validator exists
        if evidence.validator not in self.signing_info:
            return False, "Unknown validator"
        
        info = self.signing_info[evidence.validator]
        
        # Check if validator is already tombstoned
        if info.tombstoned:
            return False, "Validator already tombstoned"
        
        # Evidence-specific validation
        if evidence.evidence_type == SlashingType.DOUBLE_SIGN:
            # Verify the two block hashes are different
            if evidence.details["block_hash_1"] == evidence.details["block_hash_2"]:
                return False, "Block hashes must be different"
        
        return True, "Evidence valid"
    
    def process_pending_evidence(self, validator_set) -> List[SlashingRecord]:
        """Process all pending evidence and execute slashing"""
        records = []
        
        for evidence in self.pending_evidence:
            if evidence.evidence_hash in self._processed_evidence:
                continue
            
            record = self._execute_slash(evidence, validator_set)
            if record:
                records.append(record)
                self._processed_evidence.add(evidence.evidence_hash)
        
        self.pending_evidence = []
        return records
    
    def _execute_slash(self, evidence: SlashingEvidence, validator_set) -> Optional[SlashingRecord]:
        """Execute slashing based on evidence"""
        validator = validator_set.get_validator(evidence.validator)
        if not validator:
            return None
        
        info = self.signing_info[evidence.validator]
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        
        # Calculate slash amount and jail duration based on type
        if evidence.evidence_type == SlashingType.DOUBLE_SIGN:
            slash_fraction = self.DOUBLE_SIGN_SLASH_FRACTION
            jail_duration = self.DOUBLE_SIGN_JAIL_DURATION
            self.stats["double_sign_events"] += 1
            # Double signing is severe - tombstone the validator
            info.tombstoned = True
            self.stats["validators_tombstoned"] += 1
            
        elif evidence.evidence_type == SlashingType.DOWNTIME:
            slash_fraction = self.DOWNTIME_SLASH_FRACTION
            jail_duration = self.DOWNTIME_JAIL_DURATION
            self.stats["downtime_events"] += 1
            
        elif evidence.evidence_type == SlashingType.INVALID_ATTESTATION:
            slash_fraction = self.INVALID_ATTESTATION_SLASH_FRACTION
            jail_duration = self.INVALID_ATTESTATION_JAIL_DURATION
            self.stats["invalid_attestation_events"] += 1
        else:
            return None
        
        # Calculate slash amount
        slash_amount = int(validator.stake * slash_fraction)
        
        # Execute slash
        validator.stake -= slash_amount
        
        # Jail validator
        jail_until = now + jail_duration
        info.jailed_until = jail_until
        validator.jailed = True
        validator.active = False
        self.stats["validators_jailed"] += 1
        
        # Update total slashed
        self.stats["total_slashed_amount"] += slash_amount
        
        # Create record
        record = SlashingRecord(
            validator=evidence.validator,
            slash_type=evidence.evidence_type,
            amount_slashed=slash_amount,
            jail_until=jail_until,
            height=evidence.height,
            timestamp=now,
            evidence_hash=evidence.evidence_hash
        )
        
        self.slashing_records.append(record)
        
        # Persist if available
        if self.persistence:
            self.persistence.save_state(
                f"slash_record:{evidence.evidence_hash}",
                record.to_dict()
            )
        
        return record
    
    def unjail_validator(self, validator_address: str, validator_set) -> tuple:
        """Attempt to unjail a validator"""
        if validator_address not in self.signing_info:
            return False, "Validator not found"
        
        info = self.signing_info[validator_address]
        
        if info.tombstoned:
            return False, "Validator is tombstoned (permanent ban)"
        
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        if info.jailed_until > now:
            remaining_ms = info.jailed_until - now
            remaining_hours = remaining_ms / (60 * 60 * 1000)
            return False, f"Jail period not complete. {remaining_hours:.1f} hours remaining"
        
        # Unjail
        info.jailed_until = 0
        info.missed_blocks_counter = 0
        
        validator = validator_set.get_validator(validator_address)
        if validator:
            validator.jailed = False
            validator.active = True
        
        return True, "Validator unjailed successfully"
    
    def get_validator_signing_info(self, validator: str) -> Optional[dict]:
        """Get signing info for a validator"""
        if validator not in self.signing_info:
            return None
        
        info = self.signing_info[validator]
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        
        return {
            "validator": info.validator,
            "start_height": info.start_height,
            "missed_blocks_counter": info.missed_blocks_counter,
            "jailed": info.jailed_until > now,
            "jailed_until": info.jailed_until,
            "tombstoned": info.tombstoned,
            "signed_blocks_count": len(info.signed_blocks)
        }
    
    def get_slashing_history(self, validator: Optional[str] = None) -> List[dict]:
        """Get slashing history, optionally filtered by validator"""
        records = self.slashing_records
        if validator:
            records = [r for r in records if r.validator == validator]
        return [r.to_dict() for r in records]
    
    def get_stats(self) -> dict:
        """Get slashing statistics"""
        return {
            **self.stats,
            "pending_evidence": len(self.pending_evidence),
            "total_records": len(self.slashing_records)
        }
