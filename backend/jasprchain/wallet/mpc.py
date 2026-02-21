"""MPC (Multi-Party Computation) Wallet Implementation
Mirrors Rust MPC module

Key features from spec:
- Split key: device share + enclave share
- No single compromise drains funds
- Recovery via guardian policies
"""
from dataclasses import dataclass, field
from typing import List, Optional, Tuple, Dict, Any
import hashlib
import secrets
import base64
from datetime import datetime, timezone

from ..crypto.keys import Ed25519KeyPair, generate_wallet


@dataclass
class MPCKeyShare:
    """A share of an MPC key
    
    In production: use actual secret sharing (Shamir's)
    """
    share_id: str
    share_data: bytes
    share_index: int
    threshold: int  # Minimum shares needed
    total_shares: int
    
    def to_dict(self) -> dict:
        return {
            'share_id': self.share_id,
            'share_index': self.share_index,
            'threshold': self.threshold,
            'total_shares': self.total_shares
        }


@dataclass
class MPCSignatureRequest:
    """Request for MPC signature"""
    request_id: str
    message_hash: str
    requester: str
    timestamp: int
    status: str = 'pending'  # pending, approved, rejected, completed
    approvals: List[str] = field(default_factory=list)
    partial_signatures: List[bytes] = field(default_factory=list)


class MPCWallet:
    """MPC Wallet with threshold signing
    
    Implements:
    - 2-of-3 threshold by default
    - Device share + cloud enclave share + recovery share
    - No seed phrase required
    - Sub-100ms signing latency target
    """
    
    DEFAULT_THRESHOLD = 2
    DEFAULT_SHARES = 3
    
    def __init__(
        self, 
        address: Optional[str] = None,
        threshold: int = DEFAULT_THRESHOLD,
        num_shares: int = DEFAULT_SHARES
    ):
        self.threshold = threshold
        self.num_shares = num_shares
        self._shares: List[MPCKeyShare] = []
        self._underlying_keypair: Optional[Ed25519KeyPair] = None
        self._pending_requests: Dict[str, MPCSignatureRequest] = {}
        
        if address is None:
            self._initialize_new_wallet()
        else:
            self.address = address
    
    def _initialize_new_wallet(self):
        """Create new MPC wallet with key shares"""
        # Generate underlying keypair
        self._underlying_keypair = generate_wallet()
        self.address = self._underlying_keypair.address
        
        # Split private key into shares
        # In production: use Shamir's Secret Sharing
        private_key = self._underlying_keypair.private_key
        
        for i in range(self.num_shares):
            # Simplified: XOR with random data
            # Production: proper Shamir polynomial evaluation
            share_data = bytes([
                private_key[j] ^ secrets.token_bytes(1)[0]
                for j in range(len(private_key))
            ])
            
            share = MPCKeyShare(
                share_id=hashlib.sha256(f"{self.address}:{i}".encode()).hexdigest()[:16],
                share_data=share_data,
                share_index=i,
                threshold=self.threshold,
                total_shares=self.num_shares
            )
            self._shares.append(share)
    
    def get_device_share(self) -> MPCKeyShare:
        """Get the device share (stored on user's device)"""
        return self._shares[0] if self._shares else None
    
    def get_enclave_share(self) -> MPCKeyShare:
        """Get the cloud enclave share (stored securely in cloud)"""
        return self._shares[1] if len(self._shares) > 1 else None
    
    def get_recovery_share(self) -> MPCKeyShare:
        """Get recovery share (stored with guardians)"""
        return self._shares[2] if len(self._shares) > 2 else None
    
    def initiate_signing(self, message: bytes) -> MPCSignatureRequest:
        """Start MPC signing process"""
        request_id = hashlib.sha256(
            f"{self.address}:{message.hex()}:{datetime.now(timezone.utc).timestamp()}".encode()
        ).hexdigest()[:16]
        
        request = MPCSignatureRequest(
            request_id=request_id,
            message_hash=hashlib.sha256(message).hexdigest(),
            requester=self.address,
            timestamp=int(datetime.now(timezone.utc).timestamp() * 1000)
        )
        
        self._pending_requests[request_id] = request
        return request
    
    def approve_signing(
        self, 
        request_id: str, 
        share: MPCKeyShare,
        partial_sig: bytes
    ) -> Tuple[bool, Optional[bytes]]:
        """Approve and contribute partial signature
        
        Returns (threshold_met, final_signature)
        """
        request = self._pending_requests.get(request_id)
        if not request:
            return False, None
        
        request.approvals.append(share.share_id)
        request.partial_signatures.append(partial_sig)
        
        # Check if threshold met
        if len(request.approvals) >= self.threshold:
            request.status = 'completed'
            # Combine partial signatures
            # In production: actual MPC signature combination
            final_sig = self._combine_signatures(request.partial_signatures)
            return True, final_sig
        
        return False, None
    
    def _combine_signatures(self, partials: List[bytes]) -> bytes:
        """Combine partial signatures into final signature
        
        In production: Lagrange interpolation for threshold signatures
        """
        if not partials:
            return b''
        
        # Simplified: use underlying keypair to sign
        # In real MPC, this would combine partials mathematically
        if self._underlying_keypair:
            # Return a valid signature using the full key
            return partials[0]  # Placeholder
        
        return partials[0]
    
    def sign_direct(self, message: bytes) -> bytes:
        """Direct signing (for development/testing)
        
        In production, always use MPC flow
        """
        if self._underlying_keypair:
            return self._underlying_keypair.sign(message)
        raise ValueError("No signing capability")
    
    def to_dict(self) -> dict:
        return {
            'address': self.address,
            'threshold': self.threshold,
            'total_shares': self.num_shares,
            'shares': [s.to_dict() for s in self._shares],
            'pending_requests': len(self._pending_requests)
        }
    
    def export_public_info(self) -> dict:
        """Export public wallet info (safe to share)"""
        return {
            'address': self.address,
            'public_key': base64.b64encode(
                self._underlying_keypair.public_key
            ).decode() if self._underlying_keypair else None,
            'type': 'mpc_wallet',
            'threshold': f"{self.threshold}-of-{self.num_shares}"
        }
