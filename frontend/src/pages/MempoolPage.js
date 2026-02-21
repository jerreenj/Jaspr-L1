import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { Database, Clock, Zap, Filter, AlertTriangle, CheckCircle } from "lucide-react";
import axios from "axios";

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;
const API = `${BACKEND_URL}/api`;

function LaneBadge({ lane }) {
  const styles = {
    priority: "bg-cyan-500/10 text-cyan-500 border-cyan-500/20",
    normal: "bg-zinc-500/10 text-zinc-400 border-zinc-500/20",
    liquidation: "bg-red-500/10 text-red-500 border-red-500/20",
    institutional: "bg-purple-500/10 text-purple-500 border-purple-500/20",
    sponsored: "bg-green-500/10 text-green-500 border-green-500/20",
    quarantine: "bg-yellow-500/10 text-yellow-500 border-yellow-500/20",
  };
  
  return (
    <span className={`px-2 py-0.5 rounded font-mono text-[10px] uppercase border ${styles[lane] || styles.normal}`}>
      {lane}
    </span>
  );
}

function MempoolStats({ stats }) {
  const lanes = stats?.by_lane || {};
  
  return (
    <div className="grid grid-cols-1 md:grid-cols-4 gap-4" data-testid="mempool-stats">
      <div className="card rounded-sm p-4">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-sm bg-cyan-500/10">
            <Database className="w-5 h-5 text-cyan-500" />
          </div>
          <div>
            <p className="font-mono text-xs text-zinc-500">PENDING</p>
            <p className="font-unbounded text-2xl font-bold">{stats?.total_pending || 0}</p>
          </div>
        </div>
      </div>
      <div className="card rounded-sm p-4">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-sm bg-purple-500/10">
            <Zap className="w-5 h-5 text-purple-500" />
          </div>
          <div>
            <p className="font-mono text-xs text-zinc-500">TOTAL GAS</p>
            <p className="font-unbounded text-2xl font-bold">{(stats?.total_gas / 1_000_000)?.toFixed(2) || 0}M</p>
          </div>
        </div>
      </div>
      <div className="card rounded-sm p-4">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-sm bg-green-500/10">
            <TrendingUp className="w-5 h-5 text-green-500" />
          </div>
          <div>
            <p className="font-mono text-xs text-zinc-500">AVG GAS PRICE</p>
            <p className="font-unbounded text-2xl font-bold">{(stats?.avg_gas_price / 1_000_000_000)?.toFixed(2) || 0} Gwei</p>
          </div>
        </div>
      </div>
      <div className="card rounded-sm p-4">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-sm bg-yellow-500/10">
            <Clock className="w-5 h-5 text-yellow-500" />
          </div>
          <div>
            <p className="font-mono text-xs text-zinc-500">OLDEST TX</p>
            <p className="font-unbounded text-2xl font-bold">{(stats?.oldest_tx_age_ms / 1000)?.toFixed(1) || 0}s</p>
          </div>
        </div>
      </div>
    </div>
  );
}

import { TrendingUp } from "lucide-react";

