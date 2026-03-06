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

# ------------ Genesis ------------

@api_router.get("/genesis")
async def get_genesis():
    """Get genesis configuration file"""
    import json
    with open("/app/backend/genesis.json", "r") as f:
        return json.load(f)

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
        "balance_formatted": f"{balance} JASPR",
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
        "balance_formatted": f"{balance} JASPR"
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

@api_router.get("/transactions/recent")
async def get_recent_transactions(limit: int = 20):
    """Get recent transactions from all blocks"""
    transactions = []
    
    # Get transactions from recent blocks
    for block in reversed(chain.blocks[-50:]):  # Last 50 blocks
        block_txs = block.transactions if hasattr(block, 'transactions') else []
        for tx in block_txs:
            # tx is a dict from to_dict()
            transactions.append({
                "hash": tx.get("hash", ""),
                "type": tx.get("type", "transfer"),
                "sender": tx.get("sender", ""),
                "recipient": tx.get("recipient", ""),
                "amount": tx.get("amount", 0),
                "block_height": block.height,
                "timestamp": block.timestamp,
                "status": "confirmed"
            })
            if len(transactions) >= limit:
                break
        if len(transactions) >= limit:
            break
    
    # Also add pending transactions from mempool
    pending = chain.mempool.get_pending()
    for entry in pending[:5]:
        tx = entry.transaction if hasattr(entry, 'transaction') else entry
        inner = tx.transaction if hasattr(tx, 'transaction') else tx
        transactions.insert(0, {
            "hash": tx.hash if hasattr(tx, 'hash') else "",
            "type": inner.tx_type.value if hasattr(inner, 'tx_type') else "transfer",
            "sender": inner.sender if hasattr(inner, 'sender') else "",
            "recipient": inner.recipient if hasattr(inner, 'recipient') else "",
            "amount": inner.amount if hasattr(inner, 'amount') else 0,
            "block_height": None,
            "timestamp": int(datetime.now(timezone.utc).timestamp() * 1000),
            "status": "pending"
        })
    
    return {
        "transactions": transactions[:limit],
        "total": len(transactions)
    }


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
        "total_staked_formatted": f"{total_stake} JASPR",
        "active_validators": len(validators),
        "average_apy": round(avg_apy, 2),
        "network_security_ratio": round((total_stake / (total_stake + 1_000_000)) * 100, 2),
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
            "stake_formatted": f"{v.stake} JASPR",
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
            
            # Also broadcast network stats update
            await manager.broadcast({
                "type": "stats_update",
                "data": {
                    "height": chain.height,
                    "tps": chain.get_network_stats()['tps'],
                    "total_transactions": chain._total_transactions,
                    "total_staked": chain.validator_set.total_stake()
                }
            })
        except Exception as e:
            print(f"Block production error: {e}")

