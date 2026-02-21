"""Settlement Engine for JasprChain DEX
Mirrors Rust settlement module

Features from spec:
- Atomic batch settlement (no partial cancels)
- All fills produce settlement receipts
- Prevents auto-cancel cascades
"""
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any
from datetime import datetime, timezone
import hashlib

from .orderbook import Trade, Order


@dataclass
class Settlement:
    """Settlement record for a trade"""
    settlement_id: str
    trade_id: str
    market: str
    
    # Parties
    buyer: str
    seller: str
    
    # Amounts
    base_amount: float  # e.g., JJ tokens
    quote_amount: float  # e.g., USDC
    price: float
    
    # Fees
    maker_fee: float = 0.0
    taker_fee: float = 0.0
    
    # Status
    status: str = "pending"  # pending, confirmed, finalized
    
    # On-chain references
    block_height: Optional[int] = None
    tx_hash: Optional[str] = None
    state_root: Optional[str] = None
    
    # Timestamps
    created_at: int = field(
        default_factory=lambda: int(datetime.now(timezone.utc).timestamp() * 1000)
    )
    confirmed_at: Optional[int] = None
    finalized_at: Optional[int] = None
    
    def to_dict(self) -> dict:
        return {
            'settlement_id': self.settlement_id,
            'trade_id': self.trade_id,
            'market': self.market,
            'buyer': self.buyer,
            'seller': self.seller,
            'base_amount': self.base_amount,
            'quote_amount': self.quote_amount,
            'price': self.price,
            'maker_fee': self.maker_fee,
            'taker_fee': self.taker_fee,
            'status': self.status,
            'block_height': self.block_height,
            'tx_hash': self.tx_hash,
            'state_root': self.state_root,
            'created_at': self.created_at,
            'confirmed_at': self.confirmed_at,
            'finalized_at': self.finalized_at
        }


@dataclass
class SettlementBatch:
    """Batch of settlements for atomic execution"""
    batch_id: str
    settlements: List[Settlement]
    status: str = "pending"  # pending, processing, committed, failed
    
    # Execution results
    block_height: Optional[int] = None
    state_root_before: Optional[str] = None
    state_root_after: Optional[str] = None
    
    created_at: int = field(
        default_factory=lambda: int(datetime.now(timezone.utc).timestamp() * 1000)
    )
    processed_at: Optional[int] = None
    
    def to_dict(self) -> dict:
        return {
            'batch_id': self.batch_id,
            'settlement_count': len(self.settlements),
            'status': self.status,
            'block_height': self.block_height,
            'state_root_before': self.state_root_before,
            'state_root_after': self.state_root_after,
            'created_at': self.created_at,
            'processed_at': self.processed_at
        }


