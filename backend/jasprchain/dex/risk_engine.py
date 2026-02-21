"""DEX Risk Engine
Mirrors Rust risk engine module

Features from spec:
- Perp margining, solvency checks, mark-price guardrails
- Runtime enforceable, visible to users
- Eliminates opaque CEX controls
"""
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any
from datetime import datetime, timezone


@dataclass
class Position:
    """Trading position for margin accounts"""
    position_id: str
    trader: str
    market: str
    side: str  # 'long' or 'short'
    size: float  # Position size in base currency
    entry_price: float
    mark_price: float = 0.0
    
    # Margin
    margin: float = 0.0  # Collateral deposited
    leverage: float = 1.0
    
    # P&L
    unrealized_pnl: float = 0.0
    realized_pnl: float = 0.0
    
    # Risk metrics
    liquidation_price: float = 0.0
    margin_ratio: float = 1.0
    
    # Timestamps
    opened_at: int = field(
        default_factory=lambda: int(datetime.now(timezone.utc).timestamp() * 1000)
    )
    updated_at: int = 0
    
    def update_mark_price(self, mark_price: float):
        """Update position with new mark price"""
        self.mark_price = mark_price
        
        # Calculate unrealized P&L
        if self.side == 'long':
            self.unrealized_pnl = (mark_price - self.entry_price) * self.size
        else:
            self.unrealized_pnl = (self.entry_price - mark_price) * self.size
        
        # Update margin ratio
        equity = self.margin + self.unrealized_pnl
        position_value = self.size * mark_price
        self.margin_ratio = equity / position_value if position_value > 0 else 0
        
        self.updated_at = int(datetime.now(timezone.utc).timestamp() * 1000)
    
    def to_dict(self) -> dict:
        return {
            'position_id': self.position_id,
            'trader': self.trader,
            'market': self.market,
            'side': self.side,
            'size': self.size,
            'entry_price': self.entry_price,
            'mark_price': self.mark_price,
            'margin': self.margin,
            'leverage': self.leverage,
            'unrealized_pnl': round(self.unrealized_pnl, 4),
            'realized_pnl': round(self.realized_pnl, 4),
            'liquidation_price': round(self.liquidation_price, 4),
            'margin_ratio': round(self.margin_ratio, 4),
            'opened_at': self.opened_at
        }


@dataclass
class RiskParameters:
    """Risk parameters for a market"""
    market: str
    
    # Margin requirements
    initial_margin_rate: float = 0.1  # 10% = 10x leverage
    maintenance_margin_rate: float = 0.05  # 5%
    
    # Position limits
    max_leverage: float = 20.0
    max_position_size: float = 1000000.0  # In base currency
    
    # Price limits
    max_price_deviation: float = 0.1  # 10% from mark price
    
    # Liquidation
    liquidation_fee_rate: float = 0.005  # 0.5%
    insurance_fund_rate: float = 0.002  # 0.2%


