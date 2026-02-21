"""P2P Networking Module for JasprChain
Implements peer-to-peer communication for blockchain nodes

Features:
- Peer discovery and management
- Block propagation
- Transaction broadcasting
- Gossip protocol for state sync
"""
import asyncio
import aiohttp
from aiohttp import web
import json
import hashlib
from dataclasses import dataclass, field
from typing import Dict, List, Set, Optional, Callable, Any
from datetime import datetime, timezone
from enum import Enum
import random


class MessageType(Enum):
    """P2P message types"""
    HANDSHAKE = "handshake"
    HANDSHAKE_ACK = "handshake_ack"
    PING = "ping"
    PONG = "pong"
    NEW_BLOCK = "new_block"
    NEW_TRANSACTION = "new_transaction"
    GET_BLOCKS = "get_blocks"
    BLOCKS = "blocks"
    GET_PEERS = "get_peers"
    PEERS = "peers"
    SYNC_REQUEST = "sync_request"
    SYNC_RESPONSE = "sync_response"


@dataclass
class Peer:
    """Represents a network peer"""
    peer_id: str
    address: str
    port: int
    connected_at: int = 0
    last_seen: int = 0
    latency_ms: float = 0
    version: str = "0.1.0"
    chain_height: int = 0
    is_validator: bool = False
    reputation: int = 100  # 0-100, higher is better
    is_simulated: bool = False  # Simulated peers skip cleanup
    
    @property
    def endpoint(self) -> str:
        return f"http://{self.address}:{self.port}"
    
    def to_dict(self) -> dict:
        return {
            "peer_id": self.peer_id,
            "address": self.address,
            "port": self.port,
            "endpoint": self.endpoint,
            "connected_at": self.connected_at,
            "last_seen": self.last_seen,
            "latency_ms": self.latency_ms,
            "version": self.version,
            "chain_height": self.chain_height,
            "is_validator": self.is_validator,
            "reputation": self.reputation,
            "is_simulated": self.is_simulated
        }


@dataclass
class P2PMessage:
    """P2P message structure"""
    msg_type: MessageType
    sender_id: str
    payload: Dict[str, Any]
    timestamp: int = field(default_factory=lambda: int(datetime.now(timezone.utc).timestamp() * 1000))
    msg_id: str = ""
    
    def __post_init__(self):
        if not self.msg_id:
            data = f"{self.msg_type.value}:{self.sender_id}:{self.timestamp}"
            self.msg_id = hashlib.sha256(data.encode()).hexdigest()[:16]
    
    def to_dict(self) -> dict:
        return {
            "msg_type": self.msg_type.value,
            "sender_id": self.sender_id,
            "payload": self.payload,
            "timestamp": self.timestamp,
            "msg_id": self.msg_id
        }
    
    @classmethod
    def from_dict(cls, data: dict) -> 'P2PMessage':
        return cls(
            msg_type=MessageType(data["msg_type"]),
            sender_id=data["sender_id"],
            payload=data["payload"],
            timestamp=data["timestamp"],
            msg_id=data["msg_id"]
        )


class GossipProtocol:
    """Gossip protocol for efficient message propagation"""
    
    def __init__(self, fanout: int = 6, max_hops: int = 4):
        self.fanout = fanout  # Number of peers to forward to
        self.max_hops = max_hops
        self.seen_messages: Set[str] = set()
        self.message_ttl = 60000  # 60 second TTL
    
    def should_propagate(self, msg_id: str) -> bool:
        """Check if message should be propagated"""
        if msg_id in self.seen_messages:
            return False
        self.seen_messages.add(msg_id)
        return True
    
    def select_peers(self, all_peers: List[Peer], exclude: Set[str]) -> List[Peer]:
        """Select random peers for gossip propagation"""
        available = [p for p in all_peers if p.peer_id not in exclude]
        return random.sample(available, min(self.fanout, len(available)))
    
    def cleanup_old_messages(self, current_time: int):
        """Remove old message IDs to prevent memory bloat"""
        # In production, would track timestamps per message
        if len(self.seen_messages) > 10000:
            # Simple cleanup - remove half
            to_remove = list(self.seen_messages)[:5000]
            for msg_id in to_remove:
                self.seen_messages.discard(msg_id)


