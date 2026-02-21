"""JasprChain API Server
FastAPI backend for JasprChain L1 Blockchain

CORE L1 ONLY - No application layer (DEX, NFTs, etc.)
"""
from fastapi import FastAPI, APIRouter, HTTPException, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, Field
from typing import List, Optional, Dict, Any
import asyncio
import json
import os
from datetime import datetime, timezone

from jasprchain.engine import get_chain, JasprChain
from jasprchain.execution.transaction import TransactionType
from jasprchain.sentinel import GuardMode

# Initialize app
app = FastAPI(
    title="JasprChain L1 API",
    description="Layer 1 Blockchain API - High-performance, AI-protected",
    version="0.1.0"
)

# CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"]
)

api_router = APIRouter(prefix="/api")

# ============= Pydantic Models =============

class CreateWalletResponse(BaseModel):
    address: str
    public_key: str
    type: str
    threshold: str

class TransferRequest(BaseModel):
    sender: str
    recipient: str
    amount: int

class StakeRequest(BaseModel):
    delegator: str
    validator: str
    amount: int

class SentinelModeRequest(BaseModel):
    mode: str  # "passive", "warning", "enforced"

class MoveDeployRequest(BaseModel):
    sender: str
    name: str
    bytecode: str
    abi: dict

class MoveExecuteRequest(BaseModel):
    sender: str
    module_id: str
    function_name: str
    type_args: list = []
    args: list = []
    gas_limit: int = None

# ============= Chain Instance =============

chain = get_chain()

# ============= WebSocket Connections =============

class ConnectionManager:
    def __init__(self):
        self.active_connections: List[WebSocket] = []
    
    async def connect(self, websocket: WebSocket):
        await websocket.accept()
        self.active_connections.append(websocket)
    
    def disconnect(self, websocket: WebSocket):
        if websocket in self.active_connections:
            self.active_connections.remove(websocket)
    
    async def broadcast(self, message: dict):
        for connection in self.active_connections:
            try:
                await connection.send_json(message)
            except:
                pass

manager = ConnectionManager()

# ============= API Routes =============

@api_router.get("/")
async def root():
    return {
        "message": "JasprChain L1 API v0.1.0", 
        "status": "running",
        "network": "testnet",
        "token": "JASPR"
    }

@api_router.get("/health")
async def health():
    return {
        "status": "healthy", 
        "chain_id": chain.CHAIN_ID, 
        "height": chain.height,
        "network": "testnet"
    }

# ------------ Tokenomics ------------

@api_router.get("/tokenomics")
async def get_tokenomics():
    """Get $JASPR tokenomics from litepaper"""
    return chain.get_tokenomics()

# ------------ Network Stats ------------

@api_router.get("/network/stats")
async def get_network_stats():
    """Get comprehensive network statistics"""
    return chain.get_network_stats()

@api_router.get("/network/tps")
async def get_tps():
    """Get current TPS"""
    stats = chain.get_network_stats()
    return {
        "current_tps": round(stats['tps'], 2),
        "target_tps": stats['target_tps'],
        "total_transactions": stats['total_transactions']
    }

@api_router.get("/network/persistence")
async def get_persistence_stats():
    """Get persistence/database statistics"""
    db_stats = chain.persistence.get_db_stats()
    return {
        "persistence": "lmdb",
        "db_path": chain.persistence.db_path,
        "stats": db_stats,
        "latest_height": chain.persistence.get_latest_height()
    }

# ------------ Blocks ------------

@api_router.get("/blocks")
async def get_blocks(limit: int = 20, offset: int = 0):
    """Get recent blocks"""
    blocks = chain.blocks[-(limit + offset):][:limit] if offset == 0 else chain.blocks[-(limit + offset):-offset]
    return {
        "blocks": [b.to_summary() for b in reversed(blocks)],
        "total": len(chain.blocks),
        "latest_height": chain.height,
        "persisted": True
    }