class DEXRiskEngine:
    """Risk management engine for the DEX
    
    Implements:
    - Real-time margin calculations
    - Liquidation detection
    - Price guardrails
    - Solvency checks
    """
    
    def __init__(self):
        self._positions: Dict[str, Position] = {}
        self._risk_params: Dict[str, RiskParameters] = {}
        self._mark_prices: Dict[str, float] = {}  # market -> price
        self._insurance_fund: float = 0.0
        
        # Default risk params
        self._default_params = RiskParameters(market="default")
    
    def set_risk_params(self, market: str, params: RiskParameters):
        """Set risk parameters for a market"""
        self._risk_params[market] = params
    
    def get_risk_params(self, market: str) -> RiskParameters:
        return self._risk_params.get(market, self._default_params)
    
    def update_mark_price(self, market: str, price: float):
        """Update mark price for a market"""
        self._mark_prices[market] = price
        
        # Update all positions in this market
        for position in self._positions.values():
            if position.market == market:
                position.update_mark_price(price)
    
    def open_position(
        self,
        trader: str,
        market: str,
        side: str,
        size: float,
        entry_price: float,
        margin: float
    ) -> tuple:
        """Open a new position
        
        Returns (position, is_valid, message)
        """
        params = self.get_risk_params(market)
        
        # Calculate required margin
        position_value = size * entry_price
        required_margin = position_value * params.initial_margin_rate
        
        if margin < required_margin:
            return None, False, f"Insufficient margin. Required: {required_margin}"
        
        # Check leverage
        leverage = position_value / margin
        if leverage > params.max_leverage:
            return None, False, f"Leverage {leverage}x exceeds max {params.max_leverage}x"
        
        # Check position size
        if size > params.max_position_size:
            return None, False, f"Position size exceeds limit"
        
        # Calculate liquidation price
        if side == 'long':
            liquidation_price = entry_price * (1 - (1 / leverage) + params.maintenance_margin_rate)
        else:
            liquidation_price = entry_price * (1 + (1 / leverage) - params.maintenance_margin_rate)
        
        position = Position(
            position_id=f"{trader}:{market}:{datetime.now(timezone.utc).timestamp()}",
            trader=trader,
            market=market,
            side=side,
            size=size,
            entry_price=entry_price,
            mark_price=self._mark_prices.get(market, entry_price),
            margin=margin,
            leverage=leverage,
            liquidation_price=liquidation_price
        )
        
        position.update_mark_price(position.mark_price)
        self._positions[position.position_id] = position
        
        return position, True, "Position opened"
    
    def check_liquidation(self, position_id: str) -> tuple:
        """Check if position should be liquidated
        
        Returns (should_liquidate, liquidation_price)
        """
        position = self._positions.get(position_id)
        if not position:
            return False, 0
        
        params = self.get_risk_params(position.market)
        
        # Check margin ratio
        if position.margin_ratio < params.maintenance_margin_rate:
            return True, position.mark_price
        
        # Check liquidation price breach
        if position.side == 'long':
            if position.mark_price <= position.liquidation_price:
                return True, position.mark_price
        else:
            if position.mark_price >= position.liquidation_price:
                return True, position.mark_price
        
        return False, 0
    
    def execute_liquidation(self, position_id: str) -> Optional[Dict[str, Any]]:
        """Execute liquidation of a position"""
        position = self._positions.get(position_id)
        if not position:
            return None
        
        params = self.get_risk_params(position.market)
        
        # Calculate liquidation values
        liquidation_value = position.size * position.mark_price
        liquidation_fee = liquidation_value * params.liquidation_fee_rate
        insurance_contribution = liquidation_value * params.insurance_fund_rate
        
        # Update insurance fund
        self._insurance_fund += insurance_contribution
        
        # Close position
        del self._positions[position_id]
        
        return {
            'position_id': position_id,
            'trader': position.trader,
            'market': position.market,
            'liquidation_price': position.mark_price,
            'size': position.size,
            'liquidation_fee': liquidation_fee,
            'insurance_contribution': insurance_contribution,
            'timestamp': int(datetime.now(timezone.utc).timestamp() * 1000)
        }
    
    def get_liquidatable_positions(self) -> List[str]:
        """Get all positions that should be liquidated"""
        liquidatable = []
        for position_id in self._positions:
            should_liq, _ = self.check_liquidation(position_id)
            if should_liq:
                liquidatable.append(position_id)
        return liquidatable
    
    def validate_order_price(self, market: str, price: float) -> tuple:
        """Validate order price against guardrails
        
        Returns (is_valid, message)
        """
        mark_price = self._mark_prices.get(market)
        if not mark_price:
            return True, "No mark price set"  # Allow if no mark price
        
        params = self.get_risk_params(market)
        deviation = abs(price - mark_price) / mark_price
        
        if deviation > params.max_price_deviation:
            return False, f"Price deviates {deviation*100:.1f}% from mark price (max {params.max_price_deviation*100}%)"
        
        return True, "OK"
    
    def get_position(self, position_id: str) -> Optional[Position]:
        return self._positions.get(position_id)
    
    def get_positions_by_trader(self, trader: str) -> List[Position]:
        return [p for p in self._positions.values() if p.trader == trader]
    
    def get_stats(self) -> Dict[str, Any]:
        total_exposure = sum(p.size * p.mark_price for p in self._positions.values())
        total_margin = sum(p.margin for p in self._positions.values())
        total_unrealized_pnl = sum(p.unrealized_pnl for p in self._positions.values())
        
        return {
            'total_positions': len(self._positions),
            'total_exposure': total_exposure,
            'total_margin': total_margin,
            'total_unrealized_pnl': total_unrealized_pnl,
            'insurance_fund': self._insurance_fund,
            'markets_tracked': len(self._mark_prices)
        }
