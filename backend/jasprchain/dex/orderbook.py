"""Hybrid Orderbook Implementation
Mirrors Rust DEX module

Features from spec:
- Off-chain matching for millisecond updates
- On-chain settlement ensures truthful fills
- Zero hidden PnL manipulation
"""
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, Tuple
from enum import Enum
from datetime import datetime, timezone
import hashlib
import bisect
from collections import defaultdict


class OrderSide(Enum):
    BUY = "buy"
    SELL = "sell"


class OrderType(Enum):
    LIMIT = "limit"
    MARKET = "market"
    STOP_LIMIT = "stop_limit"
    STOP_MARKET = "stop_market"


class OrderStatus(Enum):
    OPEN = "open"
    PARTIAL = "partial"
    FILLED = "filled"
    CANCELLED = "cancelled"
    EXPIRED = "expired"


@dataclass
class Order:
    """Order in the hybrid orderbook"""
    order_id: str
    trader: str
    market: str  # e.g., "JJ/USDC"
    side: OrderSide
    order_type: OrderType
    price: float  # In quote currency
    quantity: float  # In base currency
    filled_quantity: float = 0.0
    status: OrderStatus = OrderStatus.OPEN
    
    # Timestamps
    created_at: int = field(
        default_factory=lambda: int(datetime.now(timezone.utc).timestamp() * 1000)
    )
    updated_at: int = 0
    expires_at: int = 0  # 0 = no expiry
    
    # Settlement tracking
    settlement_hash: Optional[str] = None
    
    @property
    def remaining_quantity(self) -> float:
        return self.quantity - self.filled_quantity
    
    @property
    def is_active(self) -> bool:
        return self.status in [OrderStatus.OPEN, OrderStatus.PARTIAL]
    
    def to_dict(self) -> dict:
        return {
            'order_id': self.order_id,
            'trader': self.trader,
            'market': self.market,
            'side': self.side.value,
            'type': self.order_type.value,
            'price': self.price,
            'quantity': self.quantity,
            'filled_quantity': self.filled_quantity,
            'remaining_quantity': self.remaining_quantity,
            'status': self.status.value,
            'created_at': self.created_at,
            'updated_at': self.updated_at,
            'settlement_hash': self.settlement_hash
        }


@dataclass
class Trade:
    """Executed trade between two orders"""
    trade_id: str
    market: str
    maker_order_id: str
    taker_order_id: str
    maker: str
    taker: str
    side: OrderSide  # Taker's side
    price: float
    quantity: float
    timestamp: int = field(
        default_factory=lambda: int(datetime.now(timezone.utc).timestamp() * 1000)
    )
    settlement_status: str = "pending"
    
    def to_dict(self) -> dict:
        return {
            'trade_id': self.trade_id,
            'market': self.market,
            'maker_order_id': self.maker_order_id,
            'taker_order_id': self.taker_order_id,
            'maker': self.maker,
            'taker': self.taker,
            'side': self.side.value,
            'price': self.price,
            'quantity': self.quantity,
            'timestamp': self.timestamp,
            'settlement_status': self.settlement_status
        }


class PriceLevel:
    """Price level in the orderbook"""
    def __init__(self, price: float):
        self.price = price
        self.orders: List[Order] = []
        self.total_quantity = 0.0
    
    def add_order(self, order: Order):
        self.orders.append(order)
        self.total_quantity += order.remaining_quantity
    
    def remove_order(self, order_id: str) -> Optional[Order]:
        for i, order in enumerate(self.orders):
            if order.order_id == order_id:
                removed = self.orders.pop(i)
                self.total_quantity -= removed.remaining_quantity
                return removed
        return None
    
    def update_quantities(self):
        self.total_quantity = sum(o.remaining_quantity for o in self.orders)


