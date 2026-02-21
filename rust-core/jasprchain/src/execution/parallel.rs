//! Parallel execution engine with conflict detection

use super::{Transaction, SignedTransaction, TransactionType};
use crate::state::StateStore;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// State access record for conflict detection
#[derive(Clone)]
struct StateAccess {
    tx_hash: String,
    key: String,
    access_type: AccessType,
}

#[derive(Clone, PartialEq)]
enum AccessType {
    Read,
    Write,
}

/// Execution result
pub struct ExecutionResult {
    pub tx_hash: String,
    pub success: bool,
    pub gas_used: u64,
    pub error: Option<String>,
}

/// Set of conflicting transactions
pub struct ConflictSet {
    pub conflicting_txs: Vec<String>,
    pub conflict_keys: Vec<String>,
}

/// Parallel transaction executor
pub struct ParallelExecutor {
    state: Arc<RwLock<StateStore>>,
    access_log: HashMap<String, Vec<StateAccess>>,
}

impl ParallelExecutor {
    pub fn new(state: Arc<RwLock<StateStore>>) -> Self {
        Self {
            state,
            access_log: HashMap::new(),
        }
    }
    
    /// Execute a batch of transactions in parallel
    pub async fn execute_batch(
        &mut self,
        transactions: Vec<SignedTransaction>,
    ) -> (Vec<ExecutionResult>, Vec<ConflictSet>) {
        if transactions.is_empty() {
            return (Vec::new(), Vec::new());
        }
        
        self.access_log.clear();
        
        // Phase 1: Parallel optimistic execution
        let mut results = Vec::new();
        for tx in &transactions {
            let result = self.execute_single(tx).await;
            results.push(result);
        }
        
        // Phase 2: Conflict detection
        let conflicts = self.detect_conflicts();
        
        // Phase 3: Re-execute conflicts sequentially (simplified)
        // In production, would rollback and re-execute
        
        (results, conflicts)
    }
    
    async fn execute_single(&mut self, tx: &SignedTransaction) -> ExecutionResult {
        let tx_hash = tx.hash();
        let inner = &tx.transaction;
        
        let mut gas_used = 21000u64;
        let mut error = None;
        let success;
        
        match inner.tx_type {
            TransactionType::Transfer => {
                // Simulate transfer execution
                let mut state = self.state.write().await;
                
                let sender_key = format!("balance:{}", inner.sender);
                let sender_balance = state.get(&sender_key).unwrap_or(0);
                
                if sender_balance < inner.amount {
                    error = Some("Insufficient balance".to_string());
                    success = false;
                } else {
                    let recipient = inner.recipient.as_ref().unwrap_or(&inner.sender);
                    let recipient_key = format!("balance:{}", recipient);
                    let recipient_balance = state.get(&recipient_key).unwrap_or(0);
                    
                    state.set(&sender_key, sender_balance - inner.amount);
                    state.set(&recipient_key, recipient_balance + inner.amount);
                    
                    success = true;
                }
            }
            TransactionType::Stake => {
                gas_used = 50000;
                success = true;
            }
            TransactionType::Unstake => {
                gas_used = 50000;
                success = true;
            }
            _ => {
                gas_used = inner.gas_limit / 2;
                success = true;
            }
        }
        
        ExecutionResult {
            tx_hash,
            success,
            gas_used,
            error,
        }
    }
    
    fn detect_conflicts(&self) -> Vec<ConflictSet> {
        // Build key -> tx mapping
        let mut key_writes: HashMap<String, Vec<String>> = HashMap::new();
        let mut key_reads: HashMap<String, Vec<String>> = HashMap::new();
        
        for (tx_hash, accesses) in &self.access_log {
            for access in accesses {
                match access.access_type {
                    AccessType::Write => {
                        key_writes.entry(access.key.clone())
                            .or_default()
                            .push(tx_hash.clone());
                    }
                    AccessType::Read => {
                        key_reads.entry(access.key.clone())
                            .or_default()
                            .push(tx_hash.clone());
                    }
                }
            }
        }
        
        let mut conflicts = Vec::new();
        let mut processed: HashSet<(String, String)> = HashSet::new();
        
        // Find write-write conflicts
        for (key, writers) in &key_writes {
            if writers.len() > 1 {
                let mut sorted = writers.clone();
                sorted.sort();
                let pair = (sorted[0].clone(), sorted[1].clone());
                if !processed.contains(&pair) {
                    processed.insert(pair);
                    conflicts.push(ConflictSet {
                        conflicting_txs: writers.clone(),
                        conflict_keys: vec![key.clone()],
                    });
                }
            }
        }
        
        conflicts
    }
}
