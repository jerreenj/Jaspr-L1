//! Transaction executor

use jaspr_types::{
    Address, SignedTransaction, TransactionReceipt, TransactionPayload,
    ExecutionStatus, HashValue, Amount, Block, BlockHeight, Gas,
    Account, StateChange,
};
use jaspr_state::{AccountStore, StateTree};
use jaspr_crypto::verify_transaction;
use crate::gas::{GasMeter, GasConfig, GasError};
use crate::vm_adapter::{VmAdapter, VmContext, VmOutput, NullVmAdapter};
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, warn, debug, error};

/// Execution configuration
#[derive(Clone, Debug)]
pub struct ExecutorConfig {
    /// Chain ID
    pub chain_id: u64,
    /// Gas configuration
    pub gas_config: GasConfig,
    /// Maximum gas per block
    pub block_gas_limit: Gas,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            chain_id: 1,
            gas_config: GasConfig::default(),
            block_gas_limit: 100_000_000,
        }
    }
}

/// Execution result for a single transaction
#[derive(Clone, Debug)]
pub struct ExecutionResult {
    /// Transaction hash
    pub tx_hash: HashValue,
    /// Execution status
    pub status: ExecutionStatus,
    /// Gas used
    pub gas_used: Gas,
    /// State changes
    pub state_changes: Vec<StateChange>,
    /// Events emitted
    pub events: Vec<(String, Vec<u8>)>,
    /// Return values
    pub return_values: Vec<Vec<u8>>,
}

impl ExecutionResult {
    /// Is success?
    pub fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Success)
    }
    
    /// To receipt
    pub fn to_receipt(&self, block_height: BlockHeight, tx_index: u32) -> TransactionReceipt {
        let mut receipt = if self.is_success() {
            TransactionReceipt::success(self.tx_hash, block_height, tx_index, self.gas_used)
        } else {
            let error = match &self.status {
                ExecutionStatus::Failed(e) => e.clone(),
                ExecutionStatus::OutOfGas => "Out of gas".to_string(),
                ExecutionStatus::MoveAbort { location, code } => {
                    format!("Move abort at {} with code {}", location, code)
                }
                ExecutionStatus::Success => unreachable!(),
            };
            TransactionReceipt::failed(self.tx_hash, block_height, tx_index, self.gas_used, error)
        };
        
        // Add events
        for (type_tag, data) in &self.events {
            receipt.add_event(type_tag.clone(), data.clone());
        }
        
        // Add state changes
        for change in &self.state_changes {
            receipt.add_state_change(change.clone());
        }
        
        receipt.return_values = self.return_values.clone();
        
        receipt
    }
}

/// Transaction executor
pub struct TransactionExecutor {
    config: ExecutorConfig,
    account_store: Arc<AccountStore>,
    state_tree: Arc<StateTree>,
    vm_adapter: Arc<dyn VmAdapter>,
}

impl TransactionExecutor {
    /// Create new executor
    pub fn new(
        config: ExecutorConfig,
        account_store: Arc<AccountStore>,
        state_tree: Arc<StateTree>,
    ) -> Self {
        Self {
            config,
            account_store,
            state_tree,
            vm_adapter: Arc::new(NullVmAdapter),
        }
    }
    
    /// Set VM adapter
    pub fn set_vm_adapter(&mut self, adapter: Arc<dyn VmAdapter>) {
        self.vm_adapter = adapter;
    }
    
    /// Validate transaction (signature, nonce, balance)
    pub fn validate_transaction(&self, tx: &SignedTransaction) -> Result<(), String> {
        // Verify signature
        if !verify_transaction(tx) {
            return Err("Invalid signature".to_string());
        }
        
        // Check chain ID
        if tx.transaction.chain_id != self.config.chain_id {
            return Err(format!(
                "Invalid chain ID: expected {}, got {}",
                self.config.chain_id,
                tx.transaction.chain_id
            ));
        }
        
        // Get sender account
        let sender_account = self.account_store
            .get_or_create(&tx.transaction.sender)
            .map_err(|e| e.to_string())?;
        
        // Check nonce
        if tx.transaction.nonce != sender_account.nonce() {
            return Err(format!(
                "Invalid nonce: expected {}, got {}",
                sender_account.nonce(),
                tx.transaction.nonce
            ));
        }
        
        // Check sender has enough for max gas
        let max_gas_cost = tx.transaction.max_gas * tx.transaction.gas_price;
        if sender_account.balance() < max_gas_cost as u128 {
            return Err("Insufficient balance for gas".to_string());
        }
        
        Ok(())
    }
    
