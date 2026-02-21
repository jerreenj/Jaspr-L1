//! Move VM adapter implementation

use jaspr_types::{Address, ExecutionStatus};
use jaspr_executor::{VmAdapter, VmContext, VmOutput};
use crate::module_cache::{ModuleCache, ModuleId};
use crate::stdlib::StdlibModules;
use crate::types::{MoveValue, parse_type_tag};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{info, debug, warn};

/// Move VM adapter for JasprChain
pub struct MoveVmAdapter {
    /// Module cache
    module_cache: Arc<ModuleCache>,
    /// Gas cost per instruction (simplified)
    gas_per_instruction: u64,
}

impl MoveVmAdapter {
    /// Create new Move VM adapter
    pub fn new() -> Self {
        let module_cache = Arc::new(ModuleCache::new(1000));
        
        // Load standard library
        StdlibModules::load_into_cache(&module_cache);
        
        Self {
            module_cache,
            gas_per_instruction: 1,
        }
    }
    
    /// Get module cache
    pub fn module_cache(&self) -> &Arc<ModuleCache> {
        &self.module_cache
    }
    
    /// Execute a native function (built-in)
    fn execute_native(
        &self,
        module_name: &str,
        function_name: &str,
        args: &[Vec<u8>],
    ) -> Result<VmOutput, String> {
        match (module_name, function_name) {
            ("Timestamp", "now_microseconds") => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_micros() as u64;
                
                let mut output = VmOutput::success(100);
                output.return_values.push(now.to_le_bytes().to_vec());
                Ok(output)
            }
            ("Timestamp", "now_seconds") => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                let mut output = VmOutput::success(100);
                output.return_values.push(now.to_le_bytes().to_vec());
                Ok(output)
            }
            ("Vector", "empty") => {
                let mut output = VmOutput::success(50);
                output.return_values.push(vec![0, 0, 0, 0]); // Empty vector (length 0)
                Ok(output)
            }
            ("Vector", "length") => {
                // Parse vector from args
                let len = if !args.is_empty() && args[0].len() >= 4 {
                    u32::from_le_bytes(args[0][0..4].try_into().unwrap()) as u64
                } else {
                    0
                };
                
                let mut output = VmOutput::success(50);
                output.return_values.push(len.to_le_bytes().to_vec());
                Ok(output)
            }
            _ => Err(format!("Unknown native function: {}::{}", module_name, function_name)),
        }
    }
    
    /// Simulate Move execution (placeholder for real VM)
    fn simulate_execution(
        &self,
        sender: Address,
        module_address: Address,
        module_name: &str,
        function_name: &str,
        type_args: &[String],
        args: &[Vec<u8>],
        gas_limit: u64,
    ) -> VmOutput {
        // Check if module exists
        let module_id = ModuleId::new(module_address, module_name.to_string());
        
        if !self.module_cache.contains(&module_id) {
            return VmOutput::failed(
                format!("Module not found: {}", module_id.to_string()),
                1000,
            );
        }
        
        // Try native execution first
        if let Ok(output) = self.execute_native(module_name, function_name, args) {
            return output;
        }
        
        // Simulate based on function name
        let gas_used = match function_name {
            "transfer" => {
                let mut output = VmOutput::success(21000);
                output.add_event("TransferEvent".to_string(), Vec::new());
                return output;
            }
            "stake" => {
                let mut output = VmOutput::success(50000);
                output.add_event("StakeEvent".to_string(), Vec::new());
                return output;
            }
            "unstake" => {
                let mut output = VmOutput::success(50000);
                output.add_event("UnstakeEvent".to_string(), Vec::new());
                return output;
            }
            "mint" => {
                let mut output = VmOutput::success(30000);
                output.add_event("MintEvent".to_string(), Vec::new());
                return output;
            }
            "burn" => {
                let mut output = VmOutput::success(30000);
                output.add_event("BurnEvent".to_string(), Vec::new());
                return output;
            }
            "register" => VmOutput::success(25000),
            "balance" => {
                let mut output = VmOutput::success(5000);
                // Return mock balance
                output.return_values.push(1000000u64.to_le_bytes().to_vec());
                return output;
            }
            "get_stake" => {
                let mut output = VmOutput::success(5000);
                output.return_values.push(0u128.to_le_bytes().to_vec());
                return output;
            }
            "get_rewards" => {
                let mut output = VmOutput::success(5000);
                output.return_values.push(0u64.to_le_bytes().to_vec());
                return output;
            }
            _ => VmOutput::success(10000),
        };
        
        gas_used
    }
}