class SettlementEngine:
    """Handles atomic settlement of DEX trades
    
    Ensures:
    - No partial settlements (atomic)
    - Transparent receipts for all fills
    - Deterministic state transitions
    """
    
    # Fee rates (basis points)
    MAKER_FEE_BPS = 10  # 0.1%
    TAKER_FEE_BPS = 20  # 0.2%
    
    def __init__(self, state_store):
        self.state = state_store
        self._settlements: Dict[str, Settlement] = {}
        self._batches: Dict[str, SettlementBatch] = {}
        self._pending_batch: List[Settlement] = []
    
    def create_settlement(self, trade: Trade) -> Settlement:
        """Create a settlement record from a trade"""
        # Determine buyer/seller
        if trade.side.value == "buy":
            buyer = trade.taker
            seller = trade.maker
        else:
            buyer = trade.maker
            seller = trade.taker
        
        quote_amount = trade.price * trade.quantity
        
        # Calculate fees
        maker_fee = quote_amount * self.MAKER_FEE_BPS / 10000
        taker_fee = quote_amount * self.TAKER_FEE_BPS / 10000
        
        settlement = Settlement(
            settlement_id=hashlib.sha256(
                f"settlement:{trade.trade_id}:{datetime.now(timezone.utc).timestamp()}".encode()
            ).hexdigest()[:16],
            trade_id=trade.trade_id,
            market=trade.market,
            buyer=buyer,
            seller=seller,
            base_amount=trade.quantity,
            quote_amount=quote_amount,
            price=trade.price,
            maker_fee=maker_fee,
            taker_fee=taker_fee
        )
        
        self._settlements[settlement.settlement_id] = settlement
        self._pending_batch.append(settlement)
        
        return settlement
    
    def commit_batch(self, block_height: int, state_root: str) -> Optional[SettlementBatch]:
        """Commit pending settlements as atomic batch"""
        if not self._pending_batch:
            return None
        
        batch = SettlementBatch(
            batch_id=hashlib.sha256(
                f"batch:{block_height}:{datetime.now(timezone.utc).timestamp()}".encode()
            ).hexdigest()[:16],
            settlements=self._pending_batch.copy(),
            status="processing",
            block_height=block_height,
            state_root_before=self.state.root
        )
        
        try:
            # Execute all settlements atomically
            for settlement in batch.settlements:
                self._execute_settlement(settlement)
            
            # Commit state changes
            new_root = self.state.commit()
            
            batch.state_root_after = new_root
            batch.status = "committed"
            batch.processed_at = int(datetime.now(timezone.utc).timestamp() * 1000)
            
            # Update settlement records
            now = int(datetime.now(timezone.utc).timestamp() * 1000)
            for settlement in batch.settlements:
                settlement.status = "confirmed"
                settlement.block_height = block_height
                settlement.state_root = new_root
                settlement.confirmed_at = now
            
            self._batches[batch.batch_id] = batch
            self._pending_batch.clear()
            
            return batch
            
        except Exception as e:
            # Rollback on any failure
            self.state.rollback()
            batch.status = "failed"
            return batch
    
    def _execute_settlement(self, settlement: Settlement):
        """Execute a single settlement (state changes)"""
        # Transfer base token from seller to buyer
        seller_base_key = f"balance:{settlement.seller}:base"
        buyer_base_key = f"balance:{settlement.buyer}:base"
        
        seller_base = self.state.get(seller_base_key, 0)
        buyer_base = self.state.get(buyer_base_key, 0)
        
        self.state.set(seller_base_key, seller_base - settlement.base_amount)
        self.state.set(buyer_base_key, buyer_base + settlement.base_amount)
        
        # Transfer quote token from buyer to seller (minus fees)
        seller_quote_key = f"balance:{settlement.seller}:quote"
        buyer_quote_key = f"balance:{settlement.buyer}:quote"
        
        seller_quote = self.state.get(seller_quote_key, 0)
        buyer_quote = self.state.get(buyer_quote_key, 0)
        
        net_to_seller = settlement.quote_amount - settlement.maker_fee
        total_from_buyer = settlement.quote_amount + settlement.taker_fee
        
        self.state.set(seller_quote_key, seller_quote + net_to_seller)
        self.state.set(buyer_quote_key, buyer_quote - total_from_buyer)
        
        # Collect fees to treasury
        treasury_key = "balance:treasury:quote"
        treasury = self.state.get(treasury_key, 0)
        self.state.set(treasury_key, treasury + settlement.maker_fee + settlement.taker_fee)
    
    def finalize_settlement(self, settlement_id: str):
        """Mark settlement as finalized (after block finality)"""
        settlement = self._settlements.get(settlement_id)
        if settlement and settlement.status == "confirmed":
            settlement.status = "finalized"
            settlement.finalized_at = int(datetime.now(timezone.utc).timestamp() * 1000)
    
    def get_settlement(self, settlement_id: str) -> Optional[Settlement]:
        return self._settlements.get(settlement_id)
    
    def get_settlements_by_address(self, address: str) -> List[Settlement]:
        return [
            s for s in self._settlements.values()
            if s.buyer == address or s.seller == address
        ]
    
    def get_pending_count(self) -> int:
        return len(self._pending_batch)
    
    def get_stats(self) -> Dict[str, Any]:
        total_volume = sum(s.quote_amount for s in self._settlements.values())
        total_fees = sum(s.maker_fee + s.taker_fee for s in self._settlements.values())
        
        return {
            'total_settlements': len(self._settlements),
            'total_batches': len(self._batches),
            'pending_settlements': len(self._pending_batch),
            'total_volume': total_volume,
            'total_fees_collected': total_fees
        }
