"""Persistent Storage for JasprChain using LMDB
High-performance key-value storage for blockchain data

Stores:
- Blocks by height and hash
- State (account balances, stakes)
- Unbonding entries
- Chain metadata
"""
import lmdb
import json
import os
from typing import Optional, Dict, Any, List
from datetime import datetime, timezone


class PersistentStore:
    """LMDB-based persistent storage for JasprChain"""
    
    def __init__(self, db_path: str = "/app/data/jasprchain"):
        self.db_path = db_path
        os.makedirs(db_path, exist_ok=True)
        
        # Open LMDB environment with multiple databases
        self.env = lmdb.open(
            db_path,
            map_size=10 * 1024 * 1024 * 1024,  # 10GB max
            max_dbs=10,
            create=True
        )
        
        # Named databases
        self.blocks_db = self.env.open_db(b'blocks')
        self.blocks_by_hash_db = self.env.open_db(b'blocks_by_hash')
        self.state_db = self.env.open_db(b'state')
        self.unbonding_db = self.env.open_db(b'unbonding')
        self.meta_db = self.env.open_db(b'meta')
        self.wallets_db = self.env.open_db(b'wallets')
        
        print(f"[PERSISTENCE] Initialized LMDB at {db_path}")
    
    def _serialize(self, data: Any) -> bytes:
        """Serialize data to bytes"""
        return json.dumps(data, default=str).encode('utf-8')
    
    def _deserialize(self, data: bytes) -> Any:
        """Deserialize bytes to data"""
        return json.loads(data.decode('utf-8'))
    
    # ============= Block Storage =============
    
    def save_block(self, height: int, block_hash: str, block_data: dict):
        """Save a block to persistent storage"""
        with self.env.begin(write=True) as txn:
            # Save by height
            txn.put(
                str(height).encode(),
                self._serialize(block_data),
                db=self.blocks_db
            )
            # Save by hash
            txn.put(
                block_hash.encode(),
                str(height).encode(),
                db=self.blocks_by_hash_db
            )
    
    def get_block_by_height(self, height: int) -> Optional[dict]:
        """Get block by height"""
        with self.env.begin() as txn:
            data = txn.get(str(height).encode(), db=self.blocks_db)
            if data:
                return self._deserialize(data)
        return None
    
    def get_block_by_hash(self, block_hash: str) -> Optional[dict]:
        """Get block by hash"""
        with self.env.begin() as txn:
            height_data = txn.get(block_hash.encode(), db=self.blocks_by_hash_db)
            if height_data:
                height = int(height_data.decode())
                return self.get_block_by_height(height)
        return None
    
    def get_all_blocks(self) -> List[dict]:
        """Get all blocks in order"""
        blocks = []
        with self.env.begin() as txn:
            cursor = txn.cursor(db=self.blocks_db)
            for key, value in cursor:
                blocks.append((int(key.decode()), self._deserialize(value)))
        # Sort by height
        blocks.sort(key=lambda x: x[0])
        return [b[1] for b in blocks]
    
    def get_latest_height(self) -> int:
        """Get the latest block height"""
        with self.env.begin() as txn:
            data = txn.get(b'latest_height', db=self.meta_db)
            if data:
                return int(data.decode())
        return -1
    
    def set_latest_height(self, height: int):
        """Set the latest block height"""
        with self.env.begin(write=True) as txn:
            txn.put(b'latest_height', str(height).encode(), db=self.meta_db)
    
    # ============= State Storage =============
    
    def save_state(self, key: str, value: Any):
        """Save state value"""
        with self.env.begin(write=True) as txn:
            txn.put(key.encode(), self._serialize(value), db=self.state_db)
    
    def get_state(self, key: str, default: Any = None) -> Any:
        """Get state value"""
        with self.env.begin() as txn:
            data = txn.get(key.encode(), db=self.state_db)
            if data:
                return self._deserialize(data)
        return default
    
    def delete_state(self, key: str):
        """Delete state value"""
        with self.env.begin(write=True) as txn:
            txn.delete(key.encode(), db=self.state_db)
    
    def get_all_state_keys(self, prefix: str = "") -> List[str]:
        """Get all state keys with optional prefix"""
        keys = []
        with self.env.begin() as txn:
            cursor = txn.cursor(db=self.state_db)
            for key, _ in cursor:
                key_str = key.decode()
                if key_str.startswith(prefix):
                    keys.append(key_str)
        return keys
    
    # ============= Unbonding Storage =============
    
    def add_unbonding_entry(self, delegator: str, validator: str, amount: int, unlock_time: int):
        """Add an unbonding entry"""
        entry_id = f"{delegator}:{validator}:{unlock_time}"
        entry = {
            "delegator": delegator,
            "validator": validator,
            "amount": amount,
            "unlock_time": unlock_time,
            "created_at": int(datetime.now(timezone.utc).timestamp() * 1000)
        }
        with self.env.begin(write=True) as txn:
            txn.put(entry_id.encode(), self._serialize(entry), db=self.unbonding_db)
    
    def get_unbonding_entries(self, delegator: str) -> List[dict]:
        """Get all unbonding entries for a delegator"""
        entries = []
        prefix = f"{delegator}:"
        with self.env.begin() as txn:
            cursor = txn.cursor(db=self.unbonding_db)
            for key, value in cursor:
                key_str = key.decode()
                if key_str.startswith(prefix):
                    entries.append(self._deserialize(value))
        return entries
    
    def get_claimable_unbonding(self, delegator: str) -> List[dict]:
        """Get unbonding entries that are ready to claim"""
        now = int(datetime.now(timezone.utc).timestamp() * 1000)
        entries = self.get_unbonding_entries(delegator)
        return [e for e in entries if e["unlock_time"] <= now]
    
    def remove_unbonding_entry(self, delegator: str, validator: str, unlock_time: int):
        """Remove an unbonding entry after claiming"""
        entry_id = f"{delegator}:{validator}:{unlock_time}"
        with self.env.begin(write=True) as txn:
            txn.delete(entry_id.encode(), db=self.unbonding_db)
    
    # ============= Wallet Storage =============
    
    def save_wallet(self, address: str, wallet_data: dict):
        """Save wallet data"""
        with self.env.begin(write=True) as txn:
            txn.put(address.encode(), self._serialize(wallet_data), db=self.wallets_db)
    
    def get_wallet(self, address: str) -> Optional[dict]:
        """Get wallet data"""
        with self.env.begin() as txn:
            data = txn.get(address.encode(), db=self.wallets_db)
            if data:
                return self._deserialize(data)
        return None
    
    def get_all_wallets(self) -> Dict[str, dict]:
        """Get all wallets"""
        wallets = {}
        with self.env.begin() as txn:
            cursor = txn.cursor(db=self.wallets_db)
            for key, value in cursor:
                wallets[key.decode()] = self._deserialize(value)
        return wallets
    
    # ============= Chain Metadata =============
    
    def save_metadata(self, key: str, value: Any):
        """Save chain metadata"""
        with self.env.begin(write=True) as txn:
            txn.put(key.encode(), self._serialize(value), db=self.meta_db)
    
    def get_metadata(self, key: str, default: Any = None) -> Any:
        """Get chain metadata"""
        with self.env.begin() as txn:
            data = txn.get(key.encode(), db=self.meta_db)
            if data:
                return self._deserialize(data)
        return default
    
    # ============= Utility =============
    
    def close(self):
        """Close the database"""
        self.env.close()
    
    def sync(self):
        """Force sync to disk"""
        self.env.sync()
    
    def get_db_stats(self) -> dict:
        """Get database statistics"""
        stat = self.env.stat()
        info = self.env.info()
        return {
            "page_size": stat['psize'],
            "depth": stat['depth'],
            "branch_pages": stat['branch_pages'],
            "leaf_pages": stat['leaf_pages'],
            "overflow_pages": stat['overflow_pages'],
            "entries": stat['entries'],
            "map_size": info['map_size'],
            "last_pgno": info['last_pgno'],
            "last_txnid": info['last_txnid']
        }


# Global persistence instance
_persistence: Optional[PersistentStore] = None

def get_persistence() -> PersistentStore:
    """Get or create the global persistence instance"""
    global _persistence
    if _persistence is None:
        _persistence = PersistentStore()
    return _persistence
