import { useState, useEffect } from "react";
import { BrowserRouter, Routes, Route, Link, useLocation } from "react-router-dom";
import { motion, AnimatePresence } from "framer-motion";
import { 
  Blocks, Users, Wallet, Shield, Settings, 
  Activity, Menu, X, Zap, TrendingUp, Clock, Database, Coins, History
} from "lucide-react";
import axios from "axios";

// Pages
import Dashboard from "./pages/Dashboard";
import BlockExplorer from "./pages/BlockExplorer";
import HistoryPage from "./pages/HistoryPage";
import TradePage from "./pages/TradePage";
import Validators from "./pages/Validators";
import WalletPage from "./pages/WalletPage";
import StakingPage from "./pages/StakingPage";
import SentinelPage from "./pages/SentinelPage";
import MempoolPage from "./pages/MempoolPage";

import "./App.css";

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;
const API = `${BACKEND_URL}/api`;

// Navigation items - CORE L1 ONLY
const navItems = [
  { path: "/", icon: Activity, label: "Dashboard" },
  { path: "/trade", icon: TrendingUp, label: "Trade" },
  { path: "/blocks", icon: Blocks, label: "Blocks" },
  { path: "/history", icon: History, label: "History" },
  { path: "/validators", icon: Users, label: "Validators" },
  { path: "/wallet", icon: Wallet, label: "Wallet" },
  { path: "/staking", icon: Coins, label: "Staking" },
  { path: "/sentinel", icon: Shield, label: "AI Sentinel" },
  { path: "/mempool", icon: Database, label: "Mempool" },
];

// Sidebar Component
function Sidebar({ isOpen, setIsOpen }) {
  const location = useLocation();
  
  return (
    <>
      {/* Mobile overlay */}
      <AnimatePresence>
        {isOpen && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/60 z-40 lg:hidden"
            onClick={() => setIsOpen(false)}
          />
        )}
      </AnimatePresence>
      
      {/* Sidebar */}
      <motion.aside
        className={`fixed top-0 left-0 h-full w-64 bg-zinc-950 border-r border-zinc-800 z-50 transform transition-transform lg:transform-none ${
          isOpen ? "translate-x-0" : "-translate-x-full lg:translate-x-0"
        }`}
      >
        {/* Logo */}
        <div className="p-6 border-b border-zinc-800">
          <Link to="/" className="flex items-center gap-3" data-testid="logo-link">
            <div className="w-10 h-10 rounded-sm bg-gradient-to-br from-cyan-500 to-purple-600 flex items-center justify-center">
              <Zap className="w-6 h-6 text-black" />
            </div>
            <div>
              <h1 className="font-unbounded font-bold text-xl text-white">JASPR</h1>
              <p className="font-mono text-[10px] text-zinc-500 uppercase tracking-widest">Testnet v0.1</p>
            </div>
          </Link>
        </div>
        
        {/* Navigation */}
        <nav className="p-4 space-y-1">
          {navItems.map((item) => {
            const isActive = location.pathname === item.path || 
              (item.path !== "/" && location.pathname.startsWith(item.path));
            
            return (
              <Link
                key={item.path}
                to={item.path}
                data-testid={`nav-${item.label.toLowerCase().replace(" ", "-")}`}
                className={`flex items-center gap-3 px-4 py-3 rounded-sm transition-all duration-75 group ${
                  isActive
                    ? "bg-cyan-500/10 text-cyan-500 border-l-2 border-cyan-500"
                    : "text-zinc-400 hover:text-white hover:bg-zinc-900"
                }`}
                onClick={() => setIsOpen(false)}
              >
                <item.icon className={`w-5 h-5 ${isActive ? "text-cyan-500" : "text-zinc-500 group-hover:text-zinc-300"}`} />
                <span className="font-manrope text-sm">{item.label}</span>
              </Link>
            );
          })}
        </nav>
        
        {/* Network Status */}
        <div className="absolute bottom-0 left-0 right-0 p-4 border-t border-zinc-800">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-yellow-500 animate-pulse" />
            <span className="font-mono text-xs text-yellow-500">Testnet Connected</span>
          </div>
        </div>
      </motion.aside>
    </>
  );
}

