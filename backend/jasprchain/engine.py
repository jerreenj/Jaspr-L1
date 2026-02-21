"""JasprChain Engine - Main Blockchain Orchestrator
Ties together all modules into a working blockchain

CORE L1 TESTNET - Foundation for applications to be built on top
WITH PERSISTENCE (LMDB) - Blocks survive restart
"""
import asyncio
from typing import Dict, List, Optional, Any
from datetime import datetime, timezone, timedelta
import hashlib

from .consensus import Block, BlockHeader, create_genesis_block, ValidatorSet, ProposerSelection, FinalityEngine
from .execution import ParallelExecutor, SignedTransaction, Transaction, TransactionType
from .state import StateStore
from .wallet import MPCWallet, AccountAbstraction
from .sentinel import AISentinel, GuardMode
from .network import Mempool
from .crypto import generate_wallet, sha256_hex
from .storage import get_persistence


class JasprChain:
    """Main blockchain engine - Core L1 Testnet
    
    Coordinates core modules:
    - Consensus (validators, proposer selection, finality)
    - Execution (parallel transaction processing)
    - State (Sparse Merkle Tree) + LMDB Persistence
    - Wallet (MPC + Account Abstraction)
    - AI Sentinel (risk scoring)
    - Network (mempool)
    
    This is the TESTNET/DEVNET foundation - applications built on top
    
    $JASPR TOKENOMICS (from Litepaper):
    - Total Supply: 1,000,000,000 (Fixed - no inflation)
    - Community Incentives: 52% (520,000,000)
    - Treasury Reserve: 15% (150,000,000)
    - Liquidity & Market Making: 10% (100,000,000)
    - Core Team & Advisors: 10% (100,000,000)
    - Investors (Pre-Seed + Seed): 8% (80,000,000)
    - Ecosystem & Partnerships: 5% (50,000,000)
    """
    
    # Chain configuration
    CHAIN_ID = 1  # Testnet chain ID
    BLOCK_TIME_MS = 2000  # 2 second blocks
    TARGET_TPS = 10000
    
    # Unbonding period (14 days in milliseconds)
    UNBONDING_PERIOD_MS = 14 * 24 * 60 * 60 * 1000  # 14 days
    UNBONDING_PERIOD_DAYS = 14
    
    # $JASPR Tokenomics (from Litepaper)
    TOKEN_SYMBOL = "JASPR"
    TOTAL_SUPPLY = 1_000_000_000_000_000_000  # 1 billion with 9 decimals
    DECIMALS = 9
    
    # Distribution (in base units)
    COMMUNITY_INCENTIVES = 520_000_000_000_000_000  # 52%
    TREASURY_RESERVE = 150_000_000_000_000_000      # 15%
    LIQUIDITY_MARKET_MAKING = 100_000_000_000_000_000  # 10%
    TEAM_ADVISORS = 100_000_000_000_000_000         # 10%
    INVESTORS = 80_000_000_000_000_000              # 8%
    ECOSYSTEM_PARTNERSHIPS = 50_000_000_000_000_000 # 5%
    
    def __init__(self):
        # Persistence layer
        self.persistence = get_persistence()
        
        # Core state
        self.state = StateStore()
        
        # Consensus
        self.validator_set = ValidatorSet()
        self.proposer_selection = ProposerSelection(self.validator_set)
        self.finality_engine = FinalityEngine(self.validator_set)
        
        # Execution
        self.executor = ParallelExecutor(self.state)
        
        # Wallet
        self.account_abstraction = AccountAbstraction()
        self._wallets: Dict[str, MPCWallet] = {}
        
        # AI Sentinel
        self.sentinel = AISentinel(GuardMode.ENFORCED)
        
        # Network
        self.mempool = Mempool(self.sentinel)
        
        # Blockchain state
        self.blocks: List[Block] = []
        self._blocks_by_hash: Dict[str, Block] = {}
        self._running = False
        
        # Stats
        self._total_transactions = 0
        self._start_time = int(datetime.now(timezone.utc).timestamp() * 1000)
        
        # Initialize (with persistence loading)
        self._initialize()
    
    def _initialize(self):
        """Initialize the blockchain - load from persistence or create fresh"""
        # Try to load existing chain from persistence
        latest_height = self.persistence.get_latest_height()
        
        if latest_height >= 0:
            # Load existing chain from persistence
            print(f"[CHAIN] Loading chain from persistence (height: {latest_height})")
            self._load_from_persistence()
        else:
            # Fresh start - create genesis and initialize
            print("[CHAIN] Fresh start - initializing new chain")
            self._initialize_fresh()
    
    def _load_from_persistence(self):
        """Load blockchain state from persistent storage"""
        # Load all blocks
        block_dicts = self.persistence.get_all_blocks()
        for block_dict in block_dicts:
            block = Block.from_dict(block_dict)
            self.blocks.append(block)
            self._blocks_by_hash[block.hash] = block
        
        # Load state (balances, stakes, etc.)
        state_keys = self.persistence.get_all_state_keys()
        for key in state_keys:
            value = self.persistence.get_state(key)
            self.state.set(key, value)
        
        # Load transaction count
        self._total_transactions = self.persistence.get_metadata('total_transactions', 0)
        
        # Initialize validators (they are rebuilt on load)
        validators_config = [
            ("jaspr1validator1", 10_000_000_000_000, "Jaspr Labs"),
            ("jaspr1validator2", 8_000_000_000_000, "Foundation"),
            ("jaspr1validator3", 6_000_000_000_000, "Community"),
            ("jaspr1validator4", 4_000_000_000_000, "Ecosystem"),
        ]
        for addr, stake, name in validators_config:
            self.validator_set.add_validator(addr, stake, name)
        
        # Load validator stakes from state
        for v_addr in self.validator_set.validators:
            validator = self.validator_set.get_validator(v_addr)
            if validator:
                # Get delegated stake from state
                delegated = self.persistence.get_state(f"validator_delegated:{v_addr}", 0)
                validator.stake = validator.stake + delegated
        
        print(f"[CHAIN] Loaded {len(self.blocks)} blocks, {self._total_transactions} transactions")
    
    def _initialize_fresh(self):
        """Initialize a fresh blockchain"""
        # Create genesis block
        genesis = create_genesis_block()
        self.blocks.append(genesis)
        self._blocks_by_hash[genesis.hash] = genesis
        
        # Save genesis to persistence
        self.persistence.save_block(0, genesis.hash, genesis.to_dict())
        self.persistence.set_latest_height(0)
        
        # Initialize treasury accounts (Testnet - from Litepaper tokenomics)
        treasury_accounts = {
            "jaspr1treasury_community": self.COMMUNITY_INCENTIVES,      # 52% - Community
            "jaspr1treasury_reserve": self.TREASURY_RESERVE,            # 15% - Treasury
            "jaspr1treasury_liquidity": self.LIQUIDITY_MARKET_MAKING,   # 10% - Liquidity
            "jaspr1treasury_team": self.TEAM_ADVISORS,                  # 10% - Team
            "jaspr1treasury_investors": self.INVESTORS,                  # 8% - Investors
            "jaspr1treasury_ecosystem": self.ECOSYSTEM_PARTNERSHIPS,    # 5% - Ecosystem
        }
        
        for addr, balance in treasury_accounts.items():
            self.state.set_account_balance(addr, balance)
            self.persistence.save_state(f"balance:{addr}", balance)
        
        # Initialize default validators (HyperLiquid style - start with 4)
        validators_config = [
            ("jaspr1validator1", 10_000_000_000_000, "Jaspr Labs"),
            ("jaspr1validator2", 8_000_000_000_000, "Foundation"),
            ("jaspr1validator3", 6_000_000_000_000, "Community"),
            ("jaspr1validator4", 4_000_000_000_000, "Ecosystem"),
        ]
        
        for addr, stake, name in validators_config:
            self.validator_set.add_validator(addr, stake, name)
    
    @property
    def latest_block(self) -> Block:
        return self.blocks[-1]
    
    @property
    def height(self) -> int:
        return len(self.blocks) - 1
    
    def create_wallet(self) -> MPCWallet:
        """Create a new MPC wallet with testnet tokens"""
        wallet = MPCWallet()
        self._wallets[wallet.address] = wallet
        
        # Testnet: Allocate tokens from community incentives pool
        # Each new wallet gets 10,000 JASPR for testing
        testnet_allocation = 10_000_000_000_000  # 10,000 JASPR
        community_balance = self.state.get_account_balance("jaspr1treasury_community")
        
        if community_balance >= testnet_allocation:
            # Deduct from community pool
            new_community_balance = community_balance - testnet_allocation
            self.state.set_account_balance("jaspr1treasury_community", new_community_balance)
            self.persistence.save_state("balance:jaspr1treasury_community", new_community_balance)
            
            # Credit to new wallet
            self.state.set_account_balance(wallet.address, testnet_allocation)
            self.persistence.save_state(f"balance:{wallet.address}", testnet_allocation)
        else:
            # Fallback for testing if pool depleted
            self.state.set_account_balance(wallet.address, testnet_allocation)
            self.persistence.save_state(f"balance:{wallet.address}", testnet_allocation)
        
        # Save wallet info to persistence
        self.persistence.save_wallet(wallet.address, wallet.export_public_info())
        
        # Create AA wallet
        self.account_abstraction.get_or_create_wallet(wallet.address)
        
        return wallet
    
    def get_wallet(self, address: str) -> Optional[MPCWallet]:
        return self._wallets.get(address)
    
    def get_balance(self, address: str) -> int:
        return self.state.get_account_balance(address)
    
    def create_transaction(
        self,
        sender: str,
        recipient: str,
        amount: int,
        tx_type: TransactionType = TransactionType.TRANSFER
    ) -> Transaction:
        """Create a new transaction"""
        nonce = self.state.get_account_nonce(sender)
        
        return Transaction(
            tx_type=tx_type,
            sender=sender,
            recipient=recipient,
            amount=amount,
            nonce=nonce,
            chain_id=self.CHAIN_ID
        )
    
    def sign_transaction(self, tx: Transaction, wallet: MPCWallet) -> SignedTransaction:
        """Sign a transaction with MPC wallet"""
        message = tx.to_bytes()
        signature = wallet.sign_direct(message)
        
        return SignedTransaction(
            transaction=tx,
            signature=signature,
            public_key=wallet._underlying_keypair.public_key
        )
    
    async def submit_transaction(self, signed_tx: SignedTransaction) -> tuple:
        """Submit a transaction to the mempool"""
        sender = signed_tx.transaction.sender
        balance = self.get_balance(sender)
        
        # Validate AA rules
        valid, message = self.account_abstraction.validate_user_operation(
            sender,
            signed_tx.transaction.recipient or "",
            signed_tx.transaction.amount
        )
        
        if not valid and message != "2FA_REQUIRED":
            return False, message, None
        
        # Add to mempool (includes AI Sentinel scan)
        success, msg, entry = self.mempool.add_transaction(
            signed_tx, 
            balance,
            {'age_days': 30, 'tx_count': 10, 'avg_amount': 1000000}  # Mock history
        )
        
        return success, msg, entry
    
    async def produce_block(self) -> Block:
        """Produce a new block"""
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        
        # Select proposer
        proposer = self.proposer_selection.select_proposer(
            self.height + 1,
            self.latest_block.hash
        )
        
        if not proposer:
            raise ValueError("No active validators")
        
        # Get transactions from mempool
        pending_txs = self.mempool.get_transactions_for_block(max_count=100)
        
        # Execute transactions in parallel
        results, conflicts = await self.executor.execute_batch(pending_txs)
        
        # Commit state
        state_root = self.state.commit()
        
        # Calculate transaction root
        tx_hashes = [tx.hash.encode() for tx in pending_txs]
        from .crypto.hash import merkle_root
        tx_root = merkle_root(tx_hashes).hex() if tx_hashes else sha256_hex(b'')
        
        # Calculate receipts root
        receipt_hashes = [hashlib.sha256(str(r).encode()).digest() for r in results]
        receipts_root = merkle_root(receipt_hashes).hex() if receipt_hashes else sha256_hex(b'')
        
        # Create block header
        header = BlockHeader(
            height=self.height + 1,
            previous_hash=self.latest_block.hash,
            timestamp=now,
            proposer=proposer.address,
            state_root=state_root,
            transactions_root=tx_root,
            receipts_root=receipts_root
        )
        
        # Create block
        block = Block(
            header=header,
            transactions=[tx.to_dict() for tx in pending_txs],
            receipts=[r.to_receipt(header.height, header.hash()).to_dict() for r in results]
        )
        
        # Record for finality
        self.finality_engine.on_block_proposed(block)
        
        # Simulate validator attestations
        await self._collect_attestations(block)
        
        # Add block to chain
        self.blocks.append(block)
        self._blocks_by_hash[block.hash] = block
        
        # PERSIST BLOCK
        self.persistence.save_block(block.height, block.hash, block.to_dict())
        self.persistence.set_latest_height(block.height)
        
        # Update stats
        self._total_transactions += len(pending_txs)
        self.persistence.save_metadata('total_transactions', self._total_transactions)
        
        # Update validator stats
        proposer.stats.blocks_proposed += 1
        proposer.stats.last_active = now
        
        # Remove processed transactions from mempool
        self.mempool.remove_transactions([tx.hash for tx in pending_txs])
        
        return block
    
    async def _collect_attestations(self, block: Block):
        """Simulate collecting validator attestations for finality"""
        block_hash = block.hash
        
        for validator in self.validator_set.get_active_validators():
            bls_keys = self.validator_set.get_bls_keys(validator.address)
            if bls_keys:
                signature = bls_keys.sign_block(block_hash.encode())
                
                # Add prevote
                self.finality_engine.add_prevote(
                    block_hash,
                    block.height,
                    validator.address,
                    signature
                )
                
                # Add precommit
                attestation = self.finality_engine.add_precommit(
                    block_hash,
                    block.height,
                    validator.address,
                    signature
                )
                
                # Update validator stats
                validator.stats.blocks_attested += 1
        
        # Check finality
        if self.finality_engine.is_finalized(block_hash):
            block.finalized = True
            block.finality_time_ms = self.finality_engine.get_finality_time(block_hash)
    
    # Staking Operations
    def stake(self, delegator: str, validator: str, amount: int) -> tuple:
        """Stake tokens to a validator"""
        balance = self.get_balance(delegator)
        if balance < amount:
            return False, "Insufficient balance"
        
        validator_obj = self.validator_set.get_validator(validator)
        if not validator_obj:
            return False, "Validator not found"
        
        # Deduct from balance
        new_balance = balance - amount
        self.state.set_account_balance(delegator, new_balance)
        self.persistence.save_state(f"balance:{delegator}", new_balance)
        
        # Add to stake
        stake_key = f"stake:{delegator}:{validator}"
        current_stake = self.state.get(stake_key, 0)
        new_stake = current_stake + amount
        self.state.set(stake_key, new_stake)
        self.persistence.save_state(stake_key, new_stake)
        
        # Track total delegated to validator
        delegated_key = f"validator_delegated:{validator}"
        total_delegated = self.persistence.get_state(delegated_key, 0)
        self.persistence.save_state(delegated_key, total_delegated + amount)
        
        # Update validator stake
        validator_obj.stake += amount
        
        return True, f"Staked {amount} to {validator}"
    
    def unstake(self, delegator: str, validator: str, amount: int) -> tuple:
        """Unstake tokens from a validator - starts 14-day unbonding period"""
        stake_key = f"stake:{delegator}:{validator}"
        current_stake = self.state.get(stake_key, 0)
        
        if current_stake < amount:
            return False, "Insufficient stake"
        
        validator_obj = self.validator_set.get_validator(validator)
        if not validator_obj:
            return False, "Validator not found"
        
        # Reduce active stake
        new_stake = current_stake - amount
        self.state.set(stake_key, new_stake)
        self.persistence.save_state(stake_key, new_stake)
        
        # Update validator delegated total
        delegated_key = f"validator_delegated:{validator}"
        total_delegated = self.persistence.get_state(delegated_key, 0)
        self.persistence.save_state(delegated_key, max(0, total_delegated - amount))
        
        # Update validator stake
        validator_obj.stake -= amount
        
        # Create unbonding entry (14-day lock period)
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        unlock_time = now + self.UNBONDING_PERIOD_MS
        
        self.persistence.add_unbonding_entry(
            delegator=delegator,
            validator=validator,
            amount=amount,
            unlock_time=unlock_time
        )
        
        unlock_date = datetime.fromtimestamp(unlock_time / 1000, tz=timezone.utc)
        return True, f"Unstaking {amount} from {validator}. Available after {unlock_date.strftime('%Y-%m-%d %H:%M UTC')} (14 days)"
    
    def claim_unbonded(self, delegator: str) -> tuple:
        """Claim all tokens that have completed unbonding period"""
        claimable = self.persistence.get_claimable_unbonding(delegator)
        
        if not claimable:
            return False, "No tokens available to claim"
        
        total_claimed = 0
        for entry in claimable:
            # Add to balance
            balance = self.get_balance(delegator)
            new_balance = balance + entry['amount']
            self.state.set_account_balance(delegator, new_balance)
            self.persistence.save_state(f"balance:{delegator}", new_balance)
            
            # Remove unbonding entry
            self.persistence.remove_unbonding_entry(
                entry['delegator'],
                entry['validator'],
                entry['unlock_time']
            )
            
            total_claimed += entry['amount']
        
        return True, f"Claimed {total_claimed} JASPR from {len(claimable)} unbonding entries"
    
    def get_unbonding_entries(self, delegator: str) -> List[dict]:
        """Get all unbonding entries for a delegator"""
        entries = self.persistence.get_unbonding_entries(delegator)
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        
        # Add remaining time info
        for entry in entries:
            remaining_ms = max(0, entry['unlock_time'] - now)
            remaining_days = remaining_ms / (24 * 60 * 60 * 1000)
            entry['remaining_days'] = round(remaining_days, 2)
            entry['is_claimable'] = remaining_ms <= 0
        
        return entries
    
    def get_stake(self, delegator: str, validator: str) -> int:
        """Get staked amount"""
        stake_key = f"stake:{delegator}:{validator}"
        return self.state.get(stake_key, 0)
    
    def get_all_stakes(self, delegator: str) -> Dict[str, int]:
        """Get all stakes for a delegator"""
        stakes = {}
        for validator in self.validator_set.validators:
            stake = self.get_stake(delegator, validator)
            if stake > 0:
                stakes[validator] = stake
        return stakes
    
    def calculate_validator_apy(self, validator_address: str) -> float:
        """Calculate dynamic APY for a validator
        
        APY is based on:
        - Base rate: 8% annual
        - Validator performance bonus: up to 4% based on uptime and blocks proposed
        - Stake distribution penalty: reduces APY if validator has >40% of total stake
        - Commission deduction: validator's commission is deducted from rewards
        """
        validator = self.validator_set.get_validator(validator_address)
        if not validator:
            return 0.0
        
        # Base APY rate (8%)
        base_apy = 8.0
        
        # Performance bonus (up to 4% additional)
        uptime_bonus = (validator.stats.uptime_percentage / 100) * 2.0  # Max 2%
        activity_bonus = min(validator.stats.blocks_proposed / 100, 2.0)  # Max 2%
        performance_bonus = uptime_bonus + activity_bonus
        
        # Stake concentration penalty
        total_stake = self.validator_set.total_stake()
        if total_stake > 0:
            stake_share = validator.stake / total_stake
            if stake_share > 0.4:  # Penalize validators with >40% stake
                concentration_penalty = (stake_share - 0.4) * 10  # Up to 6% penalty
            else:
                concentration_penalty = 0
        else:
            concentration_penalty = 0
        
        # Calculate gross APY
        gross_apy = base_apy + performance_bonus - concentration_penalty
        
        # Deduct validator commission
        commission_rate = validator.commission_rate / 10000  # Convert basis points
        net_apy = gross_apy * (1 - commission_rate)
        
        return round(max(net_apy, 0), 2)
    
    # Query methods
    def get_block(self, height_or_hash) -> Optional[Block]:
        """Get block by height or hash"""
        if isinstance(height_or_hash, int):
            if 0 <= height_or_hash < len(self.blocks):
                return self.blocks[height_or_hash]
        else:
            return self._blocks_by_hash.get(height_or_hash)
        return None
    
    def get_transaction(self, tx_hash: str) -> Optional[dict]:
        """Get transaction by hash"""
        # Check mempool first
        pending = self.mempool.get_transaction(tx_hash)
        if pending:
            return {'status': 'pending', 'tx': pending.to_dict()}
        
        # Search in blocks
        for block in reversed(self.blocks):
            for tx in block.transactions:
                if tx.get('hash') == tx_hash:
                    return {'status': 'confirmed', 'tx': tx, 'block_height': block.height}
        
        return None
    
    def get_network_stats(self) -> dict:
        """Get network statistics"""
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        uptime = now - self._start_time
        
        avg_finality = 0
        finalized_blocks = [b for b in self.blocks[1:] if b.finalized and b.finality_time_ms]
        if finalized_blocks:
            avg_finality = sum(b.finality_time_ms for b in finalized_blocks) / len(finalized_blocks)
        
        return {
            'chain_id': self.CHAIN_ID,
            'height': self.height,
            'latest_block_hash': self.latest_block.hash,
            'total_transactions': self._total_transactions,
            'tps': self._total_transactions / (uptime / 1000) if uptime > 0 else 0,
            'target_tps': self.TARGET_TPS,
            'validators': {
                'total': len(self.validator_set.validators),
                'active': len(self.validator_set.get_active_validators()),
                'total_stake': self.validator_set.total_stake()
            },
            'mempool': self.mempool.get_stats().to_dict(),
            'sentinel': self.sentinel.get_stats(),
            'finality': {
                'avg_time_ms': round(avg_finality, 2),
                'target_ms': self.BLOCK_TIME_MS
            },
            'uptime_ms': uptime,
            'network': 'testnet'
        }
    
    def get_tokenomics(self) -> dict:
        """Get $JASPR tokenomics info"""
        # Get current treasury balances
        treasury_balances = {
            'community_incentives': self.state.get_account_balance("jaspr1treasury_community"),
            'treasury_reserve': self.state.get_account_balance("jaspr1treasury_reserve"),
            'liquidity_market_making': self.state.get_account_balance("jaspr1treasury_liquidity"),
            'team_advisors': self.state.get_account_balance("jaspr1treasury_team"),
            'investors': self.state.get_account_balance("jaspr1treasury_investors"),
            'ecosystem_partnerships': self.state.get_account_balance("jaspr1treasury_ecosystem"),
        }
        
        # Calculate circulating supply (total - treasury balances)
        treasury_total = sum(treasury_balances.values())
        circulating = self.TOTAL_SUPPLY - treasury_total
        
        return {
            'token': {
                'symbol': self.TOKEN_SYMBOL,
                'name': 'Jaspr',
                'decimals': self.DECIMALS,
            },
            'supply': {
                'total': self.TOTAL_SUPPLY,
                'total_formatted': f"{self.TOTAL_SUPPLY / 10**self.DECIMALS:,.0f} JASPR",
                'circulating': circulating,
                'circulating_formatted': f"{circulating / 10**self.DECIMALS:,.0f} JASPR",
            },
            'distribution': {
                'community_incentives': {'percentage': 52, 'initial': self.COMMUNITY_INCENTIVES, 'current': treasury_balances['community_incentives']},
                'treasury_reserve': {'percentage': 15, 'initial': self.TREASURY_RESERVE, 'current': treasury_balances['treasury_reserve']},
                'liquidity_market_making': {'percentage': 10, 'initial': self.LIQUIDITY_MARKET_MAKING, 'current': treasury_balances['liquidity_market_making']},
                'team_advisors': {'percentage': 10, 'initial': self.TEAM_ADVISORS, 'current': treasury_balances['team_advisors']},
                'investors': {'percentage': 8, 'initial': self.INVESTORS, 'current': treasury_balances['investors']},
                'ecosystem_partnerships': {'percentage': 5, 'initial': self.ECOSYSTEM_PARTNERSHIPS, 'current': treasury_balances['ecosystem_partnerships']},
            },
            'treasury_addresses': {
                'community': 'jaspr1treasury_community',
                'reserve': 'jaspr1treasury_reserve',
                'liquidity': 'jaspr1treasury_liquidity',
                'team': 'jaspr1treasury_team',
                'investors': 'jaspr1treasury_investors',
                'ecosystem': 'jaspr1treasury_ecosystem',
            },
            'inflation': 'fixed_supply',
            'network': 'testnet'
        }


# Global chain instance
_chain_instance: Optional[JasprChain] = None

def get_chain() -> JasprChain:
    """Get or create the global chain instance"""
    global _chain_instance
    if _chain_instance is None:
        _chain_instance = JasprChain()
    return _chain_instance


def reset_chain():
    """Reset the chain (for testing)"""
    global _chain_instance
    _chain_instance = None
