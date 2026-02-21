"""Mempool Implementation for JasprChain
Mirrors Rust mempool module

Features from spec:
- Partitioned lanes: fee, risk, institutional, sponsored
- Parallel-execution aware ordering
- AI Sentinel flags suspicious transactions into quarantine
"""
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, Set
from enum import Enum
from datetime import datetime, timezone
import heapq

from ..execution.transaction import SignedTransaction, TransactionType
from ..sentinel import AISentinel, RiskScore


class MempoolLane(Enum):
    """Transaction lanes in the mempool"""
    PRIORITY = "priority"  # High fee transactions
    NORMAL = "normal"  # Standard transactions
    LIQUIDATION = "liquidation"  # DEX liquidations (highest priority)
    INSTITUTIONAL = "institutional"  # Verified institutional traders
    SPONSORED = "sponsored"  # Gas-sponsored transactions
    QUARANTINE = "quarantine"  # Flagged by AI Sentinel


@dataclass
class MempoolEntry:
    """Entry in the mempool"""
    transaction: SignedTransaction
    lane: MempoolLane
    priority_score: float  # Higher = processed first
    
    # Risk assessment
    risk_score: Optional[RiskScore] = None
    
    # Timing
    added_at: int = field(
        default_factory=lambda: int(datetime.now(timezone.utc).timestamp() * 1000)
    )
    expires_at: int = 0  # 0 = no expiry
    
    # State
    attempts: int = 0
    last_error: Optional[str] = None
    
    def __lt__(self, other):
        # For heapq (higher priority first)
        return self.priority_score > other.priority_score


@dataclass
class MempoolStats:
    """Mempool statistics"""
    total_pending: int = 0
    by_lane: Dict[str, int] = field(default_factory=dict)
    total_gas: int = 0
    avg_gas_price: float = 0.0
    oldest_tx_age_ms: int = 0
    
    def to_dict(self) -> dict:
        return {
            'total_pending': self.total_pending,
            'by_lane': self.by_lane,
            'total_gas': self.total_gas,
            'avg_gas_price': round(self.avg_gas_price, 2),
            'oldest_tx_age_ms': self.oldest_tx_age_ms
        }


