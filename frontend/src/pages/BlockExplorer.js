import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { useParams, Link } from "react-router-dom";
import { Blocks, ArrowLeft, ArrowRight, Copy, Check, Clock, User, Database, Hash } from "lucide-react";
import axios from "axios";

const BACKEND_URL = process.env.REACT_APP_BACKEND_URL;
const API = `${BACKEND_URL}/api`;

function BlockList() {
  const [blocks, setBlocks] = useState([]);
  const [loading, setLoading] = useState(true);
  
  useEffect(() => {
    const fetchBlocks = async () => {
      try {
        const response = await axios.get(`${API}/blocks?limit=20`);
        setBlocks(response.data.blocks || []);
      } catch (e) {
        console.error("Failed to fetch blocks:", e);
      } finally {
        setLoading(false);
      }
    };
    
    fetchBlocks();
    const interval = setInterval(fetchBlocks, 5000);
    return () => clearInterval(interval);
  }, []);
  
  if (loading) {
    return <div className="text-center py-12 text-zinc-500">Loading blocks...</div>;
  }
  
  return (
    <div className="card rounded-sm overflow-hidden" data-testid="block-list">
      <div className="table-header">
        <div className="grid grid-cols-12 gap-4 px-4 py-3">
          <div className="col-span-2">HEIGHT</div>
          <div className="col-span-4">HASH</div>
          <div className="col-span-2">PROPOSER</div>
          <div className="col-span-2">TXNS</div>
          <div className="col-span-2">TIME</div>
        </div>
      </div>
      <div className="divide-y divide-zinc-800">
        {blocks.map((block) => (
          <Link
            key={block.hash}
            to={`/blocks/${block.height}`}
            className="table-row grid grid-cols-12 gap-4 px-4 py-3 items-center"
            data-testid={`block-row-${block.height}`}
          >
            <div className="col-span-2">
              <span className="font-mono text-cyan-500">#{block.height}</span>
            </div>
            <div className="col-span-4">
              <span className="font-mono text-xs text-zinc-400">
                {block.hash.slice(0, 16)}...{block.hash.slice(-8)}
              </span>
            </div>
            <div className="col-span-2">
              <span className="font-mono text-xs text-zinc-400">
                {block.proposer?.slice(0, 12)}...
              </span>
            </div>
            <div className="col-span-2">
              <span className="font-mono text-white">{block.tx_count}</span>
            </div>
            <div className="col-span-2 flex items-center gap-2">
              <span className="font-mono text-xs text-zinc-400">
                {new Date(block.timestamp).toLocaleTimeString()}
              </span>
              {block.finalized && (
                <span className="badge-success">Final</span>
              )}
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}

function BlockDetail({ heightOrHash }) {
  const [block, setBlock] = useState(null);
  const [loading, setLoading] = useState(true);
  const [copied, setCopied] = useState(false);
  
  useEffect(() => {
    const fetchBlock = async () => {
      try {
        const response = await axios.get(`${API}/blocks/${heightOrHash}`);
        setBlock(response.data);
      } catch (e) {
        console.error("Failed to fetch block:", e);
      } finally {
        setLoading(false);
      }
    };
    
    fetchBlock();
  }, [heightOrHash]);
  
  const copyHash = () => {
    navigator.clipboard.writeText(block?.hash || "");
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };
  
  if (loading) {
    return <div className="text-center py-12 text-zinc-500">Loading block...</div>;
  }
  
  if (!block) {
    return <div className="text-center py-12 text-red-500">Block not found</div>;
  }
  
  return (
    <div className="space-y-6" data-testid="block-detail">
      {/* Navigation */}
      <div className="flex items-center justify-between">
        <Link to="/blocks" className="flex items-center gap-2 text-zinc-400 hover:text-white">
          <ArrowLeft className="w-4 h-4" />
          <span className="font-mono text-sm">Back to Blocks</span>
        </Link>
        <div className="flex items-center gap-2">
          {block.header.height > 0 && (
            <Link
              to={`/blocks/${block.header.height - 1}`}
              className="btn-secondary flex items-center gap-1"
            >
              <ArrowLeft className="w-4 h-4" />
              Prev
            </Link>
          )}
          <Link
            to={`/blocks/${block.header.height + 1}`}
            className="btn-secondary flex items-center gap-1"
          >
            Next
            <ArrowRight className="w-4 h-4" />
          </Link>
        </div>
      </div>
      
      {/* Block header */}
      <div className="card rounded-sm p-6">
        <div className="flex items-start justify-between mb-6">
          <div>
            <h2 className="font-unbounded text-2xl font-bold">
              Block #{block.header.height}
            </h2>
            <div className="flex items-center gap-2 mt-2">
              <span className="font-mono text-sm text-zinc-400">{block.hash}</span>
              <button
                onClick={copyHash}
                className="p-1 hover:bg-zinc-800 rounded transition-colors"
                data-testid="copy-hash-btn"
              >
                {copied ? <Check className="w-4 h-4 text-green-500" /> : <Copy className="w-4 h-4 text-zinc-500" />}
              </button>
            </div>
          </div>
          {block.finalized ? (
            <span className="badge-success text-sm px-3 py-1">Finalized</span>
          ) : (
            <span className="badge-warning text-sm px-3 py-1">Pending</span>
          )}
        </div>
        
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-6">
          <div>
            <p className="font-mono text-xs text-zinc-500 uppercase tracking-widest mb-1">Timestamp</p>
            <div className="flex items-center gap-2">
              <Clock className="w-4 h-4 text-cyan-500" />
              <span className="font-mono text-sm">{new Date(block.header.timestamp).toLocaleString()}</span>
            </div>
          </div>
          <div>
            <p className="font-mono text-xs text-zinc-500 uppercase tracking-widest mb-1">Proposer</p>
            <div className="flex items-center gap-2">
              <User className="w-4 h-4 text-purple-500" />
              <span className="font-mono text-sm">{block.header.proposer?.slice(0, 16)}...</span>
            </div>
          </div>
          <div>
            <p className="font-mono text-xs text-zinc-500 uppercase tracking-widest mb-1">Transactions</p>
            <div className="flex items-center gap-2">
              <Database className="w-4 h-4 text-green-500" />
              <span className="font-mono text-sm">{block.transaction_count}</span>
            </div>
          </div>
          <div>
            <p className="font-mono text-xs text-zinc-500 uppercase tracking-widest mb-1">Finality Time</p>
            <div className="flex items-center gap-2">
              <Blocks className="w-4 h-4 text-yellow-500" />
              <span className="font-mono text-sm">{block.finality_time_ms || 0}ms</span>
            </div>
          </div>
        </div>
      </div>
      
      {/* State roots */}
      <div className="card rounded-sm p-6">
        <h3 className="font-unbounded text-lg font-semibold mb-4">State Roots</h3>
        <div className="space-y-3">
          <div className="flex items-center gap-3">
            <Hash className="w-4 h-4 text-cyan-500" />
            <span className="font-mono text-xs text-zinc-500 w-32">STATE ROOT</span>
            <span className="font-mono text-xs text-zinc-400">{block.header.state_root}</span>
          </div>
          <div className="flex items-center gap-3">
            <Hash className="w-4 h-4 text-purple-500" />
            <span className="font-mono text-xs text-zinc-500 w-32">TX ROOT</span>
            <span className="font-mono text-xs text-zinc-400">{block.header.transactions_root}</span>
          </div>
          <div className="flex items-center gap-3">
            <Hash className="w-4 h-4 text-green-500" />
            <span className="font-mono text-xs text-zinc-500 w-32">RECEIPTS ROOT</span>
            <span className="font-mono text-xs text-zinc-400">{block.header.receipts_root}</span>
          </div>
        </div>
      </div>
      
      {/* Transactions */}
      {block.transactions && block.transactions.length > 0 && (
        <div className="card rounded-sm">
          <div className="p-4 border-b border-zinc-800">
            <h3 className="font-unbounded text-lg font-semibold">Transactions ({block.transactions.length})</h3>
          </div>
          <div className="divide-y divide-zinc-800">
            {block.transactions.map((tx, i) => (
              <div key={tx.hash || i} className="p-4">
                <div className="flex items-center justify-between">
                  <span className="font-mono text-sm text-cyan-500">{tx.hash?.slice(0, 24)}...</span>
                  <span className="badge-success">{tx.type}</span>
                </div>
                <div className="mt-2 grid grid-cols-3 gap-4 text-xs">
                  <div>
                    <span className="text-zinc-500">From: </span>
                    <span className="text-zinc-400">{tx.sender?.slice(0, 16)}...</span>
                  </div>
                  <div>
                    <span className="text-zinc-500">To: </span>
                    <span className="text-zinc-400">{tx.recipient?.slice(0, 16) || "N/A"}...</span>
                  </div>
                  <div>
                    <span className="text-zinc-500">Amount: </span>
                    <span className="text-white">{(tx.amount / 1_000_000_000).toFixed(4)} JASPR</span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export default function BlockExplorer() {
  const { heightOrHash } = useParams();
  
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="space-y-6"
    >
      <div className="flex items-center justify-between">
        <h1 className="font-unbounded text-2xl font-bold">Block Explorer</h1>
      </div>
      
      {heightOrHash ? (
        <BlockDetail heightOrHash={heightOrHash} />
      ) : (
        <BlockList />
      )}
    </motion.div>
  );
}