@api_router.get("/blocks/{height_or_hash}")
async def get_block(height_or_hash: str):
    """Get block by height or hash"""
    try:
        key = int(height_or_hash)
    except ValueError:
        key = height_or_hash
    
    block = chain.get_block(key)
    if not block:
        raise HTTPException(status_code=404, detail="Block not found")
    return block.to_dict()

@api_router.post("/blocks/produce")
async def produce_block():
    """Manually produce a block (for testing)"""
    try:
        block = await chain.produce_block()
        
        # Broadcast to websocket clients
        await manager.broadcast({
            "type": "new_block",
            "data": block.to_summary()
        })
        
        return block.to_dict()
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

# ------------ Validators ------------

@api_router.get("/validators")
async def get_validators():
    """Get all validators"""
    return chain.validator_set.to_dict()

@api_router.get("/validators/{address}")
async def get_validator(address: str):
    """Get validator details"""
    validator = chain.validator_set.get_validator(address)
    if not validator:
        raise HTTPException(status_code=404, detail="Validator not found")
    return validator.to_dict()

# ------------ Wallets ------------

@api_router.post("/wallets/create", response_model=CreateWalletResponse)
async def create_wallet():
    """Create a new MPC wallet"""
    wallet = chain.create_wallet()
    return wallet.export_public_info()

@api_router.get("/wallets/{address}")
async def get_wallet(address: str):
    """Get wallet details"""
    wallet = chain.get_wallet(address)
    balance = chain.get_balance(address)
    aa_wallet = chain.account_abstraction.wallets.get(address)
    stakes = chain.get_all_stakes(address)
    
    return {
        "address": address,
        "balance": balance,
        "balance_formatted": f"{balance / 1_000_000_000:.4f} JASPR",
        "mpc_wallet": wallet.export_public_info() if wallet else None,
        "aa_wallet": aa_wallet.to_dict() if aa_wallet else None,
        "stakes": stakes,
        "total_staked": sum(stakes.values())
    }

@api_router.get("/wallets/{address}/balance")
async def get_balance(address: str):
    """Get wallet balance"""
    balance = chain.get_balance(address)
    return {
        "address": address,
        "balance": balance,
        "balance_formatted": f"{balance / 1_000_000_000:.4f} JASPR"
    }

# ------------ Transactions ------------

@api_router.post("/transactions/transfer")
async def create_transfer(request: TransferRequest):
    """Create and submit a transfer transaction"""
    wallet = chain.get_wallet(request.sender)
    if not wallet:
        raise HTTPException(status_code=404, detail="Sender wallet not found")
    
    # Create transaction
    tx = chain.create_transaction(
        sender=request.sender,
        recipient=request.recipient,
        amount=request.amount,
        tx_type=TransactionType.TRANSFER
    )
    
    # Sign transaction
    signed_tx = chain.sign_transaction(tx, wallet)
    
    # Submit to mempool
    success, message, entry = await chain.submit_transaction(signed_tx)
    
    if not success:
        raise HTTPException(status_code=400, detail=message)
    
    # Broadcast to websocket
    await manager.broadcast({
        "type": "new_transaction",
        "data": signed_tx.to_dict()
    })
    
    return {
        "success": True,
        "tx_hash": signed_tx.hash,
        "status": "pending",
        "risk_score": entry.risk_score.to_dict() if entry and entry.risk_score else None
    }

@api_router.get("/transactions/{tx_hash}")
async def get_transaction(tx_hash: str):
    """Get transaction by hash"""
    result = chain.get_transaction(tx_hash)
    if not result:
        raise HTTPException(status_code=404, detail="Transaction not found")
    return result

# ------------ Staking ------------