class Mempool:
    """JasprChain mempool with partitioned lanes
    
    Implements:
    - Lane-based prioritization
    - AI Sentinel integration
    - Parallel-execution aware ordering
    - Spam resistance
    """
    
    MAX_SIZE = 10000
    MAX_PER_SENDER = 100
    DEFAULT_EXPIRY_MS = 300000  # 5 minutes
    
    def __init__(self, sentinel: Optional[AISentinel] = None):
        self.sentinel = sentinel
        
        # Lanes implemented as priority queues
        self._lanes: Dict[MempoolLane, List[MempoolEntry]] = {
            lane: [] for lane in MempoolLane
        }
        
        # Index by tx hash for fast lookup
        self._by_hash: Dict[str, MempoolEntry] = {}
        
        # Index by sender for rate limiting
        self._by_sender: Dict[str, Set[str]] = {}  # sender -> set of tx hashes
        
        # Processing order (combined from all lanes)
        self._processing_queue: List[MempoolEntry] = []
    
    def add_transaction(
        self, 
        tx: SignedTransaction,
        sender_balance: int = 0,
        sender_history: Optional[Dict] = None
    ) -> tuple:
        """Add transaction to mempool
        
        Returns (success, message, entry)
        """
        tx_hash = tx.hash
        sender = tx.transaction.sender
        
        # Check if already exists
        if tx_hash in self._by_hash:
            return False, "Transaction already in mempool", None
        
        # Check mempool size
        if len(self._by_hash) >= self.MAX_SIZE:
            return False, "Mempool full", None
        
        # Check sender rate limit
        sender_txs = self._by_sender.get(sender, set())
        if len(sender_txs) >= self.MAX_PER_SENDER:
            return False, f"Too many pending transactions from sender", None
        
        # AI Sentinel scan
        risk_score = None
        if self.sentinel:
            risk_score = self.sentinel.scan_transaction(tx, sender_balance, sender_history)
            
            if risk_score.guard_action == 'block':
                return False, f"Transaction blocked by AI Sentinel: {risk_score.risk_factors}", None
        
        # Determine lane
        lane = self._determine_lane(tx, risk_score)
        
        # Calculate priority score
        priority = self._calculate_priority(tx, lane, risk_score)
        
        # Create entry
        entry = MempoolEntry(
            transaction=tx,
            lane=lane,
            priority_score=priority,
            risk_score=risk_score,
            expires_at=int(datetime.now(timezone.utc).timestamp() * 1000) + self.DEFAULT_EXPIRY_MS
        )
        
        # Add to lane
        heapq.heappush(self._lanes[lane], entry)
        self._by_hash[tx_hash] = entry
        
        # Update sender index
        if sender not in self._by_sender:
            self._by_sender[sender] = set()
        self._by_sender[sender].add(tx_hash)
        
        # Update processing queue
        self._rebuild_processing_queue()
        
        return True, "Transaction added to mempool", entry
    
    def _determine_lane(self, tx: SignedTransaction, risk_score: Optional[RiskScore]) -> MempoolLane:
        """Determine which lane a transaction belongs to"""
        inner = tx.transaction
        
        # Quarantine if flagged
        if risk_score and risk_score.guard_action == 'warn':
            return MempoolLane.QUARANTINE
        
        # Liquidation transactions get highest priority
        if inner.tx_type == TransactionType.DEX_ORDER:
            if inner.args.get('is_liquidation'):
                return MempoolLane.LIQUIDATION
        
        # Check for institutional flag
        if inner.args.get('institutional'):
            return MempoolLane.INSTITUTIONAL
        
        # Check for sponsored (gas paid by another)
        if inner.args.get('sponsored_by'):
            return MempoolLane.SPONSORED
        
        # High fee = priority lane
        if inner.gas_price > 2_000_000_000:  # > 2 Gwei
            return MempoolLane.PRIORITY
        
        return MempoolLane.NORMAL
    
    def _calculate_priority(self, tx: SignedTransaction, lane: MempoolLane, risk_score: Optional[RiskScore]) -> float:
        """Calculate priority score for ordering"""
        inner = tx.transaction
        
        # Base priority from lane
        lane_priorities = {
            MempoolLane.LIQUIDATION: 10000,
            MempoolLane.PRIORITY: 5000,
            MempoolLane.INSTITUTIONAL: 4000,
            MempoolLane.NORMAL: 1000,
            MempoolLane.SPONSORED: 500,
            MempoolLane.QUARANTINE: 100
        }
        base = lane_priorities.get(lane, 1000)
        
        # Add gas price component (normalized to ~1000 range)
        gas_component = inner.gas_price / 1_000_000  # Gwei
        
        # Subtract risk score (lower risk = higher priority)
        risk_penalty = 0
        if risk_score:
            risk_penalty = risk_score.score * 10
        
        return base + gas_component - risk_penalty
    
    def _rebuild_processing_queue(self):
        """Rebuild the combined processing queue"""
        self._processing_queue.clear()
        
        # Priority order of lanes
        lane_order = [
            MempoolLane.LIQUIDATION,
            MempoolLane.PRIORITY,
            MempoolLane.INSTITUTIONAL,
            MempoolLane.NORMAL,
            MempoolLane.SPONSORED,
            MempoolLane.QUARANTINE
        ]
        
        for lane in lane_order:
            self._processing_queue.extend(self._lanes[lane])
        
        # Sort by priority score
        self._processing_queue.sort(key=lambda e: e.priority_score, reverse=True)
    
    def get_transactions_for_block(self, max_count: int = 100, max_gas: int = 10_000_000) -> List[SignedTransaction]:
        """Get transactions to include in next block"""
        self._expire_old_transactions()
        
        selected = []
        total_gas = 0
        used_hashes = set()
        
        for entry in self._processing_queue:
            if len(selected) >= max_count:
                break
            
            tx_gas = entry.transaction.transaction.gas_limit
            if total_gas + tx_gas > max_gas:
                continue
            
            if entry.transaction.hash in used_hashes:
                continue
            
            # Skip quarantined unless we're desperate
            if entry.lane == MempoolLane.QUARANTINE and len(selected) < max_count // 2:
                continue
            
            selected.append(entry.transaction)
            used_hashes.add(entry.transaction.hash)
            total_gas += tx_gas
        
        return selected
    
    def remove_transaction(self, tx_hash: str) -> bool:
        """Remove a transaction from mempool"""
        entry = self._by_hash.get(tx_hash)
        if not entry:
            return False
        
        # Remove from lane
        lane = entry.lane
        self._lanes[lane] = [e for e in self._lanes[lane] if e.transaction.hash != tx_hash]
        heapq.heapify(self._lanes[lane])
        
        # Remove from indexes
        del self._by_hash[tx_hash]
        
        sender = entry.transaction.transaction.sender
        if sender in self._by_sender:
            self._by_sender[sender].discard(tx_hash)
        
        self._rebuild_processing_queue()
        return True
    
    def remove_transactions(self, tx_hashes: List[str]):
        """Remove multiple transactions (after block execution)"""
        for tx_hash in tx_hashes:
            self.remove_transaction(tx_hash)
    
    def _expire_old_transactions(self):
        """Remove expired transactions"""
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        expired = [
            tx_hash for tx_hash, entry in self._by_hash.items()
            if entry.expires_at > 0 and entry.expires_at < now
        ]
        for tx_hash in expired:
            self.remove_transaction(tx_hash)
    
    def get_transaction(self, tx_hash: str) -> Optional[SignedTransaction]:
        entry = self._by_hash.get(tx_hash)
        return entry.transaction if entry else None
    
    def get_entry(self, tx_hash: str) -> Optional[MempoolEntry]:
        return self._by_hash.get(tx_hash)
    
    def get_pending_by_sender(self, sender: str) -> List[SignedTransaction]:
        tx_hashes = self._by_sender.get(sender, set())
        return [
            self._by_hash[h].transaction 
            for h in tx_hashes 
            if h in self._by_hash
        ]
    
    def get_stats(self) -> MempoolStats:
        """Get mempool statistics"""
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        
        by_lane = {lane.value: len(entries) for lane, entries in self._lanes.items()}
        
        total_gas = sum(
            e.transaction.transaction.gas_limit 
            for e in self._by_hash.values()
        )
        
        gas_prices = [
            e.transaction.transaction.gas_price 
            for e in self._by_hash.values()
        ]
        avg_gas_price = sum(gas_prices) / len(gas_prices) if gas_prices else 0
        
        oldest_age = 0
        if self._by_hash:
            oldest = min(e.added_at for e in self._by_hash.values())
            oldest_age = now - oldest
        
        return MempoolStats(
            total_pending=len(self._by_hash),
            by_lane=by_lane,
            total_gas=total_gas,
            avg_gas_price=avg_gas_price,
            oldest_tx_age_ms=oldest_age
        )
    
    def get_quarantined(self) -> List[Dict[str, Any]]:
        """Get all quarantined transactions"""
        return [
            {
                'tx_hash': e.transaction.hash,
                'sender': e.transaction.transaction.sender,
                'risk_score': e.risk_score.to_dict() if e.risk_score else None,
                'added_at': e.added_at
            }
            for e in self._lanes[MempoolLane.QUARANTINE]
        ]
