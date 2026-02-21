//! Move VM execution engine
//!
//! Provides the core Move VM execution functionality including:
//! - Script execution
//! - Module deployment and verification
//! - Resource management
//! - Gas metering

use jaspr_types::{Address, HashValue, Amount};
use crate::types::{TypeTag, StructTag, MoveValue, type_tag_to_string, parse_type_tag};
use crate::module_cache::{ModuleCache, ModuleId, CachedModule};
use crate::stdlib::StdlibModules;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug, error};

/// Move VM execution context
#[derive(Clone, Debug)]
pub struct ExecutionContext {
    /// Chain ID
    pub chain_id: u64,
    /// Current block height
    pub block_height: u64,
    /// Block timestamp (microseconds)
    pub timestamp: u64,
    /// Maximum gas allowed
    pub max_gas: u64,
    /// Sender address
    pub sender: Address,
    /// Secondary signers (for multi-sig)
    pub secondary_signers: Vec<Address>,
}

/// Move VM execution result
#[derive(Clone, Debug)]
pub struct MoveExecutionResult {
    /// Execution success
    pub success: bool,
    /// Gas used
    pub gas_used: u64,
    /// Return values (serialized)
    pub return_values: Vec<Vec<u8>>,
    /// Events emitted
    pub events: Vec<MoveEvent>,
    /// Resource changes
    pub resource_changes: Vec<ResourceChange>,
    /// Error message (if failed)
    pub error: Option<MoveError>,
}

impl MoveExecutionResult {
    pub fn success(gas_used: u64) -> Self {
        Self {
            success: true,
            gas_used,
            return_values: Vec::new(),
            events: Vec::new(),
            resource_changes: Vec::new(),
            error: None,
        }
    }
    
    pub fn failed(error: MoveError, gas_used: u64) -> Self {
        Self {
            success: false,
            gas_used,
            return_values: Vec::new(),
            events: Vec::new(),
            resource_changes: Vec::new(),
            error: Some(error),
        }
    }
}

/// Move event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoveEvent {
    /// Event key (module::event_name)
    pub key: String,
    /// Sequence number
    pub sequence_number: u64,
    /// Event type tag
    pub type_tag: String,
    /// Event data (BCS serialized)
    pub data: Vec<u8>,
}

/// Resource change
#[derive(Clone, Debug)]
pub struct ResourceChange {
    /// Address where resource lives
    pub address: Address,
    /// Resource type tag
    pub type_tag: StructTag,
    /// Change type
    pub change_type: ResourceChangeType,
    /// New value (if Write)
    pub new_value: Option<Vec<u8>>,
}

/// Resource change type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceChangeType {
    Write,
    Delete,
}

/// Move error
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoveError {
    /// Error category
    pub category: MoveErrorCategory,
    /// Error code
    pub code: u64,
    /// Location (module::function)
    pub location: Option<String>,
    /// Error message
    pub message: String,
}

/// Move error category
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveErrorCategory {
    /// Verification error (module invalid)
    Verification,
    /// Execution aborted
    Abort,
    /// Out of gas
    OutOfGas,
    /// Resource error
    Resource,
    /// Type mismatch
    Type,
    /// Arithmetic error
    Arithmetic,
    /// Internal VM error
    Internal,
}

/// Move function visibility
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionVisibility {
    /// Private (module only)
    Private,
    /// Public
    Public,
    /// Public(friend)
    Friend,
    /// Public entry (can be called directly)
    Entry,
}

/// Move function signature
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub name: String,
    pub visibility: FunctionVisibility,
    pub type_parameters: Vec<String>,
    pub parameters: Vec<TypeTag>,
    pub returns: Vec<TypeTag>,
    pub is_entry: bool,
}

/// Move module metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleMetadata {
    pub address: String,
    pub name: String,
    pub friends: Vec<String>,
    pub structs: Vec<StructMetadata>,
    pub functions: Vec<FunctionSignature>,
}

/// Move struct metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructMetadata {
    pub name: String,
    pub type_parameters: Vec<String>,
    pub abilities: Vec<String>,
    pub fields: Vec<FieldMetadata>,
}