@api_router.get("/staking/stats/overview")
async def get_staking_stats():
    """Get global staking statistics"""
    total_stake = chain.validator_set.total_stake()
    validators = chain.validator_set.get_active_validators()
    
    # Calculate network-wide stats
    total_delegations = 0
    for v in validators:
        # Count unique delegators (simplified - in production would track this)
        total_delegations += 1
    
    avg_apy = sum(chain.calculate_validator_apy(v.address) for v in validators) / len(validators) if validators else 0
    
    return {
        "total_staked": total_stake,
        "total_staked_formatted": f"{total_stake / 1_000_000_000_000:.2f}K JASPR",
        "active_validators": len(validators),
        "average_apy": round(avg_apy, 2),
        "network_security_ratio": round((total_stake / (total_stake + 1_000_000_000_000)) * 100, 2),
        "unbonding_period_days": 14
    }

@api_router.get("/staking/validators")
async def get_validators_for_staking():
    """Get validators with APY information for staking"""
    validators = chain.validator_set.get_active_validators()
    total_stake = chain.validator_set.total_stake()
    
    result = []
    for v in validators:
        apy = chain.calculate_validator_apy(v.address)
        stake_share = (v.stake / total_stake * 100) if total_stake > 0 else 0
        
        result.append({
            "address": v.address,
            "name": v.name,
            "stake": v.stake,
            "stake_formatted": f"{v.stake / 1_000_000_000_000:.2f}K JASPR",
            "stake_share": round(stake_share, 2),
            "apy": apy,
            "commission_rate": v.commission_rate / 100,  # Convert basis points to percentage
            "uptime": v.stats.uptime_percentage,
            "blocks_proposed": v.stats.blocks_proposed,
            "active": v.active,
            "jailed": v.jailed
        })
    
    return {
        "validators": result,
        "total_stake": total_stake,
        "count": len(result)
    }

@api_router.post("/staking/stake")
async def stake_tokens(request: StakeRequest):
    """Stake tokens to a validator"""
    success, message = chain.stake(
        delegator=request.delegator,
        validator=request.validator,
        amount=request.amount
    )
    
    if not success:
        raise HTTPException(status_code=400, detail=message)
    
    return {
        "success": True,
        "message": message,
        "delegator": request.delegator,
        "validator": request.validator,
        "amount": request.amount
    }

@api_router.post("/staking/unstake")
async def unstake_tokens(request: StakeRequest):
    """Unstake tokens from a validator"""
    success, message = chain.unstake(
        delegator=request.delegator,
        validator=request.validator,
        amount=request.amount
    )
    
    if not success:
        raise HTTPException(status_code=400, detail=message)
    
    return {
        "success": True,
        "message": message,
        "delegator": request.delegator,
        "validator": request.validator,
        "amount": request.amount,
        "unbonding_period_days": chain.UNBONDING_PERIOD_DAYS
    }

@api_router.get("/staking/{address}/unbonding")
async def get_unbonding_entries(address: str):
    """Get unbonding entries for an address"""
    entries = chain.get_unbonding_entries(address)
    total_unbonding = sum(e['amount'] for e in entries)
    claimable = sum(e['amount'] for e in entries if e['is_claimable'])
    
    return {
        "address": address,
        "unbonding_entries": entries,
        "total_unbonding": total_unbonding,
        "claimable_amount": claimable,
        "unbonding_period_days": chain.UNBONDING_PERIOD_DAYS
    }

@api_router.post("/staking/{address}/claim")
async def claim_unbonded_tokens(address: str):
    """Claim tokens that have completed unbonding"""
    success, message = chain.claim_unbonded(address)
    
    if not success:
        raise HTTPException(status_code=400, detail=message)
    
    return {
        "success": True,
        "message": message,
        "new_balance": chain.get_balance(address)
    }

