//! VM adapter trait for pluggable VM implementations

use jaspr_types::{
    Address, SignedTransaction, TransactionReceipt, TransactionPayload,
    ExecutionStatus, HashValue, Amount,
};
use async_trait::async_trait;

/// VM execution context
pub struct VmContext {
    /// Chain ID
    pub chain_id: u64,
    /// Block height
    pub block_height: u64,
    /// Block timestamp
    pub timestamp: u64,
    /// Gas limit
    pub gas_limit: u64,
}

/// VM execution output
#[derive(Clone, Debug)]
pub struct VmOutput {
    /// Execution status
    pub status: ExecutionStatus,
    /// Gas used
    pub gas_used: u64,
    /// Return values (for script calls)
    pub return_values: Vec<Vec<u8>>,
    /// Events emitted
    pub events: Vec<(String, Vec<u8>)>,
    /// Balance changes (address, delta)
    pub balance_changes: Vec<(Address, i128)>,
    /// Resource writes
    pub resource_writes: Vec<(Address, String, Vec<u8>)>,
    /// Module deployments
    pub module_deploys: Vec<(Address, String, Vec<u8>)>,
}

impl Default for VmOutput {
    fn default() -> Self {
        Self {
            status: ExecutionStatus::Success,
            gas_used: 0,
            return_values: Vec::new(),
            events: Vec::new(),
            balance_changes: Vec::new(),
            resource_writes: Vec::new(),
            module_deploys: Vec::new(),
        }
    }
}

impl VmOutput {
    /// Create success output
    pub fn success(gas_used: u64) -> Self {
        Self {
            status: ExecutionStatus::Success,
            gas_used,
            ..Default::default()
        }
    }
    
    /// Create failure output
    pub fn failed(error: String, gas_used: u64) -> Self {
        Self {
            status: ExecutionStatus::Failed(error),
            gas_used,
            ..Default::default()
        }
    }
    
    /// Create out of gas output
    pub fn out_of_gas(gas_used: u64) -> Self {
        Self {
            status: ExecutionStatus::OutOfGas,
            gas_used,
            ..Default::default()
        }
    }
    
    /// Add balance change
    pub fn add_balance_change(&mut self, address: Address, delta: i128) {
        self.balance_changes.push((address, delta));
    }
    
    /// Add event
    pub fn add_event(&mut self, type_tag: String, data: Vec<u8>) {
        self.events.push((type_tag, data));
    }
    
    /// Add resource write
    pub fn add_resource_write(&mut self, address: Address, type_tag: String, data: Vec<u8>) {
        self.resource_writes.push((address, type_tag, data));
    }
    
    /// Is success?
    pub fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Success)
    }
}

/// VM adapter trait - implement this for different VM backends
#[async_trait]
pub trait VmAdapter: Send + Sync {
    /// Execute a Move script call
    async fn execute_script(
        &self,
        context: &VmContext,
        sender: Address,
        module_address: Address,
        module_name: &str,
        function_name: &str,
        type_args: &[String],
        args: &[Vec<u8>],
    ) -> VmOutput;
    
    /// Deploy a Move module
    async fn deploy_module(
        &self,
        context: &VmContext,
        sender: Address,
        bytecode: &[u8],
    ) -> VmOutput;
    
    /// Validate module bytecode
    fn validate_module(&self, bytecode: &[u8]) -> Result<(), String>;
    
    /// Get module ABI (if available)
    fn get_module_abi(&self, address: &Address, name: &str) -> Option<String>;
}

/// Null VM adapter for testing (does nothing)
pub struct NullVmAdapter;

#[async_trait]
impl VmAdapter for NullVmAdapter {
    async fn execute_script(
        &self,
        _context: &VmContext,
        _sender: Address,
        _module_address: Address,
        _module_name: &str,
        _function_name: &str,
        _type_args: &[String],
        _args: &[Vec<u8>],
    ) -> VmOutput {
        VmOutput::success(10000)
    }
    
    async fn deploy_module(
        &self,
        _context: &VmContext,
        _sender: Address,
        _bytecode: &[u8],
    ) -> VmOutput {
        VmOutput::success(50000)
    }
    
    fn validate_module(&self, _bytecode: &[u8]) -> Result<(), String> {
        Ok(())
    }
    
    fn get_module_abi(&self, _address: &Address, _name: &str) -> Option<String> {
        None
    }
}
