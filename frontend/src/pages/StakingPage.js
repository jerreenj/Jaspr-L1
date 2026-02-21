import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { Coins, TrendingUp, Users, ArrowUpRight, ArrowDownRight, RefreshCw, Check } from "lucide-react";
import axios from "axios";

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;
const API = `${BACKEND_URL}/api`;

function ValidatorCard({ validator, onSelect, isSelected, totalStake }) {
  const stakePercentage = ((validator.stake / totalStake) * 100).toFixed(1);
  
  return (
    <button
      onClick={() => onSelect(validator)}
      className={`w-full text-left card rounded-sm p-4 transition-all ${
        isSelected ? "border-cyan-500 bg-cyan-500/10" : "hover:border-zinc-700"
      }`}
      data-testid={`validator-card-${validator.address.slice(0, 8)}`}
    >
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-sm bg-gradient-to-br from-purple-500 to-cyan-500 flex items-center justify-center">
            <span className="font-unbounded font-bold text-black text-sm">
              {validator.name?.charAt(0) || "V"}
            </span>
          </div>
          <div>
            <p className="font-manrope font-medium text-white">{validator.name || "Validator"}</p>
            <p className="font-mono text-xs text-zinc-500">
              {validator.address.slice(0, 12)}...
            </p>
          </div>
        </div>
        {isSelected && <Check className="w-5 h-5 text-cyan-500" />}
      </div>
      
      <div className="grid grid-cols-4 gap-2 text-xs">
        <div>
          <p className="text-zinc-500">Stake</p>
          <p className="font-mono text-white">{(validator.stake / 1_000_000_000_000).toFixed(1)}K</p>
        </div>
        <div>
          <p className="text-zinc-500">Share</p>
          <p className="font-mono text-cyan-500">{stakePercentage}%</p>
        </div>
        <div>
          <p className="text-zinc-500">APY</p>
          <p className="font-mono text-green-500">{validator.apy || "~9"}%</p>
        </div>
        <div>
          <p className="text-zinc-500">Fee</p>
          <p className="font-mono text-white">{(validator.commission_rate / 100).toFixed(1)}%</p>
        </div>
      </div>
      
      <div className="mt-3 h-1.5 bg-zinc-800 rounded-full overflow-hidden">
        <div 
          className="h-full bg-gradient-to-r from-cyan-500 to-purple-500"
          style={{ width: `${stakePercentage}%` }}
        />
      </div>
    </button>
  );
}

