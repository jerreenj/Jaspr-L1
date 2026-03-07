import React, { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { TrendingUp, TrendingDown, ArrowUpRight, ArrowDownRight, Loader2, CheckCircle, XCircle } from 'lucide-react';
import axios from 'axios';

const API = process.env.REACT_APP_BACKEND_URL + '/api';
const JASPR_TREASURY = 'jaspr1treasury000000000000000000000000000000000';

const SYMBOLS = [
  { symbol: 'BTC', name: 'Bitcoin', price: 67432.50 },
  { symbol: 'ETH', name: 'Ethereum', price: 3521.80 },
  { symbol: 'BNB', name: 'BNB', price: 584.20 },
  { symbol: 'SOL', name: 'Solana', price: 142.30 },
  { symbol: 'XRP', name: 'Ripple', price: 0.52 },
];

export default function TradePage() {
  const [wallets, setWallets] = useState([]);
  const [selectedWallet, setSelectedWallet] = useState(null);
  const [selectedSymbol, setSelectedSymbol] = useState(SYMBOLS[0]);
  const [mode, setMode] = useState('BUY');
  const [amount, setAmount] = useState('');
  const [executing, setExecuting] = useState(false);
  const [result, setResult] = useState(null);

  useEffect(() => {
    loadWallets();
  }, []);

  const loadWallets = async () => {
    try {
      const res = await axios.get(`${API}/wallets`);
      if (res.data.wallets?.length > 0) {
        setWallets(res.data.wallets);
        setSelectedWallet(res.data.wallets[0]);
      }
    } catch (e) {
      console.error('Failed to load wallets');
    }
  };

  const executeTrade = async () => {
    if (!selectedWallet || !amount || parseFloat(amount) <= 0) return;
    
    setExecuting(true);
    setResult(null);
    
    try {
      const usdValue = parseFloat(amount);
      const jasprAmount = Math.max(1, Math.floor(usdValue));
      
      // Execute REAL trade on JasprChain
      const response = await axios.post(`${API}/transactions/trade`, {
        sender: selectedWallet.address,
        recipient: JASPR_TREASURY,
        amount: jasprAmount,
        trade_type: mode.toLowerCase(),
        symbol: selectedSymbol.symbol
      });
      
      setResult({
        success: true,
        tx_hash: response.data.tx_hash,
        type: mode,
        symbol: selectedSymbol.symbol,
        amount: usdValue
      });
      setAmount('');
    } catch (e) {
      setResult({
        success: false,
        error: e.response?.data?.detail || 'Trade failed'
      });
    } finally {
      setExecuting(false);
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className="space-y-6"
    >
      <h1 className="font-unbounded text-2xl font-bold">Trade</h1>
      
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Trade Panel */}
        <div className="card p-6 space-y-6">
          {/* Mode Toggle */}
          <div className="flex gap-2">
            <button
              onClick={() => setMode('BUY')}
              className={`flex-1 py-3 rounded-lg font-semibold flex items-center justify-center gap-2 transition-all ${
                mode === 'BUY' 
                  ? 'bg-green-500/20 text-green-400 border border-green-500/50' 
                  : 'bg-dark-700 text-gray-400 hover:bg-dark-600'
              }`}
              data-testid="trade-buy-btn"
            >
              <TrendingUp className="w-5 h-5" />
              BUY / LONG
            </button>
            <button
              onClick={() => setMode('SELL')}
              className={`flex-1 py-3 rounded-lg font-semibold flex items-center justify-center gap-2 transition-all ${
                mode === 'SELL' 
                  ? 'bg-red-500/20 text-red-400 border border-red-500/50' 
                  : 'bg-dark-700 text-gray-400 hover:bg-dark-600'
              }`}
              data-testid="trade-sell-btn"
            >
              <TrendingDown className="w-5 h-5" />
              SELL / SHORT
            </button>
          </div>

          {/* Symbol Selection */}
          <div>
            <label className="text-sm text-gray-400 mb-2 block">Select Asset</label>
            <div className="grid grid-cols-5 gap-2">
              {SYMBOLS.map((s) => (
                <button
                  key={s.symbol}
                  onClick={() => setSelectedSymbol(s)}
                  className={`p-3 rounded-lg text-center transition-all ${
                    selectedSymbol.symbol === s.symbol
                      ? 'bg-primary/20 border border-primary/50 text-primary'
                      : 'bg-dark-700 hover:bg-dark-600'
                  }`}
                  data-testid={`symbol-${s.symbol.toLowerCase()}`}
                >
                  <div className="font-bold">{s.symbol}</div>
                  <div className="text-xs text-gray-400">${s.price.toLocaleString()}</div>
                </button>
              ))}
            </div>
          </div>

          {/* Wallet Selection */}
          <div>
            <label className="text-sm text-gray-400 mb-2 block">Wallet</label>
            {wallets.length > 0 ? (
              <select
                value={selectedWallet?.address || ''}
                onChange={(e) => setSelectedWallet(wallets.find(w => w.address === e.target.value))}
                className="w-full bg-dark-700 border border-dark-600 rounded-lg p-3"
                data-testid="trade-wallet-select"
              >
                {wallets.map((w) => (
                  <option key={w.address} value={w.address}>
                    {w.address.slice(0, 15)}... ({w.balance?.toLocaleString() || 0} JASPR)
                  </option>
                ))}
              </select>
            ) : (
              <div className="text-gray-400 text-sm">No wallets. Create one in Wallet page.</div>
            )}
          </div>

          {/* Amount Input */}
          <div>
            <label className="text-sm text-gray-400 mb-2 block">Amount (USD)</label>
            <input
              type="number"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder="Enter amount in USD"
              className="w-full bg-dark-700 border border-dark-600 rounded-lg p-3 text-lg"
              data-testid="trade-amount-input"
            />
            <div className="text-xs text-gray-500 mt-1">1 USD = 1 JASPR (no decimals)</div>
          </div>

          {/* Execute Button */}
          <button
            onClick={executeTrade}
            disabled={executing || !selectedWallet || !amount}
            className={`w-full py-4 rounded-lg font-bold text-lg flex items-center justify-center gap-2 transition-all ${
              mode === 'BUY'
                ? 'bg-green-500 hover:bg-green-600 disabled:bg-green-500/30'
                : 'bg-red-500 hover:bg-red-600 disabled:bg-red-500/30'
            }`}
            data-testid="execute-trade-btn"
          >
            {executing ? (
              <Loader2 className="w-5 h-5 animate-spin" />
            ) : mode === 'BUY' ? (
              <ArrowUpRight className="w-5 h-5" />
            ) : (
              <ArrowDownRight className="w-5 h-5" />
            )}
            {executing ? 'Executing...' : `${mode} ${selectedSymbol.symbol}`}
          </button>

          {/* Result */}
          {result && (
            <div className={`p-4 rounded-lg ${result.success ? 'bg-green-500/10 border border-green-500/30' : 'bg-red-500/10 border border-red-500/30'}`}>
              {result.success ? (
                <div className="flex items-start gap-3">
                  <CheckCircle className="w-5 h-5 text-green-400 mt-0.5" />
                  <div>
                    <div className="font-semibold text-green-400">Trade Executed!</div>
                    <div className="text-sm text-gray-300 mt-1">
                      {result.type} {result.symbol} for ${result.amount}
                    </div>
                    <div className="text-xs text-gray-400 mt-1 font-mono break-all">
                      TX: {result.tx_hash}
                    </div>
                  </div>
                </div>
              ) : (
                <div className="flex items-center gap-2 text-red-400">
                  <XCircle className="w-5 h-5" />
                  {result.error}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Info Panel */}
        <div className="card p-6">
          <h3 className="font-semibold mb-4">How Trading Works</h3>
          <div className="space-y-3 text-sm text-gray-400">
            <p>• Every trade creates a REAL transaction on JasprChain</p>
            <p>• BUY/LONG: Opens a long position on the asset</p>
            <p>• SELL/SHORT: Opens a short position on the asset</p>
            <p>• Amount in USD = Amount in JASPR (1:1)</p>
            <p>• All trades are recorded on-chain with full transparency</p>
            <p>• View your trade history in the History page</p>
          </div>
        </div>
      </div>
    </motion.div>
  );
}
