import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { LineChart, TrendingUp, TrendingDown, RefreshCw, ArrowUpDown } from "lucide-react";
import axios from "axios";

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;
const API = `${BACKEND_URL}/api`;

function OrderbookPanel({ orderbook }) {
  const maxQty = Math.max(
    ...orderbook.bids.map(b => b.quantity),
    ...orderbook.asks.map(a => a.quantity),
    1
  );
  
  return (
    <div className="card rounded-sm h-full" data-testid="orderbook-panel">
      <div className="p-4 border-b border-zinc-800 flex items-center justify-between">
        <h3 className="font-unbounded text-lg font-semibold">Orderbook</h3>
        <span className="font-mono text-xs text-zinc-500">
          Spread: {orderbook.spread?.toFixed(4) || '0.0000'}
        </span>
      </div>
      
      {/* Asks (sells) - reverse to show lowest at bottom */}
      <div className="max-h-48 overflow-y-auto">
        <div className="table-header sticky top-0 bg-zinc-950">
          <div className="grid grid-cols-3 gap-2 px-4 py-2 text-xs">
            <div>PRICE</div>
            <div className="text-right">QTY</div>
            <div className="text-right">TOTAL</div>
          </div>
        </div>
        {[...orderbook.asks].reverse().map((ask, i) => (
          <div
            key={`ask-${i}`}
            className="grid grid-cols-3 gap-2 px-4 py-1 text-sm relative"
          >
            <div
              className="absolute inset-0 bg-red-500/10"
              style={{ width: `${(ask.quantity / maxQty) * 100}%`, right: 0, left: 'auto' }}
            />
            <span className="font-mono text-red-500 relative z-10">{ask.price.toFixed(4)}</span>
            <span className="font-mono text-zinc-400 text-right relative z-10">{ask.quantity.toFixed(2)}</span>
            <span className="font-mono text-zinc-500 text-right relative z-10">{(ask.price * ask.quantity).toFixed(2)}</span>
          </div>
        ))}
      </div>
      
      {/* Current price */}
      <div className="px-4 py-3 border-y border-zinc-800 bg-zinc-900/50">
        <div className="flex items-center justify-center gap-2">
          <span className="font-unbounded text-2xl font-bold text-cyan-500">
            {orderbook.last_price?.toFixed(4) || '1.0000'}
          </span>
          <span className="font-mono text-xs text-zinc-500">USDC</span>
        </div>
      </div>
      
      {/* Bids (buys) */}
      <div className="max-h-48 overflow-y-auto">
        {orderbook.bids.map((bid, i) => (
          <div
            key={`bid-${i}`}
            className="grid grid-cols-3 gap-2 px-4 py-1 text-sm relative"
          >
            <div
              className="absolute inset-0 bg-green-500/10"
              style={{ width: `${(bid.quantity / maxQty) * 100}%` }}
            />
            <span className="font-mono text-green-500 relative z-10">{bid.price.toFixed(4)}</span>
            <span className="font-mono text-zinc-400 text-right relative z-10">{bid.quantity.toFixed(2)}</span>
            <span className="font-mono text-zinc-500 text-right relative z-10">{(bid.price * bid.quantity).toFixed(2)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function TradeForm({ market, onPlaceOrder }) {
  const [side, setSide] = useState("buy");
  const [orderType, setOrderType] = useState("limit");
  const [price, setPrice] = useState("");
  const [quantity, setQuantity] = useState("");
  const [loading, setLoading] = useState(false);
  
  const handleSubmit = async (e) => {
    e.preventDefault();
    setLoading(true);
    await onPlaceOrder({ side, orderType, price: parseFloat(price), quantity: parseFloat(quantity) });
    setLoading(false);
  };
  
  return (
    <div className="card rounded-sm" data-testid="trade-form">
      <div className="p-4 border-b border-zinc-800">
        <h3 className="font-unbounded text-lg font-semibold">Place Order</h3>
      </div>
      <div className="p-4">
        {/* Side selector */}
        <div className="grid grid-cols-2 gap-2 mb-4">
          <button
            onClick={() => setSide("buy")}
            className={`py-3 rounded-sm font-mono text-sm transition-colors ${
              side === "buy"
                ? "bg-green-500 text-black"
                : "bg-zinc-900 text-zinc-400 hover:bg-zinc-800"
            }`}
            data-testid="buy-btn"
          >
            BUY
          </button>
          <button
            onClick={() => setSide("sell")}
            className={`py-3 rounded-sm font-mono text-sm transition-colors ${
              side === "sell"
                ? "bg-red-500 text-black"
                : "bg-zinc-900 text-zinc-400 hover:bg-zinc-800"
            }`}
            data-testid="sell-btn"
          >
            SELL
          </button>
        </div>
        
        {/* Order type */}
        <div className="flex gap-2 mb-4">
          {["limit", "market"].map((type) => (
            <button
              key={type}
              onClick={() => setOrderType(type)}
              className={`px-3 py-1 rounded-sm font-mono text-xs uppercase ${
                orderType === type
                  ? "bg-cyan-500/20 text-cyan-500 border border-cyan-500/30"
                  : "bg-zinc-900 text-zinc-500 border border-zinc-800"
              }`}
            >
              {type}
            </button>
          ))}
        </div>
        
        <form onSubmit={handleSubmit} className="space-y-4">
          {orderType === "limit" && (
            <div>
              <label className="font-mono text-xs text-zinc-500 block mb-2">PRICE (USDC)</label>
              <input
                type="number"
                value={price}
                onChange={(e) => setPrice(e.target.value)}
                placeholder="0.0000"
                step="0.0001"
                className="input w-full rounded-sm"
                data-testid="price-input"
              />
            </div>
          )}
          <div>
            <label className="font-mono text-xs text-zinc-500 block mb-2">QUANTITY (JJ)</label>
            <input
              type="number"
              value={quantity}
              onChange={(e) => setQuantity(e.target.value)}
              placeholder="0.00"
              step="0.01"
              className="input w-full rounded-sm"
              data-testid="quantity-input"
            />
          </div>
          
          {price && quantity && (
            <div className="p-3 bg-zinc-900 rounded-sm">
              <div className="flex justify-between text-xs">
                <span className="text-zinc-500">Total</span>
                <span className="font-mono text-white">
                  {(parseFloat(price || 0) * parseFloat(quantity || 0)).toFixed(2)} USDC
                </span>
              </div>
            </div>
          )}
          
          <button
            type="submit"
            disabled={loading || !quantity || (orderType === "limit" && !price)}
            className={`w-full py-3 rounded-sm font-mono text-sm uppercase font-bold transition-all ${
              side === "buy"
                ? "bg-green-500 hover:bg-green-400 text-black"
                : "bg-red-500 hover:bg-red-400 text-black"
            } disabled:opacity-50`}
            data-testid="place-order-btn"
          >
            {loading ? (
              <RefreshCw className="w-4 h-4 animate-spin mx-auto" />
            ) : (
              `${side} JJ`
            )}
          </button>
        </form>
      </div>
    </div>
  );
}

function RecentTrades({ trades }) {
  return (
    <div className="card rounded-sm" data-testid="recent-trades">
      <div className="p-4 border-b border-zinc-800">
        <h3 className="font-unbounded text-lg font-semibold">Recent Trades</h3>
      </div>
      <div className="max-h-64 overflow-y-auto">
        <div className="table-header sticky top-0 bg-zinc-950">
          <div className="grid grid-cols-4 gap-2 px-4 py-2 text-xs">
            <div>PRICE</div>
            <div className="text-right">QTY</div>
            <div className="text-right">VALUE</div>
            <div className="text-right">TIME</div>
          </div>
        </div>
        {trades.length === 0 ? (
          <div className="p-8 text-center text-zinc-500">
            <ArrowUpDown className="w-8 h-8 mx-auto mb-2 opacity-50" />
            <p className="font-mono text-xs">No trades yet</p>
          </div>
        ) : (
          trades.map((trade, i) => (
            <div key={trade.trade_id || i} className="grid grid-cols-4 gap-2 px-4 py-2 text-sm border-b border-zinc-800/50">
              <span className={`font-mono ${trade.side === "buy" ? "text-green-500" : "text-red-500"}`}>
                {trade.price.toFixed(4)}
              </span>
              <span className="font-mono text-zinc-400 text-right">{trade.quantity.toFixed(2)}</span>
              <span className="font-mono text-zinc-500 text-right">{(trade.price * trade.quantity).toFixed(2)}</span>
              <span className="font-mono text-zinc-500 text-right text-xs">
                {new Date(trade.timestamp).toLocaleTimeString()}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function MarketStats({ stats }) {
  return (
    <div className="card rounded-sm p-4" data-testid="market-stats">
      <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
        <div>
          <p className="font-mono text-xs text-zinc-500">LAST PRICE</p>
          <p className="font-unbounded text-xl font-bold text-white">{stats?.last_price?.toFixed(4) || '1.0000'}</p>
        </div>
        <div>
          <p className="font-mono text-xs text-zinc-500">24H HIGH</p>
          <p className="font-mono text-lg text-green-500">{stats?.high_24h?.toFixed(4) || '0.0000'}</p>
        </div>
        <div>
          <p className="font-mono text-xs text-zinc-500">24H LOW</p>
          <p className="font-mono text-lg text-red-500">{stats?.low_24h?.toFixed(4) || '0.0000'}</p>
        </div>
        <div>
          <p className="font-mono text-xs text-zinc-500">24H VOLUME</p>
          <p className="font-mono text-lg text-white">{stats?.volume_24h?.toFixed(2) || '0.00'}</p>
        </div>
        <div>
          <p className="font-mono text-xs text-zinc-500">OPEN ORDERS</p>
          <p className="font-mono text-lg text-cyan-500">{stats?.total_orders || 0}</p>
        </div>
      </div>
    </div>
  );
}

export default function DEXPage() {
  const [market] = useState("JJ/USDC");
  const [orderbook, setOrderbook] = useState({ bids: [], asks: [], last_price: 1.0, spread: 0 });
  const [trades, setTrades] = useState([]);
  const [stats, setStats] = useState(null);
  const [orderResult, setOrderResult] = useState(null);
  
  // Fetch data
  useEffect(() => {
    const fetchData = async () => {
      try {
        const [obRes, tradesRes, marketsRes] = await Promise.all([
          axios.get(`${API}/dex/orderbook/${market}`),
          axios.get(`${API}/dex/trades/${market}`),
          axios.get(`${API}/dex/markets`)
        ]);
        setOrderbook(obRes.data);
        setTrades(tradesRes.data.trades || []);
        setStats(marketsRes.data.markets?.find(m => m.market === market));
      } catch (e) {
        console.error("DEX fetch error:", e);
      }
    };
    
    fetchData();
    const interval = setInterval(fetchData, 2000);
    return () => clearInterval(interval);
  }, [market]);
  
  const handlePlaceOrder = async ({ side, orderType, price, quantity }) => {
    // Get a wallet from localStorage
    const savedWallets = localStorage.getItem("jasprWallets");
    const wallets = savedWallets ? JSON.parse(savedWallets) : [];
    
    if (wallets.length === 0) {
      setOrderResult({ success: false, error: "Please create a wallet first" });
      return;
    }
    
    try {
      const response = await axios.post(`${API}/dex/order`, {
        trader: wallets[0].address,
        market,
        side,
        order_type: orderType,
        price: price || 1.0,
        quantity
      });
      setOrderResult({ success: true, data: response.data });
    } catch (e) {
      setOrderResult({ success: false, error: e.response?.data?.detail || "Order failed" });
    }
  };
  
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="space-y-6"
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h1 className="font-unbounded text-2xl font-bold">{market}</h1>
          <span className="badge-success">Hybrid DEX</span>
        </div>
      </div>
      
      {/* Market stats */}
      <MarketStats stats={stats} />
      
      {/* Order result notification */}
      {orderResult && (
        <div className={`p-4 rounded-sm ${
          orderResult.success ? "bg-green-500/10 border border-green-500/20" : "bg-red-500/10 border border-red-500/20"
        }`}>
          {orderResult.success ? (
            <p className="font-mono text-sm text-green-500">
              Order placed! {orderResult.data.trades?.length > 0 && `${orderResult.data.trades.length} trades executed`}
            </p>
          ) : (
            <p className="font-mono text-sm text-red-500">{orderResult.error}</p>
          )}
        </div>
      )}
      
      {/* Main trading interface */}
      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        <div className="lg:col-span-1">
          <OrderbookPanel orderbook={orderbook} />
        </div>
        <div className="lg:col-span-2">
          {/* Chart placeholder */}
          <div className="card rounded-sm h-96 flex items-center justify-center">
            <div className="text-center">
              <LineChart className="w-16 h-16 mx-auto text-zinc-700 mb-4" />
              <p className="font-mono text-sm text-zinc-500">TradingView Chart</p>
              <p className="font-mono text-xs text-zinc-600 mt-2">Integration pending</p>
            </div>
          </div>
        </div>
        <div className="lg:col-span-1 space-y-6">
          <TradeForm market={market} onPlaceOrder={handlePlaceOrder} />
          <RecentTrades trades={trades} />
        </div>
      </div>
    </motion.div>
  );
}