    /// Execute a single transaction
    pub async fn execute_transaction(
        &self,
        tx: &SignedTransaction,
        block_height: BlockHeight,
        timestamp: u64,
    ) -> ExecutionResult {
        let tx_hash = tx.hash();
        
        // Validate first
        if let Err(e) = self.validate_transaction(tx) {
            return ExecutionResult {
                tx_hash,
                status: ExecutionStatus::Failed(e),
                gas_used: 0,
                state_changes: Vec::new(),
                events: Vec::new(),
                return_values: Vec::new(),
            };
        }
        
        // Create gas meter
        let mut gas_meter = GasMeter::new(
            self.config.gas_config.clone(),
            tx.transaction.max_gas,
        );
        
        // Charge base gas
        if let Err(e) = gas_meter.charge_tx_base() {
            return ExecutionResult {
                tx_hash,
                status: ExecutionStatus::OutOfGas,
                gas_used: gas_meter.used(),
                state_changes: Vec::new(),
                events: Vec::new(),
                return_values: Vec::new(),
            };
        }
        
        // Execute based on payload type
        let result = match &tx.transaction.payload {
            TransactionPayload::Transfer { recipient, amount } => {
                self.execute_transfer(
                    &tx.transaction.sender,
                    recipient,
                    *amount,
                    &mut gas_meter,
                ).await
            }
            TransactionPayload::Stake { validator, amount } => {
                self.execute_stake(
                    &tx.transaction.sender,
                    validator,
                    *amount,
                    &mut gas_meter,
                ).await
            }
            TransactionPayload::Unstake { validator, amount } => {
                self.execute_unstake(
                    &tx.transaction.sender,
                    validator,
                    *amount,
                    &mut gas_meter,
                ).await
            }
            TransactionPayload::ModuleDeploy { bytecode, abi: _ } => {
                self.execute_deploy(
                    &tx.transaction.sender,
                    bytecode,
                    &mut gas_meter,
                    block_height,
                    timestamp,
                ).await
            }
            TransactionPayload::ScriptCall {
                module_address,
                module_name,
                function_name,
                type_args,
                args,
            } => {
                self.execute_script_call(
                    &tx.transaction.sender,
                    module_address,
                    module_name,
                    function_name,
                    type_args,
                    args,
                    &mut gas_meter,
                    block_height,
                    timestamp,
                ).await
            }
            TransactionPayload::CreateAccount { new_address, initial_balance } => {
                self.execute_create_account(
                    &tx.transaction.sender,
                    new_address,
                    *initial_balance,
                    &mut gas_meter,
                ).await
            }
        };
        
        // Update sender nonce on success
        if result.is_success() {
            if let Ok(mut sender) = self.account_store.get_or_create(&tx.transaction.sender) {
                sender.state.increment_nonce();
                let _ = self.account_store.put_account(&sender);
            }
        }
        
        ExecutionResult {
            tx_hash,
            status: result.status,
            gas_used: gas_meter.used(),
            state_changes: result.state_changes,
            events: result.events,
            return_values: result.return_values,
        }
    }
    
    /// Execute native transfer
    async fn execute_transfer(
        &self,
        sender: &Address,
        recipient: &Address,
        amount: Amount,
        gas_meter: &mut GasMeter,
    ) -> ExecutionResult {
        if let Err(_) = gas_meter.charge_transfer() {
            return ExecutionResult {
                tx_hash: HashValue::zero(),
                status: ExecutionStatus::OutOfGas,
                gas_used: gas_meter.used(),
                state_changes: Vec::new(),
                events: Vec::new(),
                return_values: Vec::new(),
            };
        }
        
        // Perform transfer
        match self.account_store.transfer(sender, recipient, amount) {
            Ok(()) => {
                let state_changes = vec![
                    StateChange::BalanceChange {
                        address: *sender,
                        old_balance: 0, // Simplified
                        new_balance: 0,
                    },
                    StateChange::BalanceChange {
                        address: *recipient,
                        old_balance: 0,
                        new_balance: 0,
                    },
                ];
                
                ExecutionResult {
                    tx_hash: HashValue::zero(),
                    status: ExecutionStatus::Success,
                    gas_used: gas_meter.used(),
                    state_changes,
                    events: vec![("Transfer".to_string(), Vec::new())],
                    return_values: Vec::new(),
                }
            }
            Err(e) => ExecutionResult {
                tx_hash: HashValue::zero(),
                status: ExecutionStatus::Failed(e.to_string()),
                gas_used: gas_meter.used(),
                state_changes: Vec::new(),
                events: Vec::new(),
                return_values: Vec::new(),
            },
        }
    }
    