@api_router.get("/staking/{address}")
async def get_staking_info(address: str):
    """Get staking info for an address"""
    stakes = chain.get_all_stakes(address)
    balance = chain.get_balance(address)
    unbonding_entries = chain.get_unbonding_entries(address)
    
    # Calculate estimated rewards
    validators_with_rewards = []
    for v_addr, amount in stakes.items():
        validator = chain.validator_set.get_validator(v_addr)
        if validator:
            apy = chain.calculate_validator_apy(v_addr)
            estimated_daily_reward = (amount * apy / 100) / 365
            validators_with_rewards.append({
                "address": v_addr,
                "staked": amount,
                "apy": apy,
                "estimated_daily_reward": int(estimated_daily_reward),
                "validator_info": validator.to_dict()
            })
    
    return {
        "address": address,
        "balance": balance,
        "stakes": stakes,
        "total_staked": sum(stakes.values()),
        "validators": validators_with_rewards,
        "unbonding": {
            "entries": unbonding_entries,
            "total_unbonding": sum(e['amount'] for e in unbonding_entries),
            "claimable": sum(e['amount'] for e in unbonding_entries if e['is_claimable'])
        },
        "unbonding_period_days": chain.UNBONDING_PERIOD_DAYS
    }

# ------------ Mempool ------------

@api_router.get("/mempool")
async def get_mempool():
    """Get mempool statistics"""
    return {
        "stats": chain.mempool.get_stats().to_dict(),
        "quarantined": chain.mempool.get_quarantined()
    }

@api_router.get("/mempool/pending/{address}")
async def get_pending_transactions(address: str):
    """Get pending transactions for an address"""
    pending = chain.mempool.get_pending_by_sender(address)
    return {
        "address": address,
        "pending_count": len(pending),
        "transactions": [tx.to_dict() for tx in pending]
    }

# ------------ AI Sentinel ------------

@api_router.get("/sentinel")
async def get_sentinel_status():
    """Get AI Sentinel status"""
    return chain.sentinel.to_dict()

@api_router.get("/sentinel/stats")
async def get_sentinel_stats():
    """Get AI Sentinel statistics"""
    return chain.sentinel.get_stats()

@api_router.get("/sentinel/quarantine")
async def get_quarantine():
    """Get quarantined transactions"""
    return {
        "quarantined": [r.to_dict() for r in chain.sentinel.get_quarantined()]
    }

@api_router.post("/sentinel/mode")
async def set_sentinel_mode(request: SentinelModeRequest):
    """Set AI Sentinel guard mode"""
    mode_map = {
        "passive": GuardMode.PASSIVE,
        "warning": GuardMode.WARNING,
        "enforced": GuardMode.ENFORCED
    }
    mode = mode_map.get(request.mode)
    if not mode:
        raise HTTPException(status_code=400, detail="Invalid mode")
    
    chain.sentinel.set_guard_mode(mode)
    return {"success": True, "mode": request.mode}

# ------------ Slashing ------------

@api_router.get("/slashing/stats")
async def get_slashing_stats():
    """Get slashing statistics"""
    return chain.slashing.get_stats()

@api_router.get("/slashing/history")
async def get_slashing_history(validator: str = None):
    """Get slashing history"""
    return {
        "records": chain.slashing.get_slashing_history(validator),
        "total": len(chain.slashing.slashing_records)
    }

@api_router.get("/slashing/validator/{address}")
async def get_validator_signing_info(address: str):
    """Get signing info for a validator"""
    info = chain.slashing.get_validator_signing_info(address)
    if not info:
        raise HTTPException(status_code=404, detail="Validator not found")
    return info

@api_router.post("/slashing/unjail/{address}")
async def unjail_validator(address: str):
    """Attempt to unjail a validator"""
    success, message = chain.slashing.unjail_validator(address, chain.validator_set)
    if not success:
        raise HTTPException(status_code=400, detail=message)
    return {"success": True, "message": message}

# ------------ P2P Network ------------

@api_router.get("/p2p/info")
async def get_p2p_info():
    """Get P2P network info"""
    return {
        "node_id": chain.p2p.node_id,
        "version": chain.p2p.VERSION,
        "is_running": chain.p2p.is_running,
        "connected_peers": len(chain.p2p.peers),
        "chain_height": chain.height
    }

