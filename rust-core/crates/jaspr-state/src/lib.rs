//! State storage for JasprChain
//!
//! Provides:
//! - RocksDB-based persistent storage
//! - Account state management
//! - Block storage
//! - State snapshots and versioning
//! - Sparse Merkle Trie

pub mod db;
pub mod account_store;
pub mod block_store;
pub mod state_tree;
pub mod trie;

pub use db::{Database, DatabaseConfig};
pub use account_store::AccountStore;
pub use block_store::BlockStore;
pub use state_tree::StateTree;
pub use trie::{SparseMerkleTrie, MerkleProof, WorldState, TrieNode};
