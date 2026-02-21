"""Parallel Execution Engine for JasprChain
Mirrors Rust: parallel execution with conflict detection

Key features from spec:
- Optimistic parallel execution with conflict rollback
- 10,000+ TPS on multi-core hardware
- Ideal for high-frequency DEX orderflow
"""
from dataclasses import dataclass, field
from typing import Dict, List, Set, Optional, Any, Tuple
from enum import Enum
from datetime import datetime, timezone
import asyncio
import hashlib
import copy

from .transaction import Transaction, SignedTransaction, TransactionReceipt, TransactionType


class ExecutionStatus(Enum):
    PENDING = "pending"
    EXECUTING = "executing"
    SUCCESS = "success"
    FAILED = "failed"
    CONFLICT = "conflict"
    RETRYING = "retrying"


@dataclass
class StateAccess:
    """Tracks read/write access to state keys
    
    Used for conflict detection in parallel execution
    """
    tx_hash: str
    key: str
    access_type: str  # 'read' or 'write'
    value_before: Optional[Any] = None
    value_after: Optional[Any] = None


@dataclass
class ConflictSet:
    """Set of conflicting transactions
    
    When two transactions access the same state key and at least
    one is a write, they conflict and need sequential re-execution
    """
    conflicting_txs: List[str]  # Transaction hashes
    conflict_keys: List[str]  # State keys that caused conflict
    resolution_order: List[str] = field(default_factory=list)  # Order to re-execute
    
    def to_dict(self) -> dict:
        return {
            'conflicting_txs': self.conflicting_txs,
            'conflict_keys': self.conflict_keys,
            'resolution_order': self.resolution_order
        }


@dataclass
class ExecutionResult:
    """Result of transaction execution"""
    tx_hash: str
    status: ExecutionStatus
    gas_used: int
    state_changes: Dict[str, Tuple[Any, Any]]  # key -> (old, new)
    logs: List[Dict[str, Any]]
    error: Optional[str] = None
    return_value: Optional[Any] = None
    execution_time_us: int = 0  # Microseconds
    
    def to_receipt(self, block_height: int, block_hash: str) -> TransactionReceipt:
        return TransactionReceipt(
            tx_hash=self.tx_hash,
            block_height=block_height,
            block_hash=block_hash,
            status='success' if self.status == ExecutionStatus.SUCCESS else 'failed',
            gas_used=self.gas_used,
            logs=self.logs,
            return_value=self.return_value,
            error_message=self.error,
            state_changes=[{'key': k, 'old': v[0], 'new': v[1]} for k, v in self.state_changes.items()]
        )