@api_router.get("/p2p/peers")
async def get_peers():
    """Get connected peers"""
    return {
        "peers": chain.p2p.get_peer_list(),
        "count": len(chain.p2p.peers)
    }

@api_router.get("/p2p/stats")
async def get_p2p_stats():
    """Get P2P statistics"""
    return chain.p2p.get_stats()

@api_router.post("/p2p/connect")
async def connect_to_peer(address: str, port: int = 30303):
    """Connect to a peer"""
    peer = await chain.p2p.connect_to_peer(address, port)
    if peer:
        return {"success": True, "peer": peer.to_dict()}
    raise HTTPException(status_code=400, detail="Failed to connect to peer")

# ------------ Move VM ------------

@api_router.get("/move/modules")
async def list_move_modules(address: str = None):
    """List deployed Move modules"""
    return {
        "modules": chain.move_vm.list_modules(address),
        "total": len(chain.move_vm.modules)
    }

@api_router.get("/move/module/{module_id:path}")
async def get_move_module(module_id: str):
    """Get a specific Move module"""
    module = chain.move_vm.get_module(module_id)
    if not module:
        raise HTTPException(status_code=404, detail="Module not found")
    return module.to_dict()

@api_router.post("/move/deploy")
async def deploy_move_module(request: MoveDeployRequest):
    """Deploy a Move module"""
    result = chain.move_vm.deploy_module(request.sender, request.name, request.bytecode, request.abi)
    if not result.success:
        raise HTTPException(status_code=400, detail=result.error)
    return result.to_dict()

@api_router.post("/move/execute")
async def execute_move_function(request: MoveExecuteRequest):
    """Execute a Move function"""
    result = chain.move_vm.execute_function(
        request.sender, request.module_id, request.function_name, 
        request.type_args, request.args, request.gas_limit
    )
    if not result.success:
        raise HTTPException(status_code=400, detail=result.error)
    return result.to_dict()

@api_router.get("/move/estimate-gas")
async def estimate_move_gas(
    module_id: str,
    function_name: str,
    type_args: str = "[]",
    args: str = "[]"
):
    """Estimate gas for a Move function call"""
    import json
    type_args_list = json.loads(type_args)
    args_list = json.loads(args)
    gas = chain.move_vm.estimate_gas(module_id, function_name, type_args_list, args_list)
    return {"estimated_gas": gas, "gas_unit_price": chain.move_vm.GAS_UNIT_PRICE}

@api_router.get("/move/stats")
async def get_move_vm_stats():
    """Get Move VM statistics"""
    return chain.move_vm.get_stats()

# ------------ WebSocket ------------

@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    """WebSocket for real-time updates"""
    await manager.connect(websocket)
    try:
        while True:
            # Keep connection alive and handle any incoming messages
            data = await websocket.receive_text()
            message = json.loads(data)
            
            # Handle subscription requests
            if message.get("type") == "subscribe":
                channel = message.get("channel")
                await websocket.send_json({
                    "type": "subscribed",
                    "channel": channel
                })
    except WebSocketDisconnect:
        manager.disconnect(websocket)

# Include router
app.include_router(api_router)

# Background task for auto block production
async def auto_produce_blocks():
    """Automatically produce blocks every 2 seconds - REAL-TIME like actual blockchain"""
    while True:
        await asyncio.sleep(2)  # 2 second block time
        try:
            # Produce block regardless of pending transactions
            # Real blockchains produce blocks even if empty
            block = await chain.produce_block()
            await manager.broadcast({
                "type": "new_block",
                "data": block.to_summary()
            })
        except Exception as e:
            print(f"Block production error: {e}")

@app.on_event("startup")
async def startup_event():
    """Start background tasks"""
    asyncio.create_task(auto_produce_blocks())

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8001)