/// Move field metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldMetadata {
    pub name: String,
    pub type_tag: TypeTag,
}

/// Move VM executor
pub struct MoveVmExecutor {
    /// Module cache
    module_cache: Arc<ModuleCache>,
    /// Resource storage
    resources: RwLock<HashMap<(Address, String), Vec<u8>>>,
    /// Event sequence numbers
    event_sequences: RwLock<HashMap<String, u64>>,
    /// Gas schedule
    gas_schedule: GasSchedule,
}

/// Gas schedule for Move operations
#[derive(Clone, Debug)]
pub struct GasSchedule {
    /// Base instruction cost
    pub instruction_base: u64,
    /// Memory read cost per byte
    pub memory_read_per_byte: u64,
    /// Memory write cost per byte
    pub memory_write_per_byte: u64,
    /// Storage read cost per byte
    pub storage_read_per_byte: u64,
    /// Storage write cost per byte
    pub storage_write_per_byte: u64,
    /// Storage delete refund per byte
    pub storage_delete_refund: u64,
    /// Call depth cost
    pub call_per_depth: u64,
    /// Native function base cost
    pub native_base: u64,
}

impl Default for GasSchedule {
    fn default() -> Self {
        Self {
            instruction_base: 1,
            memory_read_per_byte: 1,
            memory_write_per_byte: 2,
            storage_read_per_byte: 100,
            storage_write_per_byte: 500,
            storage_delete_refund: 100,
            call_per_depth: 100,
            native_base: 50,
        }
    }
}

impl MoveVmExecutor {
    /// Create new Move VM executor
    pub fn new() -> Self {
        let module_cache = Arc::new(ModuleCache::new(1000));
        
        // Load standard library
        StdlibModules::load_into_cache(&module_cache);
        
        Self {
            module_cache,
            resources: RwLock::new(HashMap::new()),
            event_sequences: RwLock::new(HashMap::new()),
            gas_schedule: GasSchedule::default(),
        }
    }
    
    /// Execute a Move script
    pub fn execute_script(
        &self,
        context: &ExecutionContext,
        module_address: Address,
        module_name: &str,
        function_name: &str,
        type_args: &[TypeTag],
        args: &[Vec<u8>],
    ) -> MoveExecutionResult {
        let mut gas_used = 0u64;
        
        // Check module exists
        let module_id = ModuleId::new(module_address, module_name.to_string());
        let module = match self.module_cache.get(&module_id) {
            Some(m) => m,
            None => {
                return MoveExecutionResult::failed(
                    MoveError {
                        category: MoveErrorCategory::Resource,
                        code: 0,
                        location: None,
                        message: format!("Module not found: {}", module_id.to_string()),
                    },
                    gas_used,
                );
            }
        };
        
        // Charge base gas
        gas_used += self.gas_schedule.instruction_base * 10;
        
        if gas_used > context.max_gas {
            return MoveExecutionResult::failed(
                MoveError {
                    category: MoveErrorCategory::OutOfGas,
                    code: 0,
                    location: Some(format!("{}::{}", module_name, function_name)),
                    message: "Out of gas".to_string(),
                },
                gas_used,
            );
        }
        
        // Simulate execution based on function name
        let result = self.simulate_function(
            context,
            &module_address,
            module_name,
            function_name,
            type_args,
            args,
            &mut gas_used,
        );
        
        result
    }
    