# Background task for continuous staking simulation
async def simulate_staking_activity():
    """Simulate ACTIVE staking activity on the testnet
    
    - Random stakes/unstakes happen periodically
    - Creates actual transactions that show in explorer
    - Shows real testnet activity
    """
    import random
    
    # Pool of simulated addresses
    simulated_addresses = [f"jaspr1sim_{i:04d}" for i in range(1, 101)]
    
    # Initialize simulated addresses with balance from community pool
    initial_allocation = 50_000  # 50K JASPR per simulated address
    for addr in simulated_addresses[:20]:
        balance = chain.get_balance(addr)
        if balance == 0:
            community_balance = chain.state.get_account_balance("jaspr1treasury_community")
            if community_balance >= initial_allocation:
                chain.state.set_account_balance("jaspr1treasury_community", community_balance - initial_allocation)
                chain.state.set_account_balance(addr, initial_allocation)
                chain.persistence.save_state(f"balance:{addr}", initial_allocation)
    
    while True:
        # Random interval: 5-15 seconds for more activity
        wait_time = random.randint(5, 15)
        await asyncio.sleep(wait_time)
        
        try:
            # Pick random simulated address
            delegator = random.choice(simulated_addresses[:20])
            
            # Pick random validator
            validators = chain.validator_set.get_active_validators()
            if not validators:
                continue
            validator = random.choice(validators)
            
            # Random amount: 100-5000 JASPR
            amount = random.randint(100, 5000)
            
            # Random action: 60% stake, 20% unstake, 20% transfer
            action = random.choices(['stake', 'unstake', 'transfer'], weights=[60, 20, 20])[0]
            
            if action == 'stake':
                balance = chain.get_balance(delegator)
                if balance >= amount:
                    # Create stake transaction
                    from jasprchain.execution.transaction import SignedTransaction, Transaction, TransactionType
                    tx = Transaction(
                        tx_type=TransactionType.STAKE,
                        sender=delegator,
                        recipient=validator.address,
                        amount=amount,
                        nonce=chain.state.get_account_nonce(delegator),
                    )
                    signed_tx = SignedTransaction(
                        transaction=tx,
                        signature=b"sim_stake_sig",
                        public_key=b"sim_pub_key"
                    )
                    success, msg, entry = await chain.submit_transaction(signed_tx)
                    if success:
                        print(f"[STAKE TX] {amount} JASPR from {delegator[:16]}... to {validator.name}")
                        
            elif action == 'unstake':
                stake = chain.get_stake(delegator, validator.address)
                if stake >= amount:
                    from jasprchain.execution.transaction import SignedTransaction, Transaction, TransactionType
                    tx = Transaction(
                        tx_type=TransactionType.UNSTAKE,
                        sender=delegator,
                        recipient=validator.address,
                        amount=amount,
                        nonce=chain.state.get_account_nonce(delegator),
                    )
                    signed_tx = SignedTransaction(
                        transaction=tx,
                        signature=b"sim_unstake_sig",
                        public_key=b"sim_pub_key"
                    )
                    success, msg, entry = await chain.submit_transaction(signed_tx)
                    if success:
                        print(f"[UNSTAKE TX] {amount} JASPR from {delegator[:16]}... from {validator.name}")
                        
            else:  # transfer
                recipient = random.choice(simulated_addresses[:20])
                if recipient != delegator:
                    balance = chain.get_balance(delegator)
                    transfer_amount = random.randint(10, 500)
                    if balance >= transfer_amount:
                        from jasprchain.execution.transaction import SignedTransaction, Transaction, TransactionType
                        tx = Transaction(
                            tx_type=TransactionType.TRANSFER,
                            sender=delegator,
                            recipient=recipient,
                            amount=transfer_amount,
                            nonce=chain.state.get_account_nonce(delegator),
                        )
                        signed_tx = SignedTransaction(
                            transaction=tx,
                            signature=b"sim_transfer_sig",
                            public_key=b"sim_pub_key"
                        )
                        success, msg, entry = await chain.submit_transaction(signed_tx)
                        if success:
                            print(f"[TRANSFER TX] {transfer_amount} JASPR: {delegator[:12]}... -> {recipient[:12]}...")
            
        except Exception as e:
            print(f"Staking simulation error: {e}")