    /// Execute stake operation
    async fn execute_stake(
        &self,
        sender: &Address,
        validator: &Address,
        amount: Amount,
        gas_meter: &mut GasMeter,
    ) -> ExecutionResult {
        if let Err(_) = gas_meter.charge_stake() {
            return ExecutionResult {
                tx_hash: HashValue::zero(),
                status: ExecutionStatus::OutOfGas,
                gas_used: gas_meter.used(),
                state_changes: Vec::new(),
                events: Vec::new(),
                return_values: Vec::new(),
            };
        }
        
        // In production, this would interact with the staking module
        // For now, just debit the sender
        let mut sender_account = match self.account_store.get_or_create(sender) {
            Ok(a) => a,
            Err(e) => return ExecutionResult {
                tx_hash: HashValue::zero(),
                status: ExecutionStatus::Failed(e.to_string()),
                gas_used: gas_meter.used(),
                state_changes: Vec::new(),
                events: Vec::new(),
                return_values: Vec::new(),
            },
        };
        
        if let Err(e) = sender_account.debit(amount) {
            return ExecutionResult {
                tx_hash: HashValue::zero(),
                status: ExecutionStatus::Failed(e),
                gas_used: gas_meter.used(),
                state_changes: Vec::new(),
                events: Vec::new(),
                return_values: Vec::new(),
            };
        }
        
        let _ = self.account_store.put_account(&sender_account);
        
        ExecutionResult {
            tx_hash: HashValue::zero(),
            status: ExecutionStatus::Success,
            gas_used: gas_meter.used(),
            state_changes: Vec::new(),
            events: vec![("Stake".to_string(), Vec::new())],
            return_values: Vec::new(),
        }
    }
    
    /// Execute unstake operation
    async fn execute_unstake(
        &self,
        sender: &Address,
        validator: &Address,
        amount: Amount,
        gas_meter: &mut GasMeter,
    ) -> ExecutionResult {
        if let Err(_) = gas_meter.charge_unstake() {
            return ExecutionResult {
                tx_hash: HashValue::zero(),
                status: ExecutionStatus::OutOfGas,
                gas_used: gas_meter.used(),
                state_changes: Vec::new(),
                events: Vec::new(),
                return_values: Vec::new(),
            };
        }
        
        // In production, this would create an unbonding entry
        ExecutionResult {
            tx_hash: HashValue::zero(),
            status: ExecutionStatus::Success,
            gas_used: gas_meter.used(),
            state_changes: Vec::new(),
            events: vec![("Unstake".to_string(), Vec::new())],
            return_values: Vec::new(),
        }
    }
    
    /// Execute module deployment
    async fn execute_deploy(
        &self,
        sender: &Address,
        bytecode: &[u8],
        gas_meter: &mut GasMeter,
        block_height: BlockHeight,
        timestamp: u64,
    ) -> ExecutionResult {
        if let Err(_) = gas_meter.charge_deploy(bytecode.len()) {
            return ExecutionResult {
                tx_hash: HashValue::zero(),
                status: ExecutionStatus::OutOfGas,
                gas_used: gas_meter.used(),
                state_changes: Vec::new(),
                events: Vec::new(),
                return_values: Vec::new(),
            };
        }
        
        let context = VmContext {
            chain_id: self.config.chain_id,
            block_height,
            timestamp,
            gas_limit: gas_meter.remaining(),
        };
        
        let output = self.vm_adapter.deploy_module(&context, *sender, bytecode).await;
        
        ExecutionResult {
            tx_hash: HashValue::zero(),
            status: output.status,
            gas_used: gas_meter.used() + output.gas_used,
            state_changes: Vec::new(),
            events: output.events,
            return_values: output.return_values,
        }
    }
    
    /// Execute Move script call
    async fn execute_script_call(
        &self,
        sender: &Address,
        module_address: &Address,
        module_name: &str,
        function_name: &str,
        type_args: &[String],
        args: &[Vec<u8>],
        gas_meter: &mut GasMeter,
        block_height: BlockHeight,
        timestamp: u64,
    ) -> ExecutionResult {
        if let Err(_) = gas_meter.charge_script_call() {
            return ExecutionResult {
                tx_hash: HashValue::zero(),
                status: ExecutionStatus::OutOfGas,
                gas_used: gas_meter.used(),
                state_changes: Vec::new(),
                events: Vec::new(),
                return_values: Vec::new(),
            };
        }
        
        let context = VmContext {
            chain_id: self.config.chain_id,
            block_height,
            timestamp,
            gas_limit: gas_meter.remaining(),
        };
        
        let output = self.vm_adapter.execute_script(
            &context,
            *sender,
            *module_address,
            module_name,
            function_name,
            type_args,
            args,
        ).await;
        
        // Apply balance changes
        for (address, delta) in &output.balance_changes {
            if let Ok(mut account) = self.account_store.get_or_create(address) {
                if *delta > 0 {
                    account.credit(*delta as u128);
                } else {
                    let _ = account.debit((-*delta) as u128);
                }
                let _ = self.account_store.put_account(&account);
            }
        }
        
        // Apply resource writes
        for (address, type_tag, data) in &output.resource_writes {
            self.state_tree.set_resource(address, type_tag, data.clone());
        }
        
        ExecutionResult {
            tx_hash: HashValue::zero(),
            status: output.status,
            gas_used: gas_meter.used() + output.gas_used,
            state_changes: Vec::new(),
            events: output.events,
            return_values: output.return_values,
        }
    }
    