    /// Simulate function execution
    fn simulate_function(
        &self,
        context: &ExecutionContext,
        module_address: &Address,
        module_name: &str,
        function_name: &str,
        type_args: &[TypeTag],
        args: &[Vec<u8>],
        gas_used: &mut u64,
    ) -> MoveExecutionResult {
        debug!(
            module = %module_name,
            function = %function_name,
            "Simulating Move function"
        );
        
        // Handle standard library functions
        match (module_name, function_name) {
            ("Coin", "transfer") => {
                *gas_used += 21000;
                let mut result = MoveExecutionResult::success(*gas_used);
                result.events.push(MoveEvent {
                    key: "0x1::Coin::TransferEvent".to_string(),
                    sequence_number: self.next_event_sequence("transfer"),
                    type_tag: "0x1::Coin::TransferEvent".to_string(),
                    data: Vec::new(),
                });
                result
            }
            ("Coin", "balance") => {
                *gas_used += 5000;
                let mut result = MoveExecutionResult::success(*gas_used);
                // Return mock balance
                result.return_values.push(1000000u64.to_le_bytes().to_vec());
                result
            }
            ("Coin", "register") => {
                *gas_used += 25000;
                MoveExecutionResult::success(*gas_used)
            }
            ("Staking", "stake") => {
                *gas_used += 50000;
                let mut result = MoveExecutionResult::success(*gas_used);
                result.events.push(MoveEvent {
                    key: "0x1::Staking::StakeEvent".to_string(),
                    sequence_number: self.next_event_sequence("stake"),
                    type_tag: "0x1::Staking::StakeEvent".to_string(),
                    data: Vec::new(),
                });
                result
            }
            ("Staking", "unstake") => {
                *gas_used += 50000;
                let mut result = MoveExecutionResult::success(*gas_used);
                result.events.push(MoveEvent {
                    key: "0x1::Staking::UnstakeEvent".to_string(),
                    sequence_number: self.next_event_sequence("unstake"),
                    type_tag: "0x1::Staking::UnstakeEvent".to_string(),
                    data: Vec::new(),
                });
                result
            }
            ("Staking", "claim_rewards") => {
                *gas_used += 30000;
                MoveExecutionResult::success(*gas_used)
            }
            ("Account", "create_account") => {
                *gas_used += 25000;
                MoveExecutionResult::success(*gas_used)
            }
            ("Account", "exists_at") => {
                *gas_used += 2000;
                let mut result = MoveExecutionResult::success(*gas_used);
                result.return_values.push(vec![1]); // true
                result
            }
            ("Timestamp", "now_microseconds") => {
                *gas_used += 100;
                let mut result = MoveExecutionResult::success(*gas_used);
                result.return_values.push(context.timestamp.to_le_bytes().to_vec());
                result
            }
            ("Timestamp", "now_seconds") => {
                *gas_used += 100;
                let mut result = MoveExecutionResult::success(*gas_used);
                result.return_values.push((context.timestamp / 1_000_000).to_le_bytes().to_vec());
                result
            }
            _ => {
                // Generic function execution
                *gas_used += 10000;
                MoveExecutionResult::success(*gas_used)
            }
        }
    }
    
    /// Deploy a Move module
    pub fn deploy_module(
        &self,
        context: &ExecutionContext,
        bytecode: &[u8],
    ) -> MoveExecutionResult {
        let mut gas_used = 0u64;
        
        // Validate bytecode
        if let Err(e) = self.validate_module(bytecode) {
            return MoveExecutionResult::failed(
                MoveError {
                    category: MoveErrorCategory::Verification,
                    code: 0,
                    location: None,
                    message: e,
                },
                gas_used,
            );
        }
        
        // Charge gas for deployment
        gas_used += self.gas_schedule.storage_write_per_byte * bytecode.len() as u64;
        
        if gas_used > context.max_gas {
            return MoveExecutionResult::failed(
                MoveError {
                    category: MoveErrorCategory::OutOfGas,
                    code: 0,
                    location: None,
                    message: "Out of gas for module deployment".to_string(),
                },
                gas_used,
            );
        }
        
        // Generate module name from bytecode hash
        let module_hash = HashValue::sha256(bytecode);
        let module_name = format!("Module_{}", &module_hash.to_hex()[..8]);
        
        // Store module
        let module_id = ModuleId::new(context.sender, module_name.clone());
        let cached = CachedModule::new(bytecode.to_vec());
        self.module_cache.put(module_id, cached);
        
        info!(
            sender = %context.sender,
            module = %module_name,
            size = bytecode.len(),
            "Module deployed"
        );
        
        let mut result = MoveExecutionResult::success(gas_used);
        result.events.push(MoveEvent {
            key: "0x1::Code::ModulePublished".to_string(),
            sequence_number: self.next_event_sequence("module_published"),
            type_tag: "0x1::Code::ModulePublished".to_string(),
            data: module_name.into_bytes(),
        });
        
        result
    }
    