impl Default for MoveVmAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VmAdapter for MoveVmAdapter {
    async fn execute_script(
        &self,
        context: &VmContext,
        sender: Address,
        module_address: Address,
        module_name: &str,
        function_name: &str,
        type_args: &[String],
        args: &[Vec<u8>],
    ) -> VmOutput {
        debug!(
            module = %module_name,
            function = %function_name,
            sender = %sender,
            "Executing Move script"
        );
        
        self.simulate_execution(
            sender,
            module_address,
            module_name,
            function_name,
            type_args,
            args,
            context.gas_limit,
        )
    }
    
    async fn deploy_module(
        &self,
        context: &VmContext,
        sender: Address,
        bytecode: &[u8],
    ) -> VmOutput {
        // Validate module first
        if let Err(e) = self.validate_module(bytecode) {
            return VmOutput::failed(e, 1000);
        }
        
        // Calculate gas cost
        let gas_cost = (bytecode.len() as u64) * 200;
        if gas_cost > context.gas_limit {
            return VmOutput::out_of_gas(context.gas_limit);
        }
        
        // In real implementation, would:
        // 1. Parse module bytecode
        // 2. Extract module name
        // 3. Store in sender's account
        // 4. Add to module cache
        
        // For now, simulate successful deployment
        let module_name = format!("UserModule_{}", hex::encode(&bytecode[..4.min(bytecode.len())]));
        let module_id = ModuleId::new(sender, module_name.clone());
        
        let cached = crate::module_cache::CachedModule::new(bytecode.to_vec());
        self.module_cache.put(module_id, cached);
        
        let mut output = VmOutput::success(gas_cost);
        output.add_event("ModulePublished".to_string(), module_name.clone().into_bytes());
        output.module_deploys.push((sender, module_name.clone(), bytecode.to_vec()));
        
        info!(module = %module_name, sender = %sender, "Module deployed");
        
        output
    }
    
    fn validate_module(&self, bytecode: &[u8]) -> Result<(), String> {
        // Basic validation
        if bytecode.is_empty() {
            return Err("Empty bytecode".to_string());
        }
        
        if bytecode.len() > 100_000 {
            return Err("Module too large (max 100KB)".to_string());
        }
        
        // In real implementation, would use move-binary-format to parse and validate
        
        Ok(())
    }
    
    fn get_module_abi(&self, address: &Address, name: &str) -> Option<String> {
        let module_id = ModuleId::new(*address, name.to_string());
        self.module_cache.get(&module_id)?.abi
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_execute_native_timestamp() {
        let adapter = MoveVmAdapter::new();
        
        let context = VmContext {
            chain_id: 1,
            block_height: 100,
            timestamp: 1000,
            gas_limit: 100000,
        };
        
        let output = adapter.execute_script(
            &context,
            Address::zero(),
            StdlibModules::framework_address(),
            "Timestamp",
            "now_seconds",
            &[],
            &[],
        ).await;
        
        assert!(output.is_success());
        assert!(!output.return_values.is_empty());
    }
    
    #[tokio::test]
    async fn test_deploy_module() {
        let adapter = MoveVmAdapter::new();
        
        let context = VmContext {
            chain_id: 1,
            block_height: 100,
            timestamp: 1000,
            gas_limit: 100000,
        };
        
        let bytecode = vec![0xDE, 0xAD, 0xBE, 0xEF]; // Mock bytecode
        let sender = Address::from_public_key(b"test");
        
        let output = adapter.deploy_module(&context, sender, &bytecode).await;
        
        assert!(output.is_success());
        assert!(!output.module_deploys.is_empty());
    }
    
    #[test]
    fn test_validate_empty_module() {
        let adapter = MoveVmAdapter::new();
        assert!(adapter.validate_module(&[]).is_err());
    }
    
    #[tokio::test]
    async fn test_coin_operations() {
        let adapter = MoveVmAdapter::new();
        
        let context = VmContext {
            chain_id: 1,
            block_height: 100,
            timestamp: 1000,
            gas_limit: 100000,
        };
        
        // Test transfer
        let output = adapter.execute_script(
            &context,
            Address::from_public_key(b"sender"),
            StdlibModules::framework_address(),
            "Coin",
            "transfer",
            &["0x1::JASPR::JASPR".to_string()],
            &[],
        ).await;
        
        assert!(output.is_success());
    }
}