// Header Component
function Header({ setIsOpen, networkStats }) {
  return (
    <header className="sticky top-0 z-30 bg-zinc-950/80 backdrop-blur-xl border-b border-zinc-800">
      <div className="flex items-center justify-between px-4 lg:px-8 h-16">
        {/* Mobile menu button */}
        <button
          className="lg:hidden p-2 text-zinc-400 hover:text-white"
          onClick={() => setIsOpen(true)}
          data-testid="mobile-menu-btn"
        >
          <Menu className="w-6 h-6" />
        </button>
        
        {/* Stats bar */}
        <div className="flex items-center gap-6 overflow-x-auto">
          <StatBadge 
            icon={Blocks} 
            label="Height" 
            value={networkStats?.height?.toLocaleString() || "0"} 
          />
          <StatBadge 
            icon={TrendingUp} 
            label="TPS" 
            value={networkStats?.tps?.toFixed(2) || "0"} 
          />
          <StatBadge 
            icon={Clock} 
            label="Finality" 
            value={`${networkStats?.finality?.avg_time_ms?.toFixed(0) || "0"}ms`} 
          />
          <StatBadge 
            icon={Activity} 
            label="Txns" 
            value={networkStats?.total_transactions?.toLocaleString() || "0"} 
          />
        </div>
        
        {/* Right side */}
        <div className="flex items-center gap-4">
          <button 
            className="p-2 text-zinc-400 hover:text-white transition-colors"
            data-testid="settings-btn"
          >
            <Settings className="w-5 h-5" />
          </button>
        </div>
      </div>
    </header>
  );
}

function StatBadge({ icon: Icon, label, value }) {
  return (
    <div className="flex items-center gap-2 whitespace-nowrap">
      <Icon className="w-4 h-4 text-zinc-500" />
      <span className="font-mono text-xs text-zinc-500">{label}:</span>
      <span className="font-mono text-sm text-cyan-500">{value}</span>
    </div>
  );
}

// Main App Component
function App() {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [networkStats, setNetworkStats] = useState(null);
  
  // WebSocket for real-time network stats
  useEffect(() => {
    const wsUrl = API.replace('/api', '').replace('https://', 'wss://').replace('http://', 'ws://');
    const ws = new WebSocket(`${wsUrl}/ws`);
    
    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      if (data.type === 'stats_update' || data.type === 'new_block') {
        setNetworkStats(prev => ({
          ...prev,
          height: data.data.height || prev?.height,
          tps: data.data.tps || prev?.tps,
          total_transactions: data.data.total_transactions || prev?.total_transactions
        }));
      }
    };
    
    ws.onerror = (e) => console.error('WebSocket error:', e);
    
    return () => ws.close();
  }, []);
  
  // Fetch network stats (initial and periodic refresh)
  useEffect(() => {
    const fetchStats = async () => {
      try {
        const response = await axios.get(`${API}/network/stats`);
        setNetworkStats(response.data);
      } catch (e) {
        console.error("Failed to fetch network stats:", e);
      }
    };
    
    fetchStats();
    const interval = setInterval(fetchStats, 3000);  // Refresh every 3 seconds
    return () => clearInterval(interval);
  }, []);
  
  return (
    <div className="App min-h-screen bg-[#050505]">
      {/* Grid background */}
      <div className="fixed inset-0 bg-[linear-gradient(to_right,#80808012_1px,transparent_1px),linear-gradient(to_bottom,#80808012_1px,transparent_1px)] bg-[size:24px_24px] pointer-events-none" />
      
      <BrowserRouter>
        <div className="flex">
          <Sidebar isOpen={sidebarOpen} setIsOpen={setSidebarOpen} />
          
          <main className="flex-1 lg:ml-64 min-h-screen">
            <Header setIsOpen={setSidebarOpen} networkStats={networkStats} />
            
            <div className="p-4 lg:p-8">
              <AnimatePresence mode="wait">
                <Routes>
                  <Route path="/" element={<Dashboard networkStats={networkStats} />} />
                  <Route path="/trade" element={<TradePage />} />
                  <Route path="/blocks" element={<BlockExplorer />} />
                  <Route path="/blocks/:heightOrHash" element={<BlockExplorer />} />
                  <Route path="/history" element={<HistoryPage />} />
                  <Route path="/validators" element={<Validators />} />
                  <Route path="/wallet" element={<WalletPage />} />
                  <Route path="/staking" element={<StakingPage />} />
                  <Route path="/sentinel" element={<SentinelPage />} />
                  <Route path="/mempool" element={<MempoolPage />} />
                </Routes>
              </AnimatePresence>
            </div>
          </main>
        </div>
      </BrowserRouter>
    </div>
  );
}

export default App;
