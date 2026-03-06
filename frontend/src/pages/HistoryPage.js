import { useState, useEffect } from "react";
import { motion } from "framer-motion";

const API_URL = process.env.REACT_APP_BACKEND_URL;

export default function HistoryPage() {
  const [transactions, setTransactions] = useState([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState("all"); // all, transfer, stake, unstake

  useEffect(() => {
    fetchTransactions();
    const interval = setInterval(fetchTransactions, 5000);
    return () => clearInterval(interval);
  }, []);

  const fetchTransactions = async () => {
    try {
      const res = await fetch(`${API_URL}/api/transactions/recent?limit=50`);
      if (res.ok) {
        const data = await res.json();
        setTransactions(data.transactions || []);
      }
    } catch (err) {
      console.error("Failed to fetch transactions:", err);
    } finally {
      setLoading(false);
    }
  };

  const filteredTxs = transactions.filter(tx => {
    if (filter === "all") return true;
    return tx.type === filter;
  });

  const formatTime = (timestamp) => {
    const date = new Date(timestamp);
    return date.toLocaleString();
  };

  const formatAddress = (addr) => {
    if (!addr) return "-";
    return `${addr.slice(0, 10)}...${addr.slice(-6)}`;
  };

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className="space-y-6"
    >
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-unbounded text-2xl font-bold text-white">
            Transaction History
          </h1>
          <p className="text-zinc-500 text-sm mt-1">
            Real transactions on JasprChain
          </p>
        </div>
        
        {/* Filter */}
        <div className="flex gap-2">
          {["all", "transfer", "stake", "unstake"].map((f) => (
            <button
              key={f}
              onClick={() => setFilter(f)}
              className={`px-3 py-1 text-xs font-mono rounded ${
                filter === f
                  ? "bg-[#00FFA3] text-black"
                  : "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
              }`}
            >
              {f.toUpperCase()}
            </button>
          ))}
        </div>
      </div>

      {/* Transaction List */}
      <div className="bg-zinc-900/50 border border-zinc-800 rounded-lg overflow-hidden">
        {loading ? (
          <div className="p-8 text-center text-zinc-500">
            Loading transactions...
          </div>
        ) : filteredTxs.length === 0 ? (
          <div className="p-8 text-center">
            <p className="text-zinc-500 mb-2">No transactions yet</p>
            <p className="text-zinc-600 text-sm">
              Submit a transfer or stake to see transactions here
            </p>
          </div>
        ) : (
          <table className="w-full">
            <thead className="bg-zinc-800/50">
              <tr className="text-left text-xs text-zinc-500 uppercase">
                <th className="p-4">TX Hash</th>
                <th className="p-4">Type</th>
                <th className="p-4">From</th>
                <th className="p-4">To</th>
                <th className="p-4">Amount</th>
                <th className="p-4">Block</th>
                <th className="p-4">Status</th>
                <th className="p-4">Time</th>
              </tr>
            </thead>
            <tbody>
              {filteredTxs.map((tx, i) => (
                <tr 
                  key={tx.hash || i} 
                  className="border-t border-zinc-800 hover:bg-zinc-800/30"
                >
                  <td className="p-4 font-mono text-xs text-[#00FFA3]">
                    {tx.hash ? `${tx.hash.slice(0, 12)}...` : "-"}
                  </td>
                  <td className="p-4">
                    <span className={`px-2 py-1 text-xs rounded ${
                      tx.type === "transfer" ? "bg-blue-500/20 text-blue-400" :
                      tx.type === "stake" ? "bg-green-500/20 text-green-400" :
                      tx.type === "unstake" ? "bg-yellow-500/20 text-yellow-400" :
                      "bg-zinc-700 text-zinc-400"
                    }`}>
                      {tx.type || "unknown"}
                    </span>
                  </td>
                  <td className="p-4 font-mono text-xs text-zinc-400">
                    {formatAddress(tx.sender)}
                  </td>
                  <td className="p-4 font-mono text-xs text-zinc-400">
                    {formatAddress(tx.recipient)}
                  </td>
                  <td className="p-4 font-mono text-sm text-white">
                    {tx.amount?.toLocaleString() || 0} JASPR
                  </td>
                  <td className="p-4 font-mono text-xs text-zinc-500">
                    {tx.block_height || "pending"}
                  </td>
                  <td className="p-4">
                    <span className={`px-2 py-1 text-xs rounded ${
                      tx.status === "confirmed" 
                        ? "bg-green-500/20 text-green-400" 
                        : "bg-yellow-500/20 text-yellow-400"
                    }`}>
                      {tx.status}
                    </span>
                  </td>
                  <td className="p-4 text-xs text-zinc-500">
                    {formatTime(tx.timestamp)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Info */}
      <div className="text-center text-zinc-600 text-sm">
        Showing {filteredTxs.length} real transactions • Auto-refreshes every 5s
      </div>
    </motion.div>
  );
}