class P2PNetwork:
    """P2P networking layer for JasprChain"""
    
    VERSION = "0.1.0"
    DEFAULT_PORT = 30303
    MAX_PEERS = 50
    PING_INTERVAL = 30  # seconds
    PEER_TIMEOUT = 120  # seconds
    
    def __init__(self, node_id: str, host: str = "0.0.0.0", port: int = 30303):
        self.node_id = node_id
        self.host = host
        self.port = port
        
        # Peer management
        self.peers: Dict[str, Peer] = {}
        self.bootstrap_nodes: List[str] = []
        
        # Message handling
        self.gossip = GossipProtocol()
        self.message_handlers: Dict[MessageType, Callable] = {}
        
        # State
        self.is_running = False
        self.chain_height = 0
        self.is_validator = False
        
        # Statistics
        self.stats = {
            "messages_sent": 0,
            "messages_received": 0,
            "blocks_propagated": 0,
            "transactions_propagated": 0,
            "bytes_sent": 0,
            "bytes_received": 0,
            "peer_connections": 0,
            "peer_disconnections": 0
        }
        
        # HTTP client session
        self._session: Optional[aiohttp.ClientSession] = None
        self._server: Optional[web.AppRunner] = None
    
    def register_handler(self, msg_type: MessageType, handler: Callable):
        """Register a message handler"""
        self.message_handlers[msg_type] = handler
    
    async def start(self):
        """Start the P2P network"""
        self.is_running = True
        self._session = aiohttp.ClientSession()
        
        # Start HTTP server for incoming connections
        app = web.Application()
        app.router.add_post('/p2p', self._handle_incoming_message)
        app.router.add_get('/p2p/info', self._handle_info_request)
        
        runner = web.AppRunner(app)
        await runner.setup()
        self._server = runner
        
        site = web.TCPSite(runner, self.host, self.port)
        await site.start()
        
        print(f"[P2P] Node {self.node_id[:8]} started on {self.host}:{self.port}")
        
        # Start background tasks
        asyncio.create_task(self._ping_loop())
        asyncio.create_task(self._peer_cleanup_loop())
    
    async def stop(self):
        """Stop the P2P network"""
        self.is_running = False
        
        if self._session:
            await self._session.close()
        
        if self._server:
            await self._server.cleanup()
        
        print(f"[P2P] Node {self.node_id[:8]} stopped")
    
    async def connect_to_peer(self, address: str, port: int) -> Optional[Peer]:
        """Connect to a peer"""
        try:
            endpoint = f"http://{address}:{port}"
            
            # Send handshake
            handshake = P2PMessage(
                msg_type=MessageType.HANDSHAKE,
                sender_id=self.node_id,
                payload={
                    "version": self.VERSION,
                    "chain_height": self.chain_height,
                    "is_validator": self.is_validator,
                    "listen_port": self.port
                }
            )
            
            start_time = datetime.now(timezone.utc).timestamp() * 1000
            
            async with self._session.post(
                f"{endpoint}/p2p",
                json=handshake.to_dict(),
                timeout=aiohttp.ClientTimeout(total=5)
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    response = P2PMessage.from_dict(data)
                    
                    latency = datetime.now(timezone.utc).timestamp() * 1000 - start_time
                    
                    peer = Peer(
                        peer_id=response.sender_id,
                        address=address,
                        port=port,
                        connected_at=int(datetime.now(timezone.utc).timestamp() * 1000),
                        last_seen=int(datetime.now(timezone.utc).timestamp() * 1000),
                        latency_ms=latency,
                        version=response.payload.get("version", "unknown"),
                        chain_height=response.payload.get("chain_height", 0),
                        is_validator=response.payload.get("is_validator", False)
                    )
                    
                    self.peers[peer.peer_id] = peer
                    self.stats["peer_connections"] += 1
                    
                    print(f"[P2P] Connected to peer {peer.peer_id[:8]} at {endpoint}")
                    return peer
                    
        except Exception as e:
            print(f"[P2P] Failed to connect to {address}:{port}: {e}")
        
        return None
    
    async def disconnect_peer(self, peer_id: str):
        """Disconnect from a peer"""
        if peer_id in self.peers:
            del self.peers[peer_id]
            self.stats["peer_disconnections"] += 1
    
    async def broadcast_block(self, block: dict):
        """Broadcast a new block to all peers"""
        message = P2PMessage(
            msg_type=MessageType.NEW_BLOCK,
            sender_id=self.node_id,
            payload={"block": block}
        )
        
        await self._broadcast(message)
        self.stats["blocks_propagated"] += 1
    
    async def broadcast_transaction(self, transaction: dict):
        """Broadcast a new transaction to all peers"""
        message = P2PMessage(
            msg_type=MessageType.NEW_TRANSACTION,
            sender_id=self.node_id,
            payload={"transaction": transaction}
        )
        
        await self._broadcast(message)
        self.stats["transactions_propagated"] += 1
    
    async def request_blocks(self, peer_id: str, start_height: int, end_height: int) -> List[dict]:
        """Request blocks from a peer"""
        if peer_id not in self.peers:
            return []
        
        peer = self.peers[peer_id]
        message = P2PMessage(
            msg_type=MessageType.GET_BLOCKS,
            sender_id=self.node_id,
            payload={
                "start_height": start_height,
                "end_height": end_height
            }
        )
        
        try:
            async with self._session.post(
                f"{peer.endpoint}/p2p",
                json=message.to_dict(),
                timeout=aiohttp.ClientTimeout(total=30)
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    response = P2PMessage.from_dict(data)
                    return response.payload.get("blocks", [])
        except Exception as e:
            print(f"[P2P] Failed to request blocks from {peer_id[:8]}: {e}")
        
        return []
    
    async def _broadcast(self, message: P2PMessage):
        """Broadcast message using gossip protocol"""
        if not self.gossip.should_propagate(message.msg_id):
            return
        
        peers_to_send = self.gossip.select_peers(
            list(self.peers.values()),
            {message.sender_id}
        )
        
        for peer in peers_to_send:
            asyncio.create_task(self._send_to_peer(peer, message))
    
    async def _send_to_peer(self, peer: Peer, message: P2PMessage):
        """Send message to a specific peer"""
        try:
            msg_bytes = json.dumps(message.to_dict()).encode()
            self.stats["bytes_sent"] += len(msg_bytes)
            
            async with self._session.post(
                f"{peer.endpoint}/p2p",
                json=message.to_dict(),
                timeout=aiohttp.ClientTimeout(total=5)
            ) as resp:
                self.stats["messages_sent"] += 1
                peer.last_seen = int(datetime.now(timezone.utc).timestamp() * 1000)
        except Exception as e:
            # Peer might be offline, reduce reputation
            peer.reputation = max(0, peer.reputation - 5)
            if peer.reputation == 0:
                await self.disconnect_peer(peer.peer_id)
    
    async def _handle_incoming_message(self, request: web.Request) -> web.Response:
        """Handle incoming P2P message"""
        try:
            data = await request.json()
            message = P2PMessage.from_dict(data)
            
            self.stats["messages_received"] += 1
            self.stats["bytes_received"] += len(json.dumps(data).encode())
            
            # Update peer last seen if known
            if message.sender_id in self.peers:
                self.peers[message.sender_id].last_seen = int(datetime.now(timezone.utc).timestamp() * 1000)
            
            # Handle message based on type
            response = await self._process_message(message)
            
            return web.json_response(response.to_dict())
            
        except Exception as e:
            return web.json_response({"error": str(e)}, status=400)
    
    async def _process_message(self, message: P2PMessage) -> P2PMessage:
        """Process incoming message and generate response"""
        
        if message.msg_type == MessageType.HANDSHAKE:
            return P2PMessage(
                msg_type=MessageType.HANDSHAKE_ACK,
                sender_id=self.node_id,
                payload={
                    "version": self.VERSION,
                    "chain_height": self.chain_height,
                    "is_validator": self.is_validator
                }
            )
        
        elif message.msg_type == MessageType.PING:
            return P2PMessage(
                msg_type=MessageType.PONG,
                sender_id=self.node_id,
                payload={"timestamp": int(datetime.now(timezone.utc).timestamp() * 1000)}
            )
        
        elif message.msg_type == MessageType.GET_PEERS:
            peer_list = [p.to_dict() for p in list(self.peers.values())[:10]]
            return P2PMessage(
                msg_type=MessageType.PEERS,
                sender_id=self.node_id,
                payload={"peers": peer_list}
            )
        
        elif message.msg_type == MessageType.NEW_BLOCK:
            # Propagate to other peers
            await self._broadcast(message)
            
            # Call registered handler
            if MessageType.NEW_BLOCK in self.message_handlers:
                await self.message_handlers[MessageType.NEW_BLOCK](message.payload)
            
            return P2PMessage(
                msg_type=MessageType.PONG,
                sender_id=self.node_id,
                payload={"received": True}
            )
        
        elif message.msg_type == MessageType.NEW_TRANSACTION:
            # Propagate to other peers
            await self._broadcast(message)
            
            # Call registered handler
            if MessageType.NEW_TRANSACTION in self.message_handlers:
                await self.message_handlers[MessageType.NEW_TRANSACTION](message.payload)
            
            return P2PMessage(
                msg_type=MessageType.PONG,
                sender_id=self.node_id,
                payload={"received": True}
            )
        
        # Default response
        return P2PMessage(
            msg_type=MessageType.PONG,
            sender_id=self.node_id,
            payload={}
        )
    
    async def _handle_info_request(self, request: web.Request) -> web.Response:
        """Handle node info request"""
        return web.json_response({
            "node_id": self.node_id,
            "version": self.VERSION,
            "chain_height": self.chain_height,
            "peer_count": len(self.peers),
            "is_validator": self.is_validator
        })
    
    async def _ping_loop(self):
        """Periodically ping peers to check connectivity"""
        while self.is_running:
            await asyncio.sleep(self.PING_INTERVAL)
            
            for peer in list(self.peers.values()):
                try:
                    ping = P2PMessage(
                        msg_type=MessageType.PING,
                        sender_id=self.node_id,
                        payload={"timestamp": int(datetime.now(timezone.utc).timestamp() * 1000)}
                    )
                    
                    start = datetime.now(timezone.utc).timestamp() * 1000
                    
                    async with self._session.post(
                        f"{peer.endpoint}/p2p",
                        json=ping.to_dict(),
                        timeout=aiohttp.ClientTimeout(total=5)
                    ) as resp:
                        if resp.status == 200:
                            peer.latency_ms = datetime.now(timezone.utc).timestamp() * 1000 - start
                            peer.last_seen = int(datetime.now(timezone.utc).timestamp() * 1000)
                            peer.reputation = min(100, peer.reputation + 1)
                except:
                    peer.reputation = max(0, peer.reputation - 10)
    
    async def _peer_cleanup_loop(self):
        """Remove inactive peers"""
        while self.is_running:
            await asyncio.sleep(60)  # Check every minute
            
            now = int(datetime.now(timezone.utc).timestamp() * 1000)
            timeout_threshold = self.PEER_TIMEOUT * 1000
            
            for peer_id, peer in list(self.peers.items()):
                if now - peer.last_seen > timeout_threshold:
                    await self.disconnect_peer(peer_id)
                    print(f"[P2P] Disconnected inactive peer {peer_id[:8]}")
    
    def get_peer_list(self) -> List[dict]:
        """Get list of connected peers"""
        return [p.to_dict() for p in self.peers.values()]
    
    def get_stats(self) -> dict:
        """Get P2P statistics"""
        return {
            **self.stats,
            "connected_peers": len(self.peers),
            "is_running": self.is_running
        }