function LaneDistribution({ lanes }) {
  const totalPending = Object.values(lanes).reduce((a, b) => a + b, 0) || 1;
  
  const laneConfig = [
    { key: "liquidation", label: "Liquidation", color: "bg-red-500" },
    { key: "priority", label: "Priority", color: "bg-cyan-500" },
    { key: "institutional", label: "Institutional", color: "bg-purple-500" },
    { key: "normal", label: "Normal", color: "bg-zinc-500" },
    { key: "sponsored", label: "Sponsored", color: "bg-green-500" },
    { key: "quarantine", label: "Quarantine", color: "bg-yellow-500" },
  ];
  
  return (
    <div className="card rounded-sm p-6">
      <h3 className="font-unbounded text-lg font-semibold mb-4">Lane Distribution</h3>
      
      {/* Visual bar */}
      <div className="h-4 rounded-sm overflow-hidden flex mb-6">
        {laneConfig.map((lane) => {
          const count = lanes[lane.key] || 0;
          const width = (count / totalPending) * 100;
          return width > 0 ? (
            <div
              key={lane.key}
              className={`${lane.color} transition-all duration-300`}
              style={{ width: `${width}%` }}
              title={`${lane.label}: ${count}`}
            />
          ) : null;
        })}
      </div>
      
      {/* Legend */}
      <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
        {laneConfig.map((lane) => {
          const count = lanes[lane.key] || 0;
          return (
            <div key={lane.key} className="flex items-center gap-2">
              <div className={`w-3 h-3 rounded-sm ${lane.color}`} />
              <span className="font-mono text-xs text-zinc-400">{lane.label}</span>
              <span className="font-mono text-xs text-white ml-auto">{count}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function QuarantinedTransactions({ quarantined }) {
  return (
    <div className="card rounded-sm" data-testid="quarantined-txs">
      <div className="p-4 border-b border-zinc-800 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <AlertTriangle className="w-5 h-5 text-yellow-500" />
          <h3 className="font-unbounded text-lg font-semibold">Quarantined</h3>
        </div>
        <span className="badge-warning">{quarantined.length}</span>
      </div>
      {quarantined.length === 0 ? (
        <div className="p-8 text-center">
          <CheckCircle className="w-12 h-12 mx-auto text-green-500 mb-4" />
          <p className="font-manrope text-zinc-400">No quarantined transactions</p>
        </div>
      ) : (
        <div className="divide-y divide-zinc-800 max-h-96 overflow-y-auto">
          {quarantined.map((item, i) => (
            <div key={item.tx_hash || i} className="p-4">
              <div className="flex items-center justify-between mb-2">
                <span className="font-mono text-sm text-white">
                  {item.tx_hash?.slice(0, 20)}...
                </span>
                <span className="font-mono text-xs text-yellow-500">
                  Score: {item.risk_score?.score?.toFixed(0) || "?"}
                </span>
              </div>
              <p className="font-mono text-xs text-zinc-500">
                From: {item.sender?.slice(0, 20)}...
              </p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default function MempoolPage() {
  const [mempool, setMempool] = useState(null);
  const [loading, setLoading] = useState(true);
  
  useEffect(() => {
    const fetchMempool = async () => {
      try {
        const response = await axios.get(`${API}/mempool`);
        setMempool(response.data);
      } catch (e) {
        console.error("Mempool fetch error:", e);
      } finally {
        setLoading(false);
      }
    };
    
    fetchMempool();
    const interval = setInterval(fetchMempool, 3000);
    return () => clearInterval(interval);
  }, []);
  
  if (loading) {
    return <div className="text-center py-12 text-zinc-500">Loading mempool...</div>;
  }
  
  const stats = mempool?.stats || {};
  const quarantined = mempool?.quarantined || [];
  
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="space-y-6"
    >
      <div className="flex items-center justify-between">
        <h1 className="font-unbounded text-2xl font-bold">Mempool</h1>
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
          <span className="font-mono text-xs text-green-500">Live</span>
        </div>
      </div>
      
      {/* Stats */}
      <MempoolStats stats={stats} />
      
      {/* Main content */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2">
          <LaneDistribution lanes={stats.by_lane || {}} />
        </div>
        <div>
          <QuarantinedTransactions quarantined={quarantined} />
        </div>
      </div>
      
      {/* Info about lanes */}
      <div className="card rounded-sm p-6">
        <h3 className="font-unbounded text-lg font-semibold mb-4">Lane Priority</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="p-4 bg-zinc-900/50 rounded-sm">
            <div className="flex items-center gap-2 mb-2">
              <div className="w-3 h-3 rounded-sm bg-red-500" />
              <span className="font-mono text-sm text-white">Liquidation</span>
            </div>
            <p className="font-mono text-xs text-zinc-500">
              Highest priority. DEX liquidations are processed first to ensure market stability.
            </p>
          </div>
          <div className="p-4 bg-zinc-900/50 rounded-sm">
            <div className="flex items-center gap-2 mb-2">
              <div className="w-3 h-3 rounded-sm bg-cyan-500" />
              <span className="font-mono text-sm text-white">Priority</span>
            </div>
            <p className="font-mono text-xs text-zinc-500">
              High gas price transactions. Willing to pay more for faster execution.
            </p>
          </div>
          <div className="p-4 bg-zinc-900/50 rounded-sm">
            <div className="flex items-center gap-2 mb-2">
              <div className="w-3 h-3 rounded-sm bg-yellow-500" />
              <span className="font-mono text-sm text-white">Quarantine</span>
            </div>
            <p className="font-mono text-xs text-zinc-500">
              Flagged by AI Sentinel. Requires manual review or additional verification.
            </p>
          </div>
        </div>
      </div>
    </motion.div>
  );
}
