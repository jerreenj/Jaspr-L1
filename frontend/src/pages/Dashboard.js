import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { Link } from "react-router-dom";
import {
  Blocks, Users, Zap, Shield, TrendingUp, Activity,
  ArrowUpRight, ArrowDownRight, Clock, Database, Cpu
} from "lucide-react";
import axios from "axios";

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;
const API = `${BACKEND_URL}/api`;

function StatCard({ title, value, subtitle, icon: Icon, trend, color = "cyan" }) {
  const colorClasses = {
    cyan: "from-cyan-500/20 to-transparent border-cyan-500/30",
    purple: "from-purple-500/20 to-transparent border-purple-500/30",
    green: "from-green-500/20 to-transparent border-green-500/30",
    yellow: "from-yellow-500/20 to-transparent border-yellow-500/30",
  };
  
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className={`card rounded-sm p-6 bg-gradient-to-br ${colorClasses[color]}`}
      data-testid={`stat-${title.toLowerCase().replace(/\s+/g, "-")}`}
    >
      <div className="flex items-start justify-between">
        <div>
          <p className="font-mono text-xs text-zinc-500 uppercase tracking-widest mb-2">{title}</p>
          <p className="font-unbounded text-3xl font-bold text-white">{value}</p>
          {subtitle && (
            <p className="font-mono text-xs text-zinc-400 mt-1">{subtitle}</p>
          )}
        </div>
        <div className="p-3 rounded-sm bg-zinc-900/50">
          <Icon className={`w-6 h-6 text-${color}-500`} />
        </div>
      </div>
      {trend && (
        <div className={`flex items-center gap-1 mt-4 text-sm ${trend > 0 ? "text-green-500" : "text-red-500"}`}>
          {trend > 0 ? <ArrowUpRight className="w-4 h-4" /> : <ArrowDownRight className="w-4 h-4" />}
          <span className="font-mono">{Math.abs(trend)}%</span>
          <span className="text-zinc-500 text-xs">vs last hour</span>
        </div>
      )}
    </motion.div>
  );
}

