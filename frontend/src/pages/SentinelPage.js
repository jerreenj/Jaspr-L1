import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { Shield, AlertTriangle, CheckCircle, XCircle, Eye, Settings } from "lucide-react";
import axios from "axios";

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;
const API = `${BACKEND_URL}/api`;

function getRiskColor(score) {
  if (score < 20) return { text: "text-green-500", bg: "bg-green-500", label: "Low" };
  if (score < 50) return { text: "text-yellow-500", bg: "bg-yellow-500", label: "Medium" };
  if (score < 80) return { text: "text-orange-500", bg: "bg-orange-500", label: "High" };
  return { text: "text-red-500", bg: "bg-red-500", label: "Critical" };
}

function StatCard({ title, value, subtitle, color = "cyan" }) {
  const colorClasses = {
    cyan: "border-cyan-500/30",
    green: "border-green-500/30",
    yellow: "border-yellow-500/30",
    red: "border-red-500/30",
  };
  
  return (
    <div className={`card rounded-sm p-4 border-l-4 ${colorClasses[color]}`}>
      <p className="font-mono text-xs text-zinc-500 uppercase">{title}</p>
      <p className="font-unbounded text-2xl font-bold mt-1">{value}</p>
      {subtitle && <p className="font-mono text-xs text-zinc-500 mt-1">{subtitle}</p>}
    </div>
  );
}

function GuardModeSelector({ currentMode, onModeChange }) {
  const modes = [
    { value: "passive", label: "Passive", description: "Monitor only, no blocking", icon: Eye, color: "text-blue-500" },
    { value: "warning", label: "Warning", description: "Warn on risky transactions", icon: AlertTriangle, color: "text-yellow-500" },
    { value: "enforced", label: "Enforced", description: "Block high-risk transactions", icon: Shield, color: "text-green-500" },
  ];
  
  return (
    <div className="card rounded-sm p-6" data-testid="guard-mode-selector">
      <div className="flex items-center gap-2 mb-4">
        <Settings className="w-5 h-5 text-cyan-500" />
        <h3 className="font-unbounded text-lg font-semibold">Guard Mode</h3>
      </div>
      <div className="grid grid-cols-3 gap-4">
        {modes.map((mode) => (
          <button
            key={mode.value}
            onClick={() => onModeChange(mode.value)}
            className={`p-4 rounded-sm border transition-all ${
              currentMode === mode.value
                ? "border-cyan-500 bg-cyan-500/10"
                : "border-zinc-800 hover:border-zinc-700"
            }`}
            data-testid={`mode-${mode.value}`}
          >
            <mode.icon className={`w-8 h-8 mx-auto mb-2 ${mode.color}`} />
            <p className="font-mono text-sm font-semibold text-white">{mode.label}</p>
            <p className="font-mono text-xs text-zinc-500 mt-1">{mode.description}</p>
          </button>
        ))}
      </div>
    </div>
  );
}

