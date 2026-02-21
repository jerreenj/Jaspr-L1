"""AI Sentinel - Real-time Risk Scoring System
Mirrors Rust AI Sentinel module

Features from spec:
- ML-based real-time risk scoring
- Blocks malicious drains, phishing, risky approvals
- Integrated into mempool + runtime pipes
- User-configurable guard modes
"""
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, Tuple
from enum import Enum
from datetime import datetime, timezone
import hashlib

from .model import RiskModel, TransactionFeatures
from ..execution.transaction import SignedTransaction, TransactionType


class ThreatType(Enum):
    """Types of threats detected by Sentinel"""
    NONE = "none"
    SCAM = "scam"
    PHISHING = "phishing"
    DRAIN = "drain"  # Wallet drainer
    RUG_PULL = "rug_pull"
    SANDWICH = "sandwich"  # MEV attack
    APPROVAL_RISK = "approval_risk"
    ANOMALY = "anomaly"
    BLOCKED_ADDRESS = "blocked_address"


class GuardMode(Enum):
    """User-configurable guard modes"""
    PASSIVE = "passive"  # Only monitor, no blocking
    WARNING = "warning"  # Warn user but allow
    ENFORCED = "enforced"  # Block high-risk transactions


@dataclass
class RiskScore:
    """Risk assessment result"""
    tx_hash: str
    score: float  # 0-100 (higher = more risky)
    threat_type: ThreatType
    guard_action: str  # 'allow', 'warn', 'block'
    
    # Detailed analysis
    analysis: Dict[str, Any] = field(default_factory=dict)
    risk_factors: List[str] = field(default_factory=list)
    
    # Timestamps
    scored_at: int = field(
        default_factory=lambda: int(datetime.now(timezone.utc).timestamp() * 1000)
    )
    
    def to_dict(self) -> dict:
        return {
            'tx_hash': self.tx_hash,
            'score': round(self.score, 2),
            'risk_level': self._get_risk_level(),
            'threat_type': self.threat_type.value,
            'guard_action': self.guard_action,
            'analysis': self.analysis,
            'risk_factors': self.risk_factors,
            'scored_at': self.scored_at
        }
    
    def _get_risk_level(self) -> str:
        if self.score < 20:
            return 'low'
        elif self.score < 50:
            return 'medium'
        elif self.score < 80:
            return 'high'
        else:
            return 'critical'


@dataclass
class SentinelStats:
    """Statistics for Sentinel performance"""
    total_scanned: int = 0
    threats_detected: int = 0
    transactions_blocked: int = 0
    false_positives_reported: int = 0
    avg_score: float = 0.0
    
    def to_dict(self) -> dict:
        return {
            'total_scanned': self.total_scanned,
            'threats_detected': self.threats_detected,
            'transactions_blocked': self.transactions_blocked,
            'false_positives_reported': self.false_positives_reported,
            'avg_score': round(self.avg_score, 2),
            'detection_rate': round(self.threats_detected / max(1, self.total_scanned) * 100, 2)
        }


