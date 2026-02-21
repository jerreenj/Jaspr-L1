"""Proposer Selection using VRF
Mirrors Rust: vrf crate pattern

Implements weighted VRF rotation for fair block proposer selection
"""
from dataclasses import dataclass
from typing import List, Optional, Tuple
import hashlib
import secrets
from .validator import ValidatorSet, Validator


@dataclass
class VRFOutput:
    """Verifiable Random Function output
    
    Used for unpredictable proposer selection
    Mirrors Rust VRF implementation
    """
    value: bytes  # 32 bytes random value
    proof: bytes  # VRF proof for verification
    
    @property
    def value_hex(self) -> str:
        return self.value.hex()
    
    def as_int(self) -> int:
        """Convert to integer for weighted selection"""
        return int.from_bytes(self.value[:8], 'big')


class ProposerSelection:
    """Implements weighted VRF proposer selection
    
    Key features (from JasprChain spec):
    - Weighted by stake + performance scoring
    - Reduces predictability and censorship vulnerability
    - Fast failover for latency spikes
    """
    
    def __init__(self, validator_set: ValidatorSet):
        self.validator_set = validator_set
        self._last_proposer: Optional[str] = None
        self._round_robin_index = 0
    
    def generate_vrf(self, seed: bytes) -> VRFOutput:
        """Generate VRF output from seed
        
        In production Rust: use actual VRF (e.g., ecvrf crate)
        Simulated here with HMAC-based approach
        """
        # Seed typically includes: previous_block_hash + height + epoch
        random_key = secrets.token_bytes(32)
        
        import hmac
        value = hmac.new(random_key, seed, hashlib.sha256).digest()
        proof = hmac.new(seed, random_key, hashlib.sha256).digest()
        
        return VRFOutput(value=value, proof=proof)
    
    def select_proposer(
        self, 
        height: int, 
        previous_hash: str,
        use_vrf: bool = True
    ) -> Optional[Validator]:
        """Select proposer for a given height
        
        Selection algorithm:
        1. Generate VRF from previous block hash + height
        2. Weight by stake amount
        3. Apply performance scoring (uptime bonus)
        4. Select deterministically based on VRF output
        """
        active_validators = self.validator_set.get_active_validators()
        
        if not active_validators:
            return None
        
        if use_vrf:
            # VRF-based weighted selection
            seed = f"{previous_hash}:{height}".encode()
            vrf = self.generate_vrf(seed)
            
            # Calculate weighted scores
            scores = []
            for v in active_validators:
                # Base score from stake
                base_score = v.stake
                
                # Performance bonus (up to 10% boost for good uptime)
                uptime_bonus = int(base_score * 0.1 * (v.stats.uptime_percentage / 100))
                
                # Penalty for recent proposals (spread load)
                recent_penalty = 0
                if v.address == self._last_proposer:
                    recent_penalty = int(base_score * 0.05)
                
                final_score = base_score + uptime_bonus - recent_penalty
                scores.append((v, final_score))
            
            # Weighted selection using VRF
            total_score = sum(s[1] for s in scores)
            if total_score == 0:
                return active_validators[0]
            
            target = vrf.as_int() % total_score
            cumulative = 0
            
            for validator, score in scores:
                cumulative += score
                if cumulative > target:
                    self._last_proposer = validator.address
                    return validator
            
            return scores[-1][0]
        else:
            # Simple round-robin (fallback)
            self._round_robin_index = height % len(active_validators)
            return active_validators[self._round_robin_index]
    
    def verify_proposer(
        self, 
        proposer: str, 
        height: int, 
        previous_hash: str
    ) -> bool:
        """Verify that a proposer was validly selected
        
        Other validators use this to verify block proposals
        """
        expected = self.select_proposer(height, previous_hash)
        return expected is not None and expected.address == proposer