function RecentBlocks({ blocks }) {
  return (
    <div className="card rounded-sm" data-testid="recent-blocks">
      <div className="p-4 border-b border-zinc-800 flex items-center justify-between">
        <h3 className="font-unbounded text-lg font-semibold">Recent Blocks</h3>
        <Link to="/blocks" className="font-mono text-xs text-cyan-500 hover:text-cyan-400">
          View All →
        </Link>
      </div>
      <div className="divide-y divide-zinc-800">
        {blocks.slice(0, 5).map((block, i) => (
          <Link
            key={block.hash}
            to={`/blocks/${block.height}`}
            className="flex items-center justify-between p-4 hover:bg-zinc-900/50 transition-colors"
          >
            <div className="flex items-center gap-4">
              <div className="w-10 h-10 rounded-sm bg-cyan-500/10 flex items-center justify-center">
                <Blocks className="w-5 h-5 text-cyan-500" />
              </div>
              <div>
                <p className="font-mono text-sm text-white">#{block.height}</p>
                <p className="font-mono text-xs text-zinc-500">{block.tx_count} txns</p>
              </div>
            </div>
            <div className="text-right">
              <p className="font-mono text-xs text-zinc-400">
                {new Date(block.timestamp).toLocaleTimeString()}
              </p>
              {block.finalized && (
                <span className="badge-success">Finalized</span>
              )}
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}

function ValidatorList({ validators }) {
  return (
    <div className="card rounded-sm" data-testid="validator-list">
      <div className="p-4 border-b border-zinc-800 flex items-center justify-between">
        <h3 className="font-unbounded text-lg font-semibold">Active Validators</h3>
        <Link to="/validators" className="font-mono text-xs text-cyan-500 hover:text-cyan-400">
          View All →
        </Link>
      </div>
      <div className="divide-y divide-zinc-800">
        {validators.slice(0, 4).map((validator, i) => (
          <div key={validator.address} className="flex items-center justify-between p-4">
            <div className="flex items-center gap-4">
              <div className="w-10 h-10 rounded-sm bg-purple-500/10 flex items-center justify-center">
                <span className="font-unbounded font-bold text-purple-500">#{i + 1}</span>
              </div>
              <div>
                <p className="font-manrope text-sm text-white">{validator.name || "Validator"}</p>
                <p className="font-mono text-xs text-zinc-500">
                  {validator.address.slice(0, 12)}...{validator.address.slice(-4)}
                </p>
              </div>
            </div>
            <div className="text-right">
              <p className="font-mono text-sm text-white">
                {(validator.stake / 1_000_000_000_000).toFixed(0)}K JASPR
              </p>
              <p className="font-mono text-xs text-zinc-500">
                {((validator.voting_power / validators.reduce((a, v) => a + v.voting_power, 0)) * 100).toFixed(1)}%
              </p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function SentinelStatus({ sentinel }) {
  const getRiskColor = (score) => {
    if (score < 20) return "text-green-500";
    if (score < 50) return "text-yellow-500";
    if (score < 80) return "text-orange-500";
    return "text-red-500";
  };
  
  return (
    <div className="card rounded-sm" data-testid="sentinel-status">
      <div className="p-4 border-b border-zinc-800 flex items-center justify-between">
        <h3 className="font-unbounded text-lg font-semibold">AI Sentinel</h3>
        <span className={`badge-${sentinel?.guard_mode === 'enforced' ? 'success' : 'warning'}`}>
          {sentinel?.guard_mode || 'Unknown'}
        </span>
      </div>
      <div className="p-4 space-y-4">
        <div className="flex items-center justify-between">
          <span className="font-mono text-xs text-zinc-500">TOTAL SCANNED</span>
          <span className="font-mono text-lg text-white">{sentinel?.stats?.total_scanned || 0}</span>
        </div>
        <div className="flex items-center justify-between">
          <span className="font-mono text-xs text-zinc-500">THREATS DETECTED</span>
          <span className="font-mono text-lg text-red-500">{sentinel?.stats?.threats_detected || 0}</span>
        </div>
        <div className="flex items-center justify-between">
          <span className="font-mono text-xs text-zinc-500">BLOCKED</span>
          <span className="font-mono text-lg text-yellow-500">{sentinel?.stats?.transactions_blocked || 0}</span>
        </div>
        <div className="flex items-center justify-between">
          <span className="font-mono text-xs text-zinc-500">AVG RISK SCORE</span>
          <span className={`font-mono text-lg ${getRiskColor(sentinel?.stats?.avg_score || 0)}`}>
            {(sentinel?.stats?.avg_score || 0).toFixed(1)}
          </span>
        </div>
        <div className="progress-bar mt-4">
          <div 
            className="progress-fill" 
            style={{ width: `${Math.min(100, (sentinel?.stats?.detection_rate || 0))}%` }}
          />
        </div>
        <p className="font-mono text-xs text-zinc-500 text-center">
          Detection Rate: {(sentinel?.stats?.detection_rate || 0).toFixed(2)}%
        </p>
      </div>
    </div>
  );
}

export default function Dashboard({ networkStats }) {
  const [blocks, setBlocks] = useState([]);
  const [validators, setValidators] = useState([]);
  const [sentinel, setSentinel] = useState(null);
  
  useEffect(() => {
    const fetchData = async () => {
      try {
        const [blocksRes, validatorsRes, sentinelRes] = await Promise.all([
          axios.get(`${API}/blocks?limit=10`),
          axios.get(`${API}/validators`),
          axios.get(`${API}/sentinel`)
        ]);
        setBlocks(blocksRes.data.blocks || []);
        setValidators(validatorsRes.data.validators || []);
        setSentinel(sentinelRes.data);
      } catch (e) {
        console.error("Dashboard fetch error:", e);
      }
    };
    
    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, []);
  
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="space-y-8"
    >
      {/* Hero section */}
      <div className="relative overflow-hidden rounded-sm bg-gradient-to-br from-zinc-900 to-zinc-950 border border-zinc-800 p-8">
        <div className="absolute inset-0 bg-[url('https://images.unsplash.com/photo-1710957987034-cea509422852?w=1200')] bg-cover bg-center opacity-10" />
        <div className="relative z-10">
          <h1 className="font-unbounded text-4xl font-bold mb-2">
            <span className="gradient-text">JasprChain</span> Dashboard
          </h1>
          <p className="font-manrope text-zinc-400 max-w-xl">
            High-performance L1 blockchain with AI-protected safety, hybrid DEX settlement, and under 2s finality.
          </p>
        </div>
      </div>
      
      {/* Stats grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          title="Block Height"
          value={networkStats?.height?.toLocaleString() || "0"}
          subtitle="Latest confirmed block"
          icon={Blocks}
          color="cyan"
        />
        <StatCard
          title="Transactions"
          value={networkStats?.total_transactions?.toLocaleString() || "0"}
          subtitle={`${(networkStats?.tps || 0).toFixed(2)} TPS`}
          icon={Activity}
          color="purple"
          trend={12}
        />
        <StatCard
          title="Validators"
          value={networkStats?.validators?.active || "0"}
          subtitle={`${((networkStats?.validators?.total_stake || 0) / 1_000_000_000_000).toFixed(0)}K JASPR staked`}
          icon={Users}
          color="green"
        />
        <StatCard
          title="Avg Finality"
          value={`${(networkStats?.finality?.avg_time_ms || 0).toFixed(0)}ms`}
          subtitle="Target: <2000ms"
          icon={Clock}
          color="yellow"
        />
      </div>
      
      {/* Main content grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2">
          <RecentBlocks blocks={blocks} />
        </div>
        <div className="space-y-6">
          <ValidatorList validators={validators} />
          <SentinelStatus sentinel={sentinel} />
        </div>
      </div>
    </motion.div>
  );
}
