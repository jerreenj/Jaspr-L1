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
    return {"message": "JasprChain L1 API v0.1.0", "status": "running"}

@api_router.get("/health")
async def health():
    return {"status": "healthy", "chain_id": chain.CHAIN_ID, "height": chain.height}

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

# ------------ Blocks ------------

@api_router.get("/blocks")
async def get_blocks(limit: int = 20, offset: int = 0):
    """Get recent blocks"""
    blocks = chain.blocks[-(limit + offset):][:limit] if offset == 0 else chain.blocks[-(limit + offset):-offset]
    return {
        "blocks": [b.to_summary() for b in reversed(blocks)],
        "total": len(chain.blocks),
        "latest_height": chain.height
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
        "balance_formatted": f"{balance / 1_000_000_000:.4f} JSP",
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
        "balance_formatted": f"{balance / 1_000_000_000:.4f} JSP"
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
        "amount": request.amount
    }

@api_router.get("/staking/{address}")
async def get_staking_info(address: str):
    """Get staking info for an address"""
    stakes = chain.get_all_stakes(address)
    balance = chain.get_balance(address)
    
    return {
        "address": address,
        "balance": balance,
        "stakes": stakes,
        "total_staked": sum(stakes.values()),
        "validators": [
            {
                "address": v_addr,
                "staked": amount,
                "validator_info": chain.validator_set.get_validator(v_addr).to_dict() if chain.validator_set.get_validator(v_addr) else None
            }
            for v_addr, amount in stakes.items()
        ]
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
    """Automatically produce blocks every 2 seconds"""
    while True:
        await asyncio.sleep(2)
        try:
            if chain.mempool.get_stats().total_pending > 0:
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