class ParallelExecutor:
    """Parallel transaction executor with conflict detection
    
    Implements JasprChain spec:
    - Parallel speculative execution
    - Read/write tracking for conflict detection
    - Localized rollback for conflict sets
    - Deterministic state evolution
    """
    
    def __init__(self, state_store):
        self.state = state_store
        self._access_log: Dict[str, List[StateAccess]] = {}  # tx_hash -> accesses
        self._results: Dict[str, ExecutionResult] = {}
    
    async def execute_batch(
        self, 
        transactions: List[SignedTransaction],
        max_parallel: int = 8
    ) -> Tuple[List[ExecutionResult], List[ConflictSet]]:
        """Execute a batch of transactions in parallel
        
        Algorithm:
        1. First pass: Execute all in parallel optimistically
        2. Detect conflicts from access patterns
        3. Rollback conflicting transactions
        4. Re-execute conflicts sequentially
        5. Return deterministic results
        """
        if not transactions:
            return [], []
        
        # Phase 1: Parallel optimistic execution
        self._access_log.clear()
        
        # Execute in parallel batches
        results = []
        for i in range(0, len(transactions), max_parallel):
            batch = transactions[i:i + max_parallel]
            batch_results = await asyncio.gather(
                *[self._execute_single(tx) for tx in batch]
            )
            results.extend(batch_results)
        
        # Phase 2: Conflict detection
        conflicts = self._detect_conflicts()
        
        if not conflicts:
            return results, []
        
        # Phase 3: Rollback and re-execute conflicts
        for conflict_set in conflicts:
            # Rollback state changes from conflicting txs
            for tx_hash in conflict_set.conflicting_txs:
                await self._rollback_tx(tx_hash)
            
            # Re-execute in deterministic order
            conflict_set.resolution_order = sorted(conflict_set.conflicting_txs)
            for tx_hash in conflict_set.resolution_order:
                tx = next((t for t in transactions if t.hash == tx_hash), None)
                if tx:
                    result = await self._execute_single(tx)
                    # Update result in list
                    for i, r in enumerate(results):
                        if r.tx_hash == tx_hash:
                            results[i] = result
                            break
        
        return results, conflicts
    
    async def _execute_single(self, tx: SignedTransaction) -> ExecutionResult:
        """Execute a single transaction"""
        start_time = datetime.now(timezone.utc)
        tx_hash = tx.hash
        self._access_log[tx_hash] = []
        
        state_changes: Dict[str, Tuple[Any, Any]] = {}
        logs: List[Dict[str, Any]] = []
        gas_used = 21000  # Base gas
        error = None
        return_value = None
        status = ExecutionStatus.SUCCESS
        
        try:
            inner_tx = tx.transaction
            
            if inner_tx.tx_type == TransactionType.TRANSFER:
                result = await self._execute_transfer(tx_hash, inner_tx)
                state_changes.update(result['state_changes'])
                gas_used = result['gas_used']
                logs.extend(result.get('logs', []))
                
            elif inner_tx.tx_type == TransactionType.STAKE:
                result = await self._execute_stake(tx_hash, inner_tx)
                state_changes.update(result['state_changes'])
                gas_used = result['gas_used']
                logs.extend(result.get('logs', []))
                
            elif inner_tx.tx_type in [TransactionType.DEX_ORDER, TransactionType.DEX_SWAP]:
                result = await self._execute_dex_operation(tx_hash, inner_tx)
                state_changes.update(result['state_changes'])
                gas_used = result['gas_used']
                logs.extend(result.get('logs', []))
                return_value = result.get('return_value')
                
            else:
                gas_used = inner_tx.gas_limit // 2  # Consume some gas
                logs.append({'event': 'unsupported_tx_type', 'type': inner_tx.tx_type.value})
                
        except Exception as e:
            status = ExecutionStatus.FAILED
            error = str(e)
            gas_used = inner_tx.gas_limit  # Consume all gas on failure
        
        end_time = datetime.now(timezone.utc)
        execution_time = int((end_time - start_time).total_seconds() * 1_000_000)
        
        result = ExecutionResult(
            tx_hash=tx_hash,
            status=status,
            gas_used=gas_used,
            state_changes=state_changes,
            logs=logs,
            error=error,
            return_value=return_value,
            execution_time_us=execution_time
        )
        
        self._results[tx_hash] = result
        return result
    
    async def _execute_transfer(self, tx_hash: str, tx: Transaction) -> dict:
        """Execute a transfer transaction"""
        sender_key = f"balance:{tx.sender}"
        recipient_key = f"balance:{tx.recipient}"
        
        # Read sender balance
        sender_balance = await self._read_state(tx_hash, sender_key, 0)
        
        if sender_balance < tx.amount:
            raise ValueError("Insufficient balance")
        
        # Read recipient balance
        recipient_balance = await self._read_state(tx_hash, recipient_key, 0)
        
        # Write new balances
        new_sender = sender_balance - tx.amount
        new_recipient = recipient_balance + tx.amount
        
        await self._write_state(tx_hash, sender_key, sender_balance, new_sender)
        await self._write_state(tx_hash, recipient_key, recipient_balance, new_recipient)
        
        return {
            'state_changes': {
                sender_key: (sender_balance, new_sender),
                recipient_key: (recipient_balance, new_recipient)
            },
            'gas_used': 21000,
            'logs': [{
                'event': 'Transfer',
                'from': tx.sender,
                'to': tx.recipient,
                'amount': tx.amount
            }]
        }
    
    async def _execute_stake(self, tx_hash: str, tx: Transaction) -> dict:
        """Execute a staking transaction"""
        sender_key = f"balance:{tx.sender}"
        stake_key = f"stake:{tx.sender}:{tx.recipient}"  # recipient = validator
        
        sender_balance = await self._read_state(tx_hash, sender_key, 0)
        current_stake = await self._read_state(tx_hash, stake_key, 0)
        
        if sender_balance < tx.amount:
            raise ValueError("Insufficient balance for staking")
        
        new_balance = sender_balance - tx.amount
        new_stake = current_stake + tx.amount
        
        await self._write_state(tx_hash, sender_key, sender_balance, new_balance)
        await self._write_state(tx_hash, stake_key, current_stake, new_stake)
        
        return {
            'state_changes': {
                sender_key: (sender_balance, new_balance),
                stake_key: (current_stake, new_stake)
            },
            'gas_used': 50000,
            'logs': [{
                'event': 'Stake',
                'delegator': tx.sender,
                'validator': tx.recipient,
                'amount': tx.amount
            }]
        }
    
    async def _execute_dex_operation(self, tx_hash: str, tx: Transaction) -> dict:
        """Execute DEX order or swap"""
        # Simplified DEX execution
        sender_key = f"balance:{tx.sender}"
        sender_balance = await self._read_state(tx_hash, sender_key, 0)
        
        order_id = hashlib.sha256(f"{tx_hash}:{tx.timestamp}".encode()).hexdigest()[:16]
        order_key = f"order:{order_id}"
        
        await self._write_state(tx_hash, order_key, None, {
            'id': order_id,
            'sender': tx.sender,
            'type': tx.args.get('order_type', 'limit'),
            'side': tx.args.get('side', 'buy'),
            'price': tx.args.get('price', 0),
            'amount': tx.amount,
            'status': 'open'
        })
        
        return {
            'state_changes': {
                order_key: (None, {'id': order_id, 'status': 'open'})
            },
            'gas_used': 65000,
            'logs': [{
                'event': 'OrderPlaced',
                'order_id': order_id,
                'trader': tx.sender,
                'amount': tx.amount
            }],
            'return_value': {'order_id': order_id}
        }
    
    async def _read_state(self, tx_hash: str, key: str, default: Any = None) -> Any:
        """Read state with access logging"""
        value = self.state.get(key, default)
        self._access_log[tx_hash].append(StateAccess(
            tx_hash=tx_hash,
            key=key,
            access_type='read',
            value_before=value
        ))
        return value
    
    async def _write_state(self, tx_hash: str, key: str, old_value: Any, new_value: Any):
        """Write state with access logging"""
        self.state[key] = new_value
        self._access_log[tx_hash].append(StateAccess(
            tx_hash=tx_hash,
            key=key,
            access_type='write',
            value_before=old_value,
            value_after=new_value
        ))
    
    def _detect_conflicts(self) -> List[ConflictSet]:
        """Detect conflicts from access patterns
        
        Conflict occurs when:
        - Two transactions access the same key
        - At least one access is a write
        """
        # Build key -> tx mapping
        key_writes: Dict[str, List[str]] = {}  # key -> list of tx_hashes that wrote
        key_reads: Dict[str, List[str]] = {}   # key -> list of tx_hashes that read
        
        for tx_hash, accesses in self._access_log.items():
            for access in accesses:
                if access.access_type == 'write':
                    if access.key not in key_writes:
                        key_writes[access.key] = []
                    key_writes[access.key].append(tx_hash)
                else:
                    if access.key not in key_reads:
                        key_reads[access.key] = []
                    key_reads[access.key].append(tx_hash)
        
        conflicts: List[ConflictSet] = []
        processed_pairs: Set[tuple] = set()
        
        # Find write-write conflicts
        for key, writers in key_writes.items():
            if len(writers) > 1:
                pair = tuple(sorted(writers))
                if pair not in processed_pairs:
                    processed_pairs.add(pair)
                    conflicts.append(ConflictSet(
                        conflicting_txs=list(writers),
                        conflict_keys=[key]
                    ))
        
        # Find read-write conflicts
        for key, writers in key_writes.items():
            readers = key_reads.get(key, [])
            for writer in writers:
                for reader in readers:
                    if writer != reader:
                        pair = tuple(sorted([writer, reader]))
                        if pair not in processed_pairs:
                            processed_pairs.add(pair)
                            conflicts.append(ConflictSet(
                                conflicting_txs=[writer, reader],
                                conflict_keys=[key]
                            ))
        
        return conflicts
    
    async def _rollback_tx(self, tx_hash: str):
        """Rollback state changes from a transaction"""
        accesses = self._access_log.get(tx_hash, [])
        
        # Rollback in reverse order
        for access in reversed(accesses):
            if access.access_type == 'write':
                self.state[access.key] = access.value_before
