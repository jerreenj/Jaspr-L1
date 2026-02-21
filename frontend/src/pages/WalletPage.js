import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { Wallet, Plus, Send, ArrowDownLeft, Copy, Check, RefreshCw, Shield } from "lucide-react";
import axios from "axios";

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;
const API = `${BACKEND_URL}/api`;

export default function WalletPage() {
  const [wallets, setWallets] = useState([]);
  const [selectedWallet, setSelectedWallet] = useState(null);
  const [walletDetails, setWalletDetails] = useState(null);
  const [creating, setCreating] = useState(false);
  const [copied, setCopied] = useState(false);
  
  // Transfer form
  const [recipient, setRecipient] = useState("");
  const [amount, setAmount] = useState("");
  const [transferring, setTransferring] = useState(false);
  const [transferResult, setTransferResult] = useState(null);
  
  // Load wallets from localStorage
  useEffect(() => {
    const saved = localStorage.getItem("jasprWallets");
    if (saved) {
      setWallets(JSON.parse(saved));
    }
  }, []);
  
  // Fetch wallet details
  useEffect(() => {
    if (!selectedWallet) {
      setWalletDetails(null);
      return;
    }
    
    const fetchDetails = async () => {
      try {
        const response = await axios.get(`${API}/wallets/${selectedWallet.address}`);
        setWalletDetails(response.data);
      } catch (e) {
        console.error("Failed to fetch wallet details:", e);
      }
    };
    
    fetchDetails();
    const interval = setInterval(fetchDetails, 5000);
    return () => clearInterval(interval);
  }, [selectedWallet]);
  
  const createWallet = async () => {
    setCreating(true);
    try {
      const response = await axios.post(`${API}/wallets/create`);
      const newWallet = response.data;
      const updated = [...wallets, newWallet];
      setWallets(updated);
      localStorage.setItem("jasprWallets", JSON.stringify(updated));
      setSelectedWallet(newWallet);
    } catch (e) {
      console.error("Failed to create wallet:", e);
    } finally {
      setCreating(false);
    }
  };
  
  const copyAddress = () => {
    navigator.clipboard.writeText(selectedWallet?.address || "");
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };
  
  const handleTransfer = async (e) => {
    e.preventDefault();
    if (!selectedWallet || !recipient || !amount) return;
    
    setTransferring(true);
    setTransferResult(null);
    
    try {
      const response = await axios.post(`${API}/transactions/transfer`, {
        sender: selectedWallet.address,
        recipient: recipient,
        amount: Math.floor(parseFloat(amount) * 1_000_000_000)
      });
      setTransferResult({ success: true, data: response.data });
      setRecipient("");
      setAmount("");
    } catch (e) {
      setTransferResult({ success: false, error: e.response?.data?.detail || "Transfer failed" });
    } finally {
      setTransferring(false);
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
        <h1 className="font-unbounded text-2xl font-bold">Wallet</h1>
        <button
          onClick={createWallet}
          disabled={creating}
          className="btn-primary flex items-center gap-2"
          data-testid="create-wallet-btn"
        >
          {creating ? (
            <RefreshCw className="w-4 h-4 animate-spin" />
          ) : (
            <Plus className="w-4 h-4" />
          )}
          Create Wallet
        </button>
      </div>
      
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Wallet list */}
        <div className="card rounded-sm" data-testid="wallet-list">
          <div className="p-4 border-b border-zinc-800">
            <h3 className="font-unbounded text-lg font-semibold">Your Wallets</h3>
          </div>
          <div className="divide-y divide-zinc-800">
            {wallets.length === 0 ? (
              <div className="p-8 text-center text-zinc-500">
                <Wallet className="w-12 h-12 mx-auto mb-4 opacity-50" />
                <p className="font-manrope">No wallets yet</p>
                <p className="font-mono text-xs mt-2">Create your first MPC wallet</p>
              </div>
            ) : (
              wallets.map((wallet) => (
                <button
                  key={wallet.address}
                  onClick={() => setSelectedWallet(wallet)}
                  className={`w-full p-4 text-left transition-colors ${
                    selectedWallet?.address === wallet.address
                      ? "bg-cyan-500/10 border-l-2 border-cyan-500"
                      : "hover:bg-zinc-900"
                  }`}
                  data-testid={`wallet-item-${wallet.address.slice(0, 8)}`}
                >
                  <div className="flex items-center gap-3">
                    <div className="w-10 h-10 rounded-sm bg-gradient-to-br from-cyan-500 to-purple-600 flex items-center justify-center">
                      <Wallet className="w-5 h-5 text-black" />
                    </div>
                    <div>
                      <p className="font-mono text-sm text-white">
                        {wallet.address.slice(0, 12)}...{wallet.address.slice(-4)}
                      </p>
                      <p className="font-mono text-xs text-zinc-500">{wallet.threshold}</p>
                    </div>
                  </div>
                </button>
              ))
            )}
          </div>
        </div>
        
        {/* Wallet details */}
        <div className="lg:col-span-2 space-y-6">
          {selectedWallet ? (
            <>
              {/* Balance card */}
              <div className="card rounded-sm p-6 bg-gradient-to-br from-cyan-500/10 to-purple-500/10">
                <div className="flex items-start justify-between">
                  <div>
                    <p className="font-mono text-xs text-zinc-500 uppercase tracking-widest">Balance</p>
                    <p className="font-unbounded text-4xl font-bold mt-2">
                      {walletDetails?.balance_formatted || "0.0000 JASPR"}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Shield className="w-5 h-5 text-green-500" />
                    <span className="font-mono text-xs text-green-500">MPC Protected</span>
                  </div>
                </div>
                <div className="mt-6 flex items-center gap-3">
                  <span className="font-mono text-sm text-zinc-400">{selectedWallet.address}</span>
                  <button
                    onClick={copyAddress}
                    className="p-1 hover:bg-zinc-800 rounded transition-colors"
                    data-testid="copy-address-btn"
                  >
                    {copied ? <Check className="w-4 h-4 text-green-500" /> : <Copy className="w-4 h-4 text-zinc-500" />}
                  </button>
                </div>
              </div>
              
              {/* Actions */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {/* Send */}
                <div className="card rounded-sm p-6">
                  <div className="flex items-center gap-2 mb-4">
                    <Send className="w-5 h-5 text-cyan-500" />
                    <h3 className="font-unbounded text-lg font-semibold">Send</h3>
                  </div>
                  <form onSubmit={handleTransfer} className="space-y-4">
                    <div>
                      <label className="font-mono text-xs text-zinc-500 block mb-2">RECIPIENT</label>
                      <input
                        type="text"
                        value={recipient}
                        onChange={(e) => setRecipient(e.target.value)}
                        placeholder="jaspr1..."
                        className="input w-full rounded-sm"
                        data-testid="recipient-input"
                      />
                    </div>
                    <div>
                      <label className="font-mono text-xs text-zinc-500 block mb-2">AMOUNT (JASPR)</label>
                      <input
                        type="number"
                        value={amount}
                        onChange={(e) => setAmount(e.target.value)}
                        placeholder="0.0"
                        step="0.0001"
                        className="input w-full rounded-sm"
                        data-testid="amount-input"
                      />
                    </div>
                    <button
                      type="submit"
                      disabled={transferring || !recipient || !amount}
                      className="btn-primary w-full flex items-center justify-center gap-2"
                      data-testid="send-btn"
                    >
                      {transferring ? (
                        <RefreshCw className="w-4 h-4 animate-spin" />
                      ) : (
                        <Send className="w-4 h-4" />
                      )}
                      Send
                    </button>
                  </form>
                  
                  {transferResult && (
                    <div className={`mt-4 p-3 rounded-sm ${
                      transferResult.success ? "bg-green-500/10 border border-green-500/20" : "bg-red-500/10 border border-red-500/20"
                    }`}>
                      {transferResult.success ? (
                        <div>
                          <p className="font-mono text-xs text-green-500">Transaction submitted!</p>
                          <p className="font-mono text-xs text-zinc-400 mt-1">
                            Hash: {transferResult.data.tx_hash.slice(0, 16)}...
                          </p>
                        </div>
                      ) : (
                        <p className="font-mono text-xs text-red-500">{transferResult.error}</p>
                      )}
                    </div>
                  )}
                </div>
                
                {/* Receive */}
                <div className="card rounded-sm p-6">
                  <div className="flex items-center gap-2 mb-4">
                    <ArrowDownLeft className="w-5 h-5 text-green-500" />
                    <h3 className="font-unbounded text-lg font-semibold">Receive</h3>
                  </div>
                  <div className="text-center py-4">
                    <div className="w-32 h-32 mx-auto bg-white rounded-sm flex items-center justify-center">
                      <span className="font-mono text-xs text-black">QR Code</span>
                    </div>
                    <p className="font-mono text-xs text-zinc-400 mt-4 break-all">
                      {selectedWallet.address}
                    </p>
                    <button
                      onClick={copyAddress}
                      className="btn-secondary mt-4 flex items-center gap-2 mx-auto"
                    >
                      <Copy className="w-4 h-4" />
                      Copy Address
                    </button>
                  </div>
                </div>
              </div>
              
              {/* AA Wallet info */}
              {walletDetails?.aa_wallet && (
                <div className="card rounded-sm p-6">
                  <h3 className="font-unbounded text-lg font-semibold mb-4">Account Abstraction</h3>
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                    <div>
                      <p className="font-mono text-xs text-zinc-500">SESSION KEYS</p>
                      <p className="font-mono text-lg text-white">{walletDetails.aa_wallet.active_session_keys}</p>
                    </div>
                    <div>
                      <p className="font-mono text-xs text-zinc-500">GUARDIANS</p>
                      <p className="font-mono text-lg text-white">{walletDetails.aa_wallet.guardians?.length || 0}</p>
                    </div>
                    <div>
                      <p className="font-mono text-xs text-zinc-500">2FA</p>
                      <p className={`font-mono text-lg ${walletDetails.aa_wallet.two_factor_enabled ? "text-green-500" : "text-zinc-500"}`}>
                        {walletDetails.aa_wallet.two_factor_enabled ? "Enabled" : "Disabled"}
                      </p>
                    </div>
                    <div>
                      <p className="font-mono text-xs text-zinc-500">BLOCKED ADDR</p>
                      <p className="font-mono text-lg text-white">{walletDetails.aa_wallet.blocked_addresses?.length || 0}</p>
                    </div>
                  </div>
                </div>
              )}
            </>
          ) : (
            <div className="card rounded-sm p-12 text-center">
              <Wallet className="w-16 h-16 mx-auto text-zinc-700 mb-4" />
              <h3 className="font-unbounded text-xl font-semibold text-zinc-400">Select a Wallet</h3>
              <p className="font-manrope text-zinc-500 mt-2">Choose a wallet from the list or create a new one</p>
            </div>
          )}
        </div>
      </div>
    </motion.div>
  );
}