function StakeForm({ selectedValidator, walletAddress, onStake, onUnstake, currentStake }) {
  const [action, setAction] = useState("stake");
  const [amount, setAmount] = useState("");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState(null);
  
  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!selectedValidator || !amount) return;
    
    setLoading(true);
    setResult(null);
    
    try {
      const amountWei = Math.floor(parseFloat(amount) * 1_000_000_000);
      if (action === "stake") {
        await onStake(selectedValidator.address, amountWei);
        setResult({ success: true, message: `Staked ${amount} $JSP to ${selectedValidator.name}` });
      } else {
        await onUnstake(selectedValidator.address, amountWei);
        setResult({ success: true, message: `Unstaked ${amount} $JSP from ${selectedValidator.name}` });
      }
      setAmount("");
    } catch (e) {
      setResult({ success: false, message: e.response?.data?.detail || "Operation failed" });
    } finally {
      setLoading(false);
    }
  };
  
  if (!selectedValidator) {
    return (
      <div className="card rounded-sm p-8 text-center">
        <Coins className="w-12 h-12 mx-auto text-zinc-700 mb-4" />
        <p className="font-manrope text-zinc-400">Select a validator to stake</p>
      </div>
    );
  }
  
  return (
    <div className="card rounded-sm p-6" data-testid="stake-form">
      <h3 className="font-unbounded text-lg font-semibold mb-4">
        {action === "stake" ? "Stake" : "Unstake"} to {selectedValidator.name}
      </h3>
      
      {/* APY Display */}
      {selectedValidator.apy && (
        <div className="p-3 bg-green-500/10 border border-green-500/20 rounded-sm mb-4">
          <p className="font-mono text-xs text-green-400">ESTIMATED APY</p>
          <p className="font-unbounded text-2xl text-green-500">{selectedValidator.apy}%</p>
        </div>
      )}
      
      {/* Action selector */}
      <div className="grid grid-cols-2 gap-2 mb-4">
        <button
          onClick={() => setAction("stake")}
          className={`py-3 rounded-sm font-mono text-sm transition-colors ${
            action === "stake"
              ? "bg-green-500 text-black"
              : "bg-zinc-900 text-zinc-400 hover:bg-zinc-800"
          }`}
          data-testid="stake-action-btn"
        >
          <ArrowUpRight className="w-4 h-4 inline mr-2" />
          STAKE
        </button>
        <button
          onClick={() => setAction("unstake")}
          className={`py-3 rounded-sm font-mono text-sm transition-colors ${
            action === "unstake"
              ? "bg-red-500 text-black"
              : "bg-zinc-900 text-zinc-400 hover:bg-zinc-800"
          }`}
          data-testid="unstake-action-btn"
        >
          <ArrowDownRight className="w-4 h-4 inline mr-2" />
          UNSTAKE
        </button>
      </div>
      
      {/* Current stake info */}
      {currentStake > 0 && (
        <div className="p-3 bg-zinc-900 rounded-sm mb-4">
          <p className="font-mono text-xs text-zinc-500">YOUR CURRENT STAKE</p>
          <p className="font-mono text-lg text-white">{(currentStake / 1_000_000_000).toFixed(4)} $JSP</p>
        </div>
      )}
      
      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label className="font-mono text-xs text-zinc-500 block mb-2">AMOUNT ($JSP)</label>
          <input
            type="number"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="0.0"
            step="0.0001"
            className="input w-full rounded-sm"
            data-testid="stake-amount-input"
          />
        </div>
        
        <button
          type="submit"
          disabled={loading || !amount || !walletAddress}
          className={`w-full py-3 rounded-sm font-mono text-sm uppercase font-bold transition-all ${
            action === "stake"
              ? "bg-green-500 hover:bg-green-400 text-black"
              : "bg-red-500 hover:bg-red-400 text-black"
          } disabled:opacity-50`}
          data-testid="submit-stake-btn"
        >
          {loading ? (
            <RefreshCw className="w-4 h-4 animate-spin mx-auto" />
          ) : (
            `${action} $JSP`
          )}
        </button>
        
        {!walletAddress && (
          <p className="font-mono text-xs text-yellow-500 text-center">
            Create a wallet first to stake
          </p>
        )}
      </form>
      
      {result && (
        <div className={`mt-4 p-3 rounded-sm ${
          result.success ? "bg-green-500/10 border border-green-500/20" : "bg-red-500/10 border border-red-500/20"
        }`}>
          <p className={`font-mono text-xs ${result.success ? "text-green-500" : "text-red-500"}`}>
            {result.message}
          </p>
        </div>
      )}
    </div>
  );
}