class AISentinel:
    """AI-powered transaction security sentinel
    
    Integrates into:
    - Mempool (pre-execution scanning)
    - Runtime (execution-time checks)
    - Wallet (approval validation)
    """
    
    # Risk thresholds
    WARN_THRESHOLD = 40
    BLOCK_THRESHOLD = 75
    
    def __init__(self, guard_mode: GuardMode = GuardMode.ENFORCED):
        self.guard_mode = guard_mode
        self.risk_model = RiskModel()
        self.stats = SentinelStats()
        
        # Cache of recent scores
        self._score_cache: Dict[str, RiskScore] = {}
        
        # Flagged transactions (quarantine)
        self._quarantine: Dict[str, RiskScore] = {}
        
        # User-submitted feedback
        self._feedback: Dict[str, str] = {}  # tx_hash -> 'legitimate' or 'malicious'
        
        # Account history for feature extraction
        self._account_history: Dict[str, Dict[str, Any]] = {}
    
    def scan_transaction(
        self, 
        tx: SignedTransaction,
        sender_balance: int = 0,
        sender_history: Optional[Dict[str, Any]] = None
    ) -> RiskScore:
        """Scan a transaction for risks
        
        Called by:
        - Mempool on tx submission
        - Runtime before execution
        """
        tx_hash = tx.hash
        
        # Check cache
        if tx_hash in self._score_cache:
            return self._score_cache[tx_hash]
        
        # Extract features
        features = self._extract_features(tx, sender_balance, sender_history)
        
        # ML-based scoring
        score, analysis = self.risk_model.score_transaction(features)
        
        # Check for known threats
        threat_type = self._detect_threat_type(tx, score, analysis)
        
        # Determine action based on guard mode
        action = self._determine_action(score, threat_type)
        
        # Create result
        result = RiskScore(
            tx_hash=tx_hash,
            score=score,
            threat_type=threat_type,
            guard_action=action,
            analysis=analysis,
            risk_factors=analysis.get('factors', [])
        )
        
        # Update stats
        self._update_stats(result)
        
        # Cache result
        self._score_cache[tx_hash] = result
        
        # Quarantine if risky
        if action == 'block' or score > self.WARN_THRESHOLD:
            self._quarantine[tx_hash] = result
        
        return result
    
    def _extract_features(self, tx: SignedTransaction, balance: int, history: Optional[Dict] = None) -> TransactionFeatures:
        """Extract ML features from transaction"""
        inner = tx.transaction
        now = datetime.now(timezone.utc)
        
        history = history or self._account_history.get(inner.sender, {})
        
        return TransactionFeatures(
            amount=float(inner.amount),
            gas_price=float(inner.gas_price),
            sender_age_days=history.get('age_days', 0),
            sender_tx_count=history.get('tx_count', 0),
            recipient_age_days=0,  # Would need recipient history
            recipient_tx_count=0,
            time_since_last_tx=history.get('hours_since_last', 24),
            is_contract_interaction=1 if inner.contract_address else 0,
            is_new_recipient=1 if inner.recipient not in history.get('recipients', []) else 0,
            amount_vs_avg=inner.amount / max(1, history.get('avg_amount', inner.amount)),
            gas_vs_avg=inner.gas_price / 1_000_000_000,  # vs 1 Gwei baseline
            hour_of_day=now.hour
        )
    
    def _detect_threat_type(self, tx: SignedTransaction, score: float, analysis: Dict) -> ThreatType:
        """Classify the type of threat"""
        inner = tx.transaction
        
        # Check known malicious addresses
        if inner.recipient and self.risk_model.is_known_scam(inner.recipient):
            return ThreatType.BLOCKED_ADDRESS
        
        if inner.contract_address and self.risk_model.is_known_scam(inner.contract_address):
            return ThreatType.SCAM
        
        # Pattern-based detection
        if score > 80:
            factors = analysis.get('factors', [])
            if any('drain' in f.lower() for f in factors):
                return ThreatType.DRAIN
            if any('approval' in f.lower() for f in factors):
                return ThreatType.APPROVAL_RISK
            return ThreatType.ANOMALY
        
        if score > 50:
            return ThreatType.ANOMALY
        
        return ThreatType.NONE
    
    def _determine_action(self, score: float, threat_type: ThreatType) -> str:
        """Determine action based on guard mode and risk"""
        # Always block known malicious
        if threat_type in [ThreatType.BLOCKED_ADDRESS, ThreatType.SCAM, ThreatType.DRAIN]:
            return 'block'
        
        if self.guard_mode == GuardMode.PASSIVE:
            return 'allow'
        
        if self.guard_mode == GuardMode.WARNING:
            if score >= self.BLOCK_THRESHOLD:
                return 'warn'
            return 'allow'
        
        # Enforced mode
        if score >= self.BLOCK_THRESHOLD:
            return 'block'
        elif score >= self.WARN_THRESHOLD:
            return 'warn'
        return 'allow'
    
    def _update_stats(self, result: RiskScore):
        """Update sentinel statistics"""
        self.stats.total_scanned += 1
        
        if result.threat_type != ThreatType.NONE:
            self.stats.threats_detected += 1
        
        if result.guard_action == 'block':
            self.stats.transactions_blocked += 1
        
        # Running average
        n = self.stats.total_scanned
        self.stats.avg_score = (
            (self.stats.avg_score * (n - 1) + result.score) / n
        )
    
    def get_score(self, tx_hash: str) -> Optional[RiskScore]:
        """Get cached score for a transaction"""
        return self._score_cache.get(tx_hash)
    
    def get_quarantined(self) -> List[RiskScore]:
        """Get all quarantined transactions"""
        return list(self._quarantine.values())
    
    def submit_feedback(self, tx_hash: str, is_legitimate: bool):
        """Submit user feedback on a flagged transaction"""
        self._feedback[tx_hash] = 'legitimate' if is_legitimate else 'malicious'
        if is_legitimate:
            self.stats.false_positives_reported += 1
    
    def update_account_history(self, address: str, history: Dict[str, Any]):
        """Update account history for better feature extraction"""
        self._account_history[address] = history
    
    def add_malicious_address(self, address: str):
        """Add address to blocklist"""
        self.risk_model.add_known_scam_address(address)
    
    def set_guard_mode(self, mode: GuardMode):
        """Update guard mode"""
        self.guard_mode = mode
    
    def get_stats(self) -> Dict[str, Any]:
        """Get sentinel statistics"""
        return self.stats.to_dict()
    
    def to_dict(self) -> dict:
        return {
            'guard_mode': self.guard_mode.value,
            'stats': self.stats.to_dict(),
            'quarantine_count': len(self._quarantine),
            'cache_size': len(self._score_cache)
        }