function QuarantineList({ items }) {
  return (
    <div className="card rounded-sm" data-testid="quarantine-list">
      <div className="p-4 border-b border-zinc-800 flex items-center justify-between">
        <h3 className="font-unbounded text-lg font-semibold">Quarantined Transactions</h3>
        <span className="badge-warning">{items.length}</span>
      </div>
      {items.length === 0 ? (
        <div className="p-8 text-center">
          <CheckCircle className="w-12 h-12 mx-auto text-green-500 mb-4" />
          <p className="font-manrope text-zinc-400">No suspicious transactions</p>
          <p className="font-mono text-xs text-zinc-500 mt-2">All transactions are clean</p>
        </div>
      ) : (
        <div className="divide-y divide-zinc-800 max-h-96 overflow-y-auto">
          {items.map((item, i) => {
            const risk = item.risk_score ? getRiskColor(item.risk_score.score) : getRiskColor(0);
            
            return (
              <div key={item.tx_hash || i} className="p-4">
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <p className="font-mono text-sm text-white">
                      {item.tx_hash?.slice(0, 20)}...
                    </p>
                    <p className="font-mono text-xs text-zinc-500">
                      {item.risk_score?.threat_type || 'unknown'}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className={`font-mono text-lg font-bold ${risk.text}`}>
                      {item.risk_score?.score?.toFixed(0) || 0}
                    </span>
                    <span className={`badge-${risk.label === 'Low' ? 'success' : risk.label === 'Critical' ? 'danger' : 'warning'}`}>
                      {risk.label}
                    </span>
                  </div>
                </div>
                
                {item.risk_score?.risk_factors && item.risk_score.risk_factors.length > 0 && (
                  <div className="mt-2 space-y-1">
                    {item.risk_score.risk_factors.map((factor, j) => (
                      <div key={j} className="flex items-center gap-2">
                        <AlertTriangle className="w-3 h-3 text-yellow-500" />
                        <span className="font-mono text-xs text-zinc-400">{factor}</span>
                      </div>
                    ))}
                  </div>
                )}
                
                <div className="flex gap-2 mt-3">
                  <button className="btn-secondary text-xs">
                    Mark Safe
                  </button>
                  <button className="bg-red-500/10 text-red-500 border border-red-500/20 px-3 py-1 rounded-sm font-mono text-xs">
                    Reject
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function RiskDistribution({ stats }) {
  const distribution = [
    { label: "Low", range: "0-20", color: "bg-green-500", width: 60 },
    { label: "Medium", range: "21-50", color: "bg-yellow-500", width: 25 },
    { label: "High", range: "51-80", color: "bg-orange-500", width: 10 },
    { label: "Critical", range: "81-100", color: "bg-red-500", width: 5 },
  ];
  
  return (
    <div className="card rounded-sm p-6">
      <h3 className="font-unbounded text-lg font-semibold mb-4">Risk Distribution</h3>
      <div className="space-y-4">
        {distribution.map((item) => (
          <div key={item.label}>
            <div className="flex items-center justify-between mb-1">
              <span className="font-mono text-xs text-zinc-400">{item.label}</span>
              <span className="font-mono text-xs text-zinc-500">{item.range}</span>
            </div>
            <div className="h-3 bg-zinc-800 rounded-sm overflow-hidden">
              <div
                className={`h-full ${item.color} transition-all duration-500`}
                style={{ width: `${item.width}%` }}
              />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export default function SentinelPage() {
  const [sentinel, setSentinel] = useState(null);
  const [quarantine, setQuarantine] = useState([]);
  const [loading, setLoading] = useState(true);
  
  const fetchData = async () => {
    try {
      const [sentinelRes, quarantineRes] = await Promise.all([
        axios.get(`${API}/sentinel`),
        axios.get(`${API}/sentinel/quarantine`)
      ]);
      setSentinel(sentinelRes.data);
      setQuarantine(quarantineRes.data.quarantined || []);
    } catch (e) {
      console.error("Sentinel fetch error:", e);
    } finally {
      setLoading(false);
    }
  };
  
  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, []);
  
  const handleModeChange = async (mode) => {
    try {
      await axios.post(`${API}/sentinel/mode`, { mode });
      fetchData();
    } catch (e) {
      console.error("Failed to change mode:", e);
    }
  };
  
  if (loading) {
    return <div className="text-center py-12 text-zinc-500">Loading AI Sentinel...</div>;
  }
  
  const stats = sentinel?.stats || {};
  const avgRisk = getRiskColor(stats.avg_score || 0);
  
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="space-y-6"
    >
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h1 className="font-unbounded text-2xl font-bold">AI Sentinel</h1>
          <span className={`badge-${sentinel?.guard_mode === 'enforced' ? 'success' : 'warning'}`}>
            {sentinel?.guard_mode || 'Unknown'}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
          <span className="font-mono text-xs text-green-500">ML Model Active</span>
        </div>
      </div>
      
      {/* Hero stats */}
      <div className="card rounded-sm p-8 bg-gradient-to-br from-purple-500/10 to-cyan-500/10 border-purple-500/20">
        <div className="flex items-center gap-6">
          <div className="w-20 h-20 rounded-sm bg-gradient-to-br from-purple-500 to-cyan-500 flex items-center justify-center">
            <Shield className="w-10 h-10 text-black" />
          </div>
          <div>
            <p className="font-mono text-xs text-zinc-500 uppercase tracking-widest">Protection Status</p>
            <p className="font-unbounded text-4xl font-bold mt-2">
              {stats.total_scanned?.toLocaleString() || 0} <span className="text-zinc-500 text-2xl">Scanned</span>
            </p>
            <p className="font-manrope text-zinc-400 mt-2">
              {stats.threats_detected || 0} threats detected · {stats.transactions_blocked || 0} blocked
            </p>
          </div>
        </div>
      </div>
      
      {/* Stats grid */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <StatCard
          title="Total Scanned"
          value={stats.total_scanned?.toLocaleString() || "0"}
          color="cyan"
        />
        <StatCard
          title="Threats Detected"
          value={stats.threats_detected || "0"}
          subtitle={`${stats.detection_rate?.toFixed(2) || 0}% rate`}
          color="yellow"
        />
        <StatCard
          title="Blocked"
          value={stats.transactions_blocked || "0"}
          color="red"
        />
        <StatCard
          title="Avg Risk Score"
          value={stats.avg_score?.toFixed(1) || "0"}
          subtitle={avgRisk.label}
          color={avgRisk.label === 'Low' ? 'green' : avgRisk.label === 'Medium' ? 'yellow' : 'red'}
        />
      </div>
      
      {/* Guard mode selector */}
      <GuardModeSelector 
        currentMode={sentinel?.guard_mode || 'enforced'} 
        onModeChange={handleModeChange} 
      />
      
      {/* Main content */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2">
          <QuarantineList items={quarantine} />
        </div>
        <div>
          <RiskDistribution stats={stats} />
        </div>
      </div>
    </motion.div>
  );
}