export default function StakingPage() {
  const [validators, setValidators] = useState([]);
  const [selectedValidator, setSelectedValidator] = useState(null);
  const [totalStake, setTotalStake] = useState(0);
  const [walletAddress, setWalletAddress] = useState(null);
  const [stakingInfo, setStakingInfo] = useState(null);
  const [loading, setLoading] = useState(true);
  
  // Load wallet from localStorage
  useEffect(() => {
    const saved = localStorage.getItem("jasprWallets");
    if (saved) {
      const wallets = JSON.parse(saved);
      if (wallets.length > 0) {
        setWalletAddress(wallets[0].address);
      }
    }
  }, []);
  
  // Fetch validators
  useEffect(() => {
    const fetchData = async () => {
      try {
        const response = await axios.get(`${API}/validators`);
        setValidators(response.data.validators || []);
        setTotalStake(response.data.total_stake || 0);
      } catch (e) {
        console.error("Failed to fetch validators:", e);
      } finally {
        setLoading(false);
      }
    };
    
    fetchData();
    const interval = setInterval(fetchData, 10000);
    return () => clearInterval(interval);
  }, []);
  
  // Fetch staking info for wallet
  useEffect(() => {
    if (!walletAddress) return;
    
    const fetchStakingInfo = async () => {
      try {
        const response = await axios.get(`${API}/staking/${walletAddress}`);
        setStakingInfo(response.data);
      } catch (e) {
        console.error("Failed to fetch staking info:", e);
      }
    };
    
    fetchStakingInfo();
    const interval = setInterval(fetchStakingInfo, 5000);
    return () => clearInterval(interval);
  }, [walletAddress]);
  
  const handleStake = async (validatorAddress, amount) => {
    await axios.post(`${API}/staking/stake`, {
      delegator: walletAddress,
      validator: validatorAddress,
      amount: amount
    });
  };
  
  const handleUnstake = async (validatorAddress, amount) => {
    await axios.post(`${API}/staking/unstake`, {
      delegator: walletAddress,
      validator: validatorAddress,
      amount: amount
    });
  };
  
  const getCurrentStake = () => {
    if (!stakingInfo || !selectedValidator) return 0;
    return stakingInfo.stakes?.[selectedValidator.address] || 0;
  };
  
  if (loading) {
    return <div className="text-center py-12 text-zinc-500">Loading staking info...</div>;
  }
  
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="space-y-6"
    >
      <div className="flex items-center justify-between">
        <h1 className="font-unbounded text-2xl font-bold">Staking</h1>
        <span className="badge-success">Core L1 Feature</span>
      </div>
      
      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="card rounded-sm p-4">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-sm bg-cyan-500/10">
              <Coins className="w-5 h-5 text-cyan-500" />
            </div>
            <div>
              <p className="font-mono text-xs text-zinc-500">TOTAL STAKED</p>
              <p className="font-unbounded text-2xl font-bold">{(totalStake / 1_000_000_000_000).toFixed(0)}K JJ</p>
            </div>
          </div>
        </div>
        <div className="card rounded-sm p-4">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-sm bg-purple-500/10">
              <Users className="w-5 h-5 text-purple-500" />
            </div>
            <div>
              <p className="font-mono text-xs text-zinc-500">VALIDATORS</p>
              <p className="font-unbounded text-2xl font-bold">{validators.length}</p>
            </div>
          </div>
        </div>
        <div className="card rounded-sm p-4">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-sm bg-green-500/10">
              <TrendingUp className="w-5 h-5 text-green-500" />
            </div>
            <div>
              <p className="font-mono text-xs text-zinc-500">YOUR STAKED</p>
              <p className="font-unbounded text-2xl font-bold">
                {stakingInfo ? (stakingInfo.total_staked / 1_000_000_000).toFixed(2) : "0"} JJ
              </p>
            </div>
          </div>
        </div>
        <div className="card rounded-sm p-4">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-sm bg-yellow-500/10">
              <Coins className="w-5 h-5 text-yellow-500" />
            </div>
            <div>
              <p className="font-mono text-xs text-zinc-500">YOUR BALANCE</p>
              <p className="font-unbounded text-2xl font-bold">
                {stakingInfo ? (stakingInfo.balance / 1_000_000_000).toFixed(2) : "0"} JJ
              </p>
            </div>
          </div>
        </div>
      </div>
      
      {/* Main content */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Validator list */}
        <div className="lg:col-span-2 space-y-4">
          <h3 className="font-unbounded text-lg font-semibold">Select Validator</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {validators.map((validator) => (
              <ValidatorCard
                key={validator.address}
                validator={validator}
                onSelect={setSelectedValidator}
                isSelected={selectedValidator?.address === validator.address}
                totalStake={totalStake}
              />
            ))}
          </div>
        </div>
        
        {/* Stake form */}
        <div>
          <StakeForm
            selectedValidator={selectedValidator}
            walletAddress={walletAddress}
            onStake={handleStake}
            onUnstake={handleUnstake}
            currentStake={getCurrentStake()}
          />
          
          {/* Your stakes */}
          {stakingInfo && Object.keys(stakingInfo.stakes || {}).length > 0 && (
            <div className="card rounded-sm mt-4 p-4">
              <h4 className="font-unbounded text-sm font-semibold mb-3">Your Active Stakes</h4>
              <div className="space-y-2">
                {Object.entries(stakingInfo.stakes).map(([validatorAddr, amount]) => {
                  const validator = validators.find(v => v.address === validatorAddr);
                  return (
                    <div key={validatorAddr} className="flex items-center justify-between p-2 bg-zinc-900 rounded-sm">
                      <span className="font-mono text-xs text-zinc-400">
                        {validator?.name || validatorAddr.slice(0, 12)}...
                      </span>
                      <span className="font-mono text-sm text-white">
                        {(amount / 1_000_000_000).toFixed(4)} JJ
                      </span>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>
      </div>
    </motion.div>
  );
}
