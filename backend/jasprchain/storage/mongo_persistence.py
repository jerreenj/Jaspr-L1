"""MongoDB Persistence Layer for JasprChain - SURVIVES DEPLOYMENTS"""

import os
from pymongo import MongoClient
from typing import Dict, Any, List, Optional
import json

class MongoPersistentStore:
    """MongoDB-based persistence that survives deployments"""
    
    def __init__(self):
        mongo_url = os.environ.get('MONGO_URL', 'mongodb://localhost:27017')
        self.client = MongoClient(mongo_url)
        self.db = self.client['jasprchain']
        
        # Collections
        self.blocks_col = self.db['blocks']
        self.state_col = self.db['state']
        self.wallets_col = self.db['wallets']
        self.metadata_col = self.db['metadata']
        
        # Create indexes for fast queries
        self.blocks_col.create_index('height', unique=True)
        self.blocks_col.create_index('hash')
        self.wallets_col.create_index('address', unique=True)
        self.state_col.create_index('key', unique=True)
        
        print(f"[MONGO] Connected to MongoDB - Data persists across deployments!")
    
    # Block operations
    def save_block(self, height: int, block_hash: str, block_data: dict):
        """Save block to MongoDB"""
        self.blocks_col.update_one(
            {'height': height},
            {'$set': {'height': height, 'hash': block_hash, 'data': block_data}},
            upsert=True
        )
    
    def get_block(self, height: int) -> Optional[dict]:
        """Get block by height"""
        doc = self.blocks_col.find_one({'height': height})
        return doc['data'] if doc else None
    
    def get_block_by_hash(self, block_hash: str) -> Optional[dict]:
        """Get block by hash"""
        doc = self.blocks_col.find_one({'hash': block_hash})
        return doc['data'] if doc else None
    
    def get_all_blocks(self) -> List[dict]:
        """Get all blocks ordered by height"""
        docs = self.blocks_col.find().sort('height', 1)
        return [doc['data'] for doc in docs]
    
    def get_latest_height(self) -> int:
        """Get the latest block height"""
        doc = self.blocks_col.find_one(sort=[('height', -1)])
        return doc['height'] if doc else -1
    
    def set_latest_height(self, height: int):
        """Update latest height in metadata"""
        self.save_metadata('latest_height', height)
    
    # State operations
    def save_state(self, key: str, value: Any):
        """Save state to MongoDB"""
        self.state_col.update_one(
            {'key': key},
            {'$set': {'key': key, 'value': value}},
            upsert=True
        )
    
    def get_state(self, key: str, default: Any = None) -> Any:
        """Get state from MongoDB"""
        doc = self.state_col.find_one({'key': key})
        return doc['value'] if doc else default
    
    def delete_state(self, key: str):
        """Delete state from MongoDB"""
        self.state_col.delete_one({'key': key})
    
    def get_all_state_keys(self) -> List[str]:
        """Get all state keys"""
        return [doc['key'] for doc in self.state_col.find({}, {'key': 1})]
    
    # Wallet operations
    def save_wallet(self, address: str, wallet_data: dict):
        """Save wallet to MongoDB"""
        self.wallets_col.update_one(
            {'address': address},
            {'$set': {'address': address, 'data': wallet_data}},
            upsert=True
        )
    
    def get_wallet(self, address: str) -> Optional[dict]:
        """Get wallet by address"""
        doc = self.wallets_col.find_one({'address': address})
        return doc['data'] if doc else None
    
    def get_all_wallets(self) -> Dict[str, dict]:
        """Get all wallets"""
        wallets = {}
        for doc in self.wallets_col.find():
            wallets[doc['address']] = doc['data']
        return wallets
    
    # Metadata operations
    def save_metadata(self, key: str, value: Any):
        """Save metadata to MongoDB"""
        self.metadata_col.update_one(
            {'key': key},
            {'$set': {'key': key, 'value': value}},
            upsert=True
        )
    
    def get_metadata(self, key: str, default: Any = None) -> Any:
        """Get metadata from MongoDB"""
        doc = self.metadata_col.find_one({'key': key})
        return doc['value'] if doc else default
    
    # Unbonding operations (for staking)
    def save_unbonding_entry(self, delegator: str, validator: str, amount: int, unlock_time: int):
        """Save unbonding entry"""
        entry_id = f"{delegator}:{validator}:{unlock_time}"
        self.save_state(f"unbonding:{entry_id}", {
            'delegator': delegator,
            'validator': validator,
            'amount': amount,
            'unlock_time': unlock_time
        })
    
    def get_unbonding_entries(self, delegator: str) -> List[dict]:
        """Get unbonding entries for delegator"""
        entries = []
        for doc in self.state_col.find({'key': {'$regex': f'^unbonding:{delegator}:'}}):
            entries.append(doc['value'])
        return entries
    
    def remove_unbonding_entry(self, delegator: str, validator: str, unlock_time: int):
        """Remove unbonding entry"""
        entry_id = f"{delegator}:{validator}:{unlock_time}"
        self.delete_state(f"unbonding:{entry_id}")


# Singleton instance
_mongo_persistence = None

def get_mongo_persistence() -> MongoPersistentStore:
    """Get MongoDB persistence singleton"""
    global _mongo_persistence
    if _mongo_persistence is None:
        _mongo_persistence = MongoPersistentStore()
    return _mongo_persistence