class HybridOrderbook:
    """Hybrid orderbook with off-chain matching
    
    Implements JasprChain DEX spec:
    - Millisecond order updates (in-memory)
    - Deterministic matching for on-chain settlement
    - No hidden manipulation
    """
    
    def __init__(self, market: str):
        self.market = market
        
        # Bids sorted descending (highest first)
        self._bids: Dict[float, PriceLevel] = {}
        self._bid_prices: List[float] = []  # Sorted descending
        
        # Asks sorted ascending (lowest first)
        self._asks: Dict[float, PriceLevel] = {}
        self._ask_prices: List[float] = []  # Sorted ascending
        
        # All orders by ID
        self._orders: Dict[str, Order] = {}
        
        # Trade history
        self._trades: List[Trade] = []
        
        # Stats
        self._last_price: float = 0.0
        self._volume_24h: float = 0.0
        self._high_24h: float = 0.0
        self._low_24h: float = float('inf')
    
    def place_order(self, order: Order) -> Tuple[Order, List[Trade]]:
        """Place an order and match against book
        
        Returns (updated_order, list_of_trades)
        """
        trades = []
        
        if order.order_type == OrderType.MARKET:
            trades = self._match_market_order(order)
        else:
            trades = self._match_limit_order(order)
        
        # Add remaining quantity to book if limit order
        if order.order_type == OrderType.LIMIT and order.remaining_quantity > 0:
            self._add_to_book(order)
        
        self._orders[order.order_id] = order
        self._trades.extend(trades)
        
        return order, trades
    
    def cancel_order(self, order_id: str) -> Optional[Order]:
        """Cancel an order"""
        order = self._orders.get(order_id)
        if not order or not order.is_active:
            return None
        
        self._remove_from_book(order)
        order.status = OrderStatus.CANCELLED
        order.updated_at = int(datetime.now(timezone.utc).timestamp() * 1000)
        
        return order
    
    def _match_limit_order(self, order: Order) -> List[Trade]:
        """Match a limit order against the book"""
        trades = []
        
        if order.side == OrderSide.BUY:
            # Match against asks
            while order.remaining_quantity > 0 and self._ask_prices:
                best_ask = self._ask_prices[0]
                if order.price < best_ask:
                    break  # No more matchable prices
                
                trade = self._match_at_price(order, best_ask, self._asks)
                if trade:
                    trades.append(trade)
                else:
                    break
        else:
            # Match against bids
            while order.remaining_quantity > 0 and self._bid_prices:
                best_bid = self._bid_prices[0]
                if order.price > best_bid:
                    break  # No more matchable prices
                
                trade = self._match_at_price(order, best_bid, self._bids)
                if trade:
                    trades.append(trade)
                else:
                    break
        
        # Update order status
        if order.filled_quantity > 0:
            if order.remaining_quantity == 0:
                order.status = OrderStatus.FILLED
            else:
                order.status = OrderStatus.PARTIAL
        
        return trades
    
    def _match_market_order(self, order: Order) -> List[Trade]:
        """Match a market order (takes best available prices)"""
        trades = []
        
        if order.side == OrderSide.BUY:
            prices = self._ask_prices
            book = self._asks
        else:
            prices = self._bid_prices
            book = self._bids
        
        while order.remaining_quantity > 0 and prices:
            best_price = prices[0]
            trade = self._match_at_price(order, best_price, book)
            if trade:
                trades.append(trade)
            else:
                break
        
        if order.filled_quantity > 0:
            order.status = OrderStatus.FILLED if order.remaining_quantity == 0 else OrderStatus.PARTIAL
        
        return trades
    
    def _match_at_price(self, taker_order: Order, price: float, book: Dict[float, PriceLevel]) -> Optional[Trade]:
        """Match taker order at a specific price level"""
        level = book.get(price)
        if not level or not level.orders:
            return None
        
        maker_order = level.orders[0]
        
        # Determine fill quantity
        fill_qty = min(taker_order.remaining_quantity, maker_order.remaining_quantity)
        
        if fill_qty <= 0:
            return None
        
        # Create trade
        trade = Trade(
            trade_id=hashlib.sha256(
                f"{maker_order.order_id}:{taker_order.order_id}:{datetime.now(timezone.utc).timestamp()}".encode()
            ).hexdigest()[:16],
            market=self.market,
            maker_order_id=maker_order.order_id,
            taker_order_id=taker_order.order_id,
            maker=maker_order.trader,
            taker=taker_order.trader,
            side=taker_order.side,
            price=price,
            quantity=fill_qty
        )
        
        # Update orders
        maker_order.filled_quantity += fill_qty
        taker_order.filled_quantity += fill_qty
        
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        maker_order.updated_at = now
        taker_order.updated_at = now
        
        # Update maker status
        if maker_order.remaining_quantity == 0:
            maker_order.status = OrderStatus.FILLED
            level.orders.pop(0)
        else:
            maker_order.status = OrderStatus.PARTIAL
        
        level.update_quantities()
        
        # Remove empty price level
        if not level.orders:
            del book[price]
            if taker_order.side == OrderSide.BUY:
                self._ask_prices.remove(price)
            else:
                self._bid_prices.remove(price)
        
        # Update market stats
        self._last_price = price
        self._volume_24h += fill_qty * price
        self._high_24h = max(self._high_24h, price)
        self._low_24h = min(self._low_24h, price)
        
        return trade
    
    def _add_to_book(self, order: Order):
        """Add order to the orderbook"""
        price = order.price
        
        if order.side == OrderSide.BUY:
            if price not in self._bids:
                self._bids[price] = PriceLevel(price)
                bisect.insort(self._bid_prices, price)
                self._bid_prices.reverse()  # Keep descending
            self._bids[price].add_order(order)
        else:
            if price not in self._asks:
                self._asks[price] = PriceLevel(price)
                bisect.insort(self._ask_prices, price)
            self._asks[price].add_order(order)
    
    def _remove_from_book(self, order: Order):
        """Remove order from the orderbook"""
        price = order.price
        
        if order.side == OrderSide.BUY and price in self._bids:
            self._bids[price].remove_order(order.order_id)
            if not self._bids[price].orders:
                del self._bids[price]
                self._bid_prices.remove(price)
        elif order.side == OrderSide.SELL and price in self._asks:
            self._asks[price].remove_order(order.order_id)
            if not self._asks[price].orders:
                del self._asks[price]
                self._ask_prices.remove(price)
    
    def get_order(self, order_id: str) -> Optional[Order]:
        return self._orders.get(order_id)
    
    def get_orderbook_snapshot(self, depth: int = 20) -> Dict[str, Any]:
        """Get orderbook snapshot for API"""
        bids = []
        for price in self._bid_prices[:depth]:
            level = self._bids[price]
            bids.append({
                'price': price,
                'quantity': level.total_quantity,
                'orders': len(level.orders)
            })
        
        asks = []
        for price in self._ask_prices[:depth]:
            level = self._asks[price]
            asks.append({
                'price': price,
                'quantity': level.total_quantity,
                'orders': len(level.orders)
            })
        
        return {
            'market': self.market,
            'bids': bids,
            'asks': asks,
            'last_price': self._last_price,
            'spread': (self._ask_prices[0] - self._bid_prices[0]) if self._bid_prices and self._ask_prices else 0
        }
    
    def get_recent_trades(self, limit: int = 50) -> List[Dict]:
        return [t.to_dict() for t in self._trades[-limit:]]
    
    def get_market_stats(self) -> Dict[str, Any]:
        return {
            'market': self.market,
            'last_price': self._last_price,
            'high_24h': self._high_24h if self._high_24h > 0 else 0,
            'low_24h': self._low_24h if self._low_24h < float('inf') else 0,
            'volume_24h': self._volume_24h,
            'bid_count': sum(len(l.orders) for l in self._bids.values()),
            'ask_count': sum(len(l.orders) for l in self._asks.values()),
            'total_orders': len([o for o in self._orders.values() if o.is_active]),
            'total_trades': len(self._trades)
        }
