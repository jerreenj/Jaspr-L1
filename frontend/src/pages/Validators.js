import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { Users, Shield, TrendingUp, Clock, CheckCircle, XCircle } from "lucide-react";
import axios from "axios";

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;
const API = `${BACKEND_URL}/api`;

export default function Validators() {
  const [validatorData, setValidatorData] = useState(null);
  const [loading, setLoading] = useState(true);
  
  useEffect(() => {
    const fetchValidators = async () => {
      try {
        const response = await axios.get(`${API}/validators`);
        setValidatorData(response.data);
      } catch (e) {
        console.error("Failed to fetch validators:", e);
      } finally {
        setLoading(false);
      }
    };
    
    fetchValidators();
    const interval = setInterval(fetchValidators, 10000);
    return () => clearInterval(interval);
  }, []);
  
  if (loading) {
    return <div className="text-center py-12 text-zinc-500">Loading validators...</div>;
  }
  
  const validators = validatorData?.validators || [];
  const totalStake = validatorData?.total_stake || 0;
  
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="space-y-4 md:space-y-6"
    >
      <h1 className="font-unbounded text-xl md:text-2xl font-bold">Validators</h1>
      
      {/* Stats - mobile responsive grid */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-2 md:gap-4">
        <div className="card rounded-sm p-3 md:p-4">
          <div className="flex items-center gap-2 md:gap-3">
            <div className="p-1.5 md:p-2 rounded-sm bg-cyan-500/10">
              <Users className="w-4 h-4 md:w-5 md:h-5 text-cyan-500" />
            </div>
            <div>
              <p className="font-mono text-[10px] md:text-xs text-zinc-500">VALIDATORS</p>
              <p className="font-unbounded text-lg md:text-2xl font-bold">{validatorData?.total_validators || 0}</p>
            </div>
          </div>
        </div>
        <div className="card rounded-sm p-3 md:p-4">
          <div className="flex items-center gap-2 md:gap-3">
            <div className="p-1.5 md:p-2 rounded-sm bg-green-500/10">
              <CheckCircle className="w-4 h-4 md:w-5 md:h-5 text-green-500" />
            </div>
            <div>
              <p className="font-mono text-[10px] md:text-xs text-zinc-500">ACTIVE</p>
              <p className="font-unbounded text-lg md:text-2xl font-bold text-green-500">{validatorData?.active_validators || 0}</p>
            </div>
          </div>
        </div>
        <div className="card rounded-sm p-3 md:p-4">
          <div className="flex items-center gap-2 md:gap-3">
            <div className="p-1.5 md:p-2 rounded-sm bg-purple-500/10">
              <TrendingUp className="w-4 h-4 md:w-5 md:h-5 text-purple-500" />
            </div>
            <div>
              <p className="font-mono text-[10px] md:text-xs text-zinc-500">STAKED</p>
              <p className="font-unbounded text-lg md:text-2xl font-bold">{(totalStake / 1_000_000_000_000).toFixed(0)}K</p>
            </div>
          </div>
        </div>
        <div className="card rounded-sm p-3 md:p-4">
          <div className="flex items-center gap-2 md:gap-3">
            <div className="p-1.5 md:p-2 rounded-sm bg-yellow-500/10">
              <Shield className="w-4 h-4 md:w-5 md:h-5 text-yellow-500" />
            </div>
            <div>
              <p className="font-mono text-[10px] md:text-xs text-zinc-500">POWER</p>
              <p className="font-unbounded text-lg md:text-2xl font-bold">{(validatorData?.total_voting_power / 1_000_000_000_000).toFixed(0)}K</p>
            </div>
          </div>
        </div>
      </div>
      
      {/* Validator list - mobile cards, desktop table */}
      <div className="card rounded-sm" data-testid="validator-table">
        {/* Desktop header */}
        <div className="table-header hidden md:block">
          <div className="grid grid-cols-12 gap-4 px-4 py-3">
            <div className="col-span-1">RANK</div>
            <div className="col-span-3">VALIDATOR</div>
            <div className="col-span-2">STAKE</div>
            <div className="col-span-2">VOTING POWER</div>
            <div className="col-span-2">PERFORMANCE</div>
            <div className="col-span-2">STATUS</div>
          </div>
        </div>
        
        {/* Mobile header */}
        <div className="md:hidden px-4 py-3 border-b border-zinc-800">
          <p className="font-mono text-xs text-zinc-500">ALL VALIDATORS ({validators.length})</p>
        </div>
        
        <div className="divide-y divide-zinc-800">
          {validators.map((validator, i) => (
            <div
              key={validator.address}
              className="p-3 md:p-4"
              data-testid={`validator-row-${i}`}
            >
              {/* Mobile layout */}
              <div className="md:hidden">
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <span className="font-unbounded font-bold text-sm text-zinc-400">#{i + 1}</span>
                    <span className="font-manrope font-medium text-white text-sm">{validator.name || "Validator"}</span>
                  </div>
                  <span className={`badge-${validator.active ? 'success' : 'danger'} text-xs`}>
                    {validator.active ? 'Active' : 'Inactive'}
                  </span>
                </div>
                <div className="grid grid-cols-2 gap-2 text-xs">
                  <div>
                    <p className="font-mono text-zinc-500">Stake</p>
                    <p className="font-mono text-white">{(validator.stake / 1_000_000_000_000).toFixed(1)}K</p>
                  </div>
                  <div>
                    <p className="font-mono text-zinc-500">Share</p>
                    <p className="font-mono text-cyan-500">{((validator.stake / totalStake) * 100).toFixed(1)}%</p>
                  </div>
                </div>
              </div>
              
              {/* Desktop layout */}
              <div className="hidden md:grid grid-cols-12 gap-4 items-center">
              <div className="col-span-1">
                <span className="font-unbounded font-bold text-lg text-zinc-400">#{i + 1}</span>
              </div>
              <div className="col-span-3">
                <p className="font-manrope font-medium text-white">{validator.name || "Validator"}</p>
                <p className="font-mono text-xs text-zinc-500">
                  {validator.address.slice(0, 16)}...{validator.address.slice(-4)}
                </p>
              </div>
              <div className="col-span-2">
                <p className="font-mono text-white">{(validator.stake / 1_000_000_000_000).toFixed(2)}K JASPR</p>
                <p className="font-mono text-xs text-zinc-500">
                  {((validator.stake / totalStake) * 100).toFixed(1)}% of total
                </p>
              </div>
              <div className="col-span-2">
                <div className="w-full bg-zinc-800 rounded-full h-2">
                  <div 
                    className="bg-cyan-500 h-2 rounded-full"
                    style={{ width: `${(validator.voting_power / totalStake) * 100}%` }}
                  />
                </div>
                <p className="font-mono text-xs text-zinc-400 mt-1">
                  {((validator.voting_power / totalStake) * 100).toFixed(1)}%
                </p>
              </div>
              <div className="col-span-2">
                <div className="space-y-1">
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-zinc-500">Blocks</span>
                    <span className="text-green-500">{validator.stats?.blocks_proposed || 0}</span>
                  </div>
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-zinc-500">Attested</span>
                    <span className="text-cyan-500">{validator.stats?.blocks_attested || 0}</span>
                  </div>
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-zinc-500">Uptime</span>
                    <span className="text-white">{validator.stats?.uptime_percentage?.toFixed(1) || 100}%</span>
                  </div>
                </div>
              </div>
              <div className="col-span-2">
                <div className="flex items-center gap-2">
                  {validator.active && !validator.jailed ? (
                    <>
                      <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
                      <span className="badge-success">Active</span>
                    </>
                  ) : validator.jailed ? (
                    <>
                      <div className="w-2 h-2 rounded-full bg-red-500" />
                      <span className="badge-danger">Jailed</span>
                    </>
                  ) : (
                    <>
                      <div className="w-2 h-2 rounded-full bg-yellow-500" />
                      <span className="badge-warning">Inactive</span>
                    </>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </motion.div>
  );
}