    /// Execute account creation
    async fn execute_create_account(
        &self,
        sender: &Address,
        new_address: &Address,
        initial_balance: Amount,
        gas_meter: &mut GasMeter,
    ) -> ExecutionResult {
        if let Err(_) = gas_meter.charge_create_account() {
            return ExecutionResult {
                tx_hash: HashValue::zero(),
                status: ExecutionStatus::OutOfGas,
                gas_used: gas_meter.used(),
                state_changes: Vec::new(),
                events: Vec::new(),
                return_values: Vec::new(),
            };
        }
        
        // Check if account already exists
        if let Ok(Some(_)) = self.account_store.get_account(new_address) {
            return ExecutionResult {
                tx_hash: HashValue::zero(),
                status: ExecutionStatus::Failed("Account already exists".to_string()),
                gas_used: gas_meter.used(),
                state_changes: Vec::new(),
                events: Vec::new(),
                return_values: Vec::new(),
            };
        }
        
        // Create new account
        let new_account = Account::new(*new_address, 0);
        if let Err(e) = self.account_store.put_account(&new_account) {
            return ExecutionResult {
                tx_hash: HashValue::zero(),
                status: ExecutionStatus::Failed(e.to_string()),
                gas_used: gas_meter.used(),
                state_changes: Vec::new(),
                events: Vec::new(),
                return_values: Vec::new(),
            };
        }
        
        // Transfer initial balance if specified
        if initial_balance > 0 {
            if let Err(e) = self.account_store.transfer(sender, new_address, initial_balance) {
                return ExecutionResult {
                    tx_hash: HashValue::zero(),
                    status: ExecutionStatus::Failed(e.to_string()),
                    gas_used: gas_meter.used(),
                    state_changes: Vec::new(),
                    events: Vec::new(),
                    return_values: Vec::new(),
                };
            }
        }
        
        ExecutionResult {
            tx_hash: HashValue::zero(),
            status: ExecutionStatus::Success,
            gas_used: gas_meter.used(),
            state_changes: Vec::new(),
            events: vec![("AccountCreated".to_string(), Vec::new())],
            return_values: Vec::new(),
        }
    }
    
    /// Execute all transactions in a block
    pub async fn execute_block(
        &self,
        block: &Block,
    ) -> (Vec<TransactionReceipt>, HashValue, Gas) {
        let mut receipts = Vec::new();
        let mut total_gas = 0u64;
        
        for (idx, tx) in block.body.transactions.iter().enumerate() {
            let result = self.execute_transaction(
                tx,
                block.height(),
                block.header.timestamp,
            ).await;
            
            total_gas += result.gas_used;
            receipts.push(result.to_receipt(block.height(), idx as u32));
        }
        
        // Commit state changes
        let state_root = self.state_tree.commit();
        
        (receipts, state_root, total_gas)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jaspr_types::{Transaction, TransactionPayload};
    use jaspr_state::Database;
    use jaspr_crypto::KeyPair;
    
    async fn create_test_executor() -> TransactionExecutor {
        let db = Database::open_in_memory().unwrap();
        let account_store = Arc::new(AccountStore::new(db));
        let state_tree = Arc::new(StateTree::new());
        
        TransactionExecutor::new(
            ExecutorConfig::default(),
            account_store,
            state_tree,
        )
    }
    
    #[tokio::test]
    async fn test_execute_transfer() {
        let executor = create_test_executor().await;
        
        // Create sender with balance
        let sender_key = KeyPair::generate();
        let sender = Account::new(sender_key.address(), 1_000_000_000_000); // 1000 JASPR
        executor.account_store.put_account(&sender).unwrap();
        
        // Create recipient
        let recipient_key = KeyPair::generate();
        
        // Create and sign transaction
        let tx = Transaction::new(
            sender_key.address(),
            TransactionPayload::Transfer {
                recipient: recipient_key.address(),
                amount: 100_000_000_000, // 100 JASPR
            },
            0,
            100000,
            100,
            1,
        );
        
        let signed = jaspr_crypto::sign_transaction(&sender_key, tx);
        
        // Execute
        let result = executor.execute_transaction(&signed, 1, 1000).await;
        assert!(result.is_success());
        
        // Check balances
        let sender_balance = executor.account_store.get_balance(&sender_key.address()).unwrap();
        let recipient_balance = executor.account_store.get_balance(&recipient_key.address()).unwrap();
        
        assert!(sender_balance < 1_000_000_000_000);
        assert_eq!(recipient_balance, 100_000_000_000);
    }
}