@app.on_event("startup")
async def startup_event():
    """Start background tasks"""
    import random
    
    # Initialize additional validators (up to 20)
    additional_validators = [
        ("jaspr1validator5", 35_000, "Alpha Node"),
        ("jaspr1validator6", 32_000, "Beta Node"),
        ("jaspr1validator7", 29_000, "Gamma Node"),
        ("jaspr1validator8", 26_000, "Delta Node"),
        ("jaspr1validator9", 23_000, "Epsilon Node"),
        ("jaspr1validator10", 21_000, "Zeta Node"),
        ("jaspr1validator11", 19_000, "Eta Node"),
        ("jaspr1validator12", 17_000, "Theta Node"),
        ("jaspr1validator13", 15_000, "Iota Node"),
        ("jaspr1validator14", 14_000, "Kappa Node"),
        ("jaspr1validator15", 13_000, "Lambda Node"),
        ("jaspr1validator16", 12_000, "Mu Node"),
        ("jaspr1validator17", 11_000, "Nu Node"),
        ("jaspr1validator18", 10_500, "Xi Node"),
        ("jaspr1validator19", 10_000, "Omicron Node"),
        ("jaspr1validator20", 9_500, "Pi Node"),
    ]
    
    for addr, stake, name in additional_validators:
        if not chain.validator_set.get_validator(addr):
            chain.validator_set.add_validator(addr, stake, name)
            chain.slashing.register_validator(addr, chain.height)
    
    print(f"[STARTUP] Initialized {len(chain.validator_set.validators)} validators")
    
    # Start P2P network with simulated peers
    await chain.p2p.start()
    
    # Add simulated peers (representing other testnet nodes)
    simulated_peers = [
        ("peer_alpha", "10.0.0.1", 30303, True),
        ("peer_beta", "10.0.0.2", 30303, True),
        ("peer_gamma", "10.0.0.3", 30303, False),
        ("peer_delta", "10.0.0.4", 30303, False),
        ("peer_epsilon", "10.0.0.5", 30303, False),
    ]
    
    from jasprchain.network.p2p import Peer
    from datetime import datetime, timezone
    now = int(datetime.now(timezone.utc).timestamp() * 1000)
    
    for peer_id, addr, port, is_validator in simulated_peers:
        peer = Peer(
            peer_id=peer_id,
            address=addr,
            port=port,
            connected_at=now,
            last_seen=now,
            latency_ms=random.randint(10, 100),
            version="0.1.0",
            chain_height=chain.height,
            is_validator=is_validator,
            reputation=random.randint(80, 100),
            is_simulated=True  # Mark as simulated to skip cleanup
        )
        chain.p2p.peers[peer_id] = peer
    
    print(f"[STARTUP] P2P network started with {len(chain.p2p.peers)} simulated peers")
    
    # Start background tasks
    asyncio.create_task(auto_produce_blocks())
    asyncio.create_task(simulate_staking_activity())
    asyncio.create_task(simulate_slashing_detection())
    asyncio.create_task(simulate_move_contract_activity())
    
    print("[STARTUP] Background tasks started: blocks, staking, slashing, contracts")

async def simulate_slashing_detection():
    """Monitor validator signatures and detect misbehavior"""
    import random
    
    while True:
        # Check every 30-60 seconds
        await asyncio.sleep(random.randint(30, 60))
        
        try:
            validators = chain.validator_set.get_active_validators()
            if not validators:
                continue
            
            # Simulate block signing - 95% sign, 5% miss
            for validator in validators:
                if chain.blocks:
                    if random.random() < 0.95:
                        chain.slashing.record_block_signature(
                            validator.address, 
                            chain.height, 
                            chain.blocks[-1].hash
                        )
                    else:
                        chain.slashing.record_missed_block(validator.address, chain.height)
                        print(f"[MISS] {validator.name} missed block {chain.height}")
            
            # Process any slashing evidence
            records = chain.slashing.process_pending_evidence(chain.validator_set)
            for record in records:
                print(f"[SLASH] {record.slash_type.value}: {validator.name} slashed {record.amount_slashed} JASPR")
                await manager.broadcast({
                    "type": "slashing_event",
                    "data": record.to_dict()
                })
                
        except Exception as e:
            print(f"Slashing detection error: {e}")

async def simulate_move_contract_activity():
    """Simulate Move VM contract activity"""
    import random
    
    while True:
        # Every 20-45 seconds
        await asyncio.sleep(random.randint(20, 45))
        
        try:
            sender = f"jaspr1move_{random.randint(1, 100):04d}"
            recipient = f"jaspr1move_{random.randint(1, 100):04d}"
            amount = random.randint(10, 500)
            
            result = chain.move_vm.execute_function(
                sender=sender,
                module_id="0x1::JASPR",
                function_name="transfer",
                type_args=[],
                args=[recipient, amount]
            )
            
            if result.success:
                await manager.broadcast({
                    "type": "move_event",
                    "data": {
                        "activity": "transfer",
                        "from": sender[:16],
                        "to": recipient[:16],
                        "amount": amount,
                        "gas_used": result.gas_used
                    }
                })
                
        except Exception as e:
            pass  # Silently continue

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8001)