    /// Validate module bytecode
    pub fn validate_module(&self, bytecode: &[u8]) -> Result<(), String> {
        if bytecode.is_empty() {
            return Err("Empty bytecode".to_string());
        }
        
        if bytecode.len() > 100_000 {
            return Err("Module too large (max 100KB)".to_string());
        }
        
        // In real implementation, would use move-binary-format for validation
        
        Ok(())
    }
    
    /// Read resource from storage
    pub fn read_resource(
        &self,
        address: &Address,
        type_tag: &str,
    ) -> Option<Vec<u8>> {
        let key = (*address, type_tag.to_string());
        self.resources.read().get(&key).cloned()
    }
    
    /// Write resource to storage
    pub fn write_resource(
        &self,
        address: &Address,
        type_tag: &str,
        value: Vec<u8>,
    ) {
        let key = (*address, type_tag.to_string());
        self.resources.write().insert(key, value);
    }
    
    /// Delete resource from storage
    pub fn delete_resource(&self, address: &Address, type_tag: &str) -> Option<Vec<u8>> {
        let key = (*address, type_tag.to_string());
        self.resources.write().remove(&key)
    }
    
    /// Get next event sequence number
    fn next_event_sequence(&self, key: &str) -> u64 {
        let mut sequences = self.event_sequences.write();
        let seq = sequences.entry(key.to_string()).or_insert(0);
        let current = *seq;
        *seq += 1;
        current
    }
    
    /// Get module metadata
    pub fn get_module_metadata(
        &self,
        address: &Address,
        name: &str,
    ) -> Option<ModuleMetadata> {
        let module_id = ModuleId::new(*address, name.to_string());
        let module = self.module_cache.get(&module_id)?;
        
        // Parse ABI if available
        if let Some(abi) = &module.abi {
            if let Ok(metadata) = serde_json::from_str(abi) {
                return Some(metadata);
            }
        }
        
        // Return basic metadata
        Some(ModuleMetadata {
            address: address.to_hex(),
            name: name.to_string(),
            friends: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
        })
    }
    
    /// Get module cache reference
    pub fn module_cache(&self) -> &Arc<ModuleCache> {
        &self.module_cache
    }
}

impl Default for MoveVmExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_context() -> ExecutionContext {
        ExecutionContext {
            chain_id: 1,
            block_height: 100,
            timestamp: 1000000,
            max_gas: 1000000,
            sender: Address::from_public_key(b"test"),
            secondary_signers: Vec::new(),
        }
    }
    
    #[test]
    fn test_execute_coin_transfer() {
        let executor = MoveVmExecutor::new();
        let context = create_test_context();
        
        let result = executor.execute_script(
            &context,
            StdlibModules::framework_address(),
            "Coin",
            "transfer",
            &[],
            &[],
        );
        
        assert!(result.success);
        assert!(result.gas_used > 0);
        assert!(!result.events.is_empty());
    }
    
    #[test]
    fn test_execute_timestamp() {
        let executor = MoveVmExecutor::new();
        let context = create_test_context();
        
        let result = executor.execute_script(
            &context,
            StdlibModules::framework_address(),
            "Timestamp",
            "now_seconds",
            &[],
            &[],
        );
        
        assert!(result.success);
        assert!(!result.return_values.is_empty());
    }
    
    #[test]
    fn test_deploy_module() {
        let executor = MoveVmExecutor::new();
        let context = create_test_context();
        
        let bytecode = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let result = executor.deploy_module(&context, &bytecode);
        
        assert!(result.success);
        assert!(result.gas_used > 0);
    }
    
    #[test]
    fn test_resource_operations() {
        let executor = MoveVmExecutor::new();
        let address = Address::from_public_key(b"test");
        let type_tag = "0x1::Coin::Balance";
        
        // Write
        executor.write_resource(&address, type_tag, vec![1, 2, 3]);
        
        // Read
        let value = executor.read_resource(&address, type_tag);
        assert_eq!(value, Some(vec![1, 2, 3]));
        
        // Delete
        let deleted = executor.delete_resource(&address, type_tag);
        assert_eq!(deleted, Some(vec![1, 2, 3]));
        
        // Read again (should be None)
        assert!(executor.read_resource(&address, type_tag).is_none());
    }
}
