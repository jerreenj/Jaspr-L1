//! Move bytecode verification and analysis
//!
//! Provides bytecode validation, ABI extraction, and security checks

use jaspr_types::{Address, HashValue};
use crate::types::{TypeTag, StructTag, MoveValue};
use crate::module_cache::{ModuleId, CachedModule};
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug};

/// Bytecode verification result
#[derive(Clone, Debug)]
pub struct VerificationResult {
    /// Is bytecode valid
    pub is_valid: bool,
    /// Errors found
    pub errors: Vec<VerificationError>,
    /// Warnings
    pub warnings: Vec<VerificationWarning>,
    /// Module info (if valid)
    pub module_info: Option<ModuleInfo>,
}

impl VerificationResult {
    pub fn success(module_info: ModuleInfo) -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            module_info: Some(module_info),
        }
    }
    
    pub fn failure(errors: Vec<VerificationError>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
            module_info: None,
        }
    }
}

/// Verification error
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationError {
    pub code: u32,
    pub message: String,
    pub location: Option<String>,
}

/// Verification warning
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationWarning {
    pub code: u32,
    pub message: String,
    pub location: Option<String>,
}

/// Module information extracted from bytecode
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Module name
    pub name: String,
    /// Module address
    pub address: String,
    /// Friend modules
    pub friends: Vec<String>,
    /// Struct definitions
    pub structs: Vec<StructInfo>,
    /// Function definitions
    pub functions: Vec<FunctionInfo>,
    /// Constants
    pub constants: Vec<ConstantInfo>,
    /// Bytecode hash
    pub bytecode_hash: String,
    /// Bytecode size
    pub bytecode_size: usize,
}

/// Struct information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructInfo {
    pub name: String,
    pub type_parameters: Vec<TypeParameterInfo>,
    pub abilities: Vec<String>,
    pub fields: Vec<FieldInfo>,
    pub is_native: bool,
}

/// Type parameter information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeParameterInfo {
    pub name: String,
    pub constraints: Vec<String>,
    pub is_phantom: bool,
}

/// Field information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub type_signature: String,
}

/// Function information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub visibility: String,
    pub is_entry: bool,
    pub type_parameters: Vec<TypeParameterInfo>,
    pub parameters: Vec<String>,
    pub returns: Vec<String>,
    pub acquires: Vec<String>,
    pub is_native: bool,
}

/// Constant information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConstantInfo {
    pub name: String,
    pub type_signature: String,
    pub value: String,
}

/// Bytecode verifier
pub struct BytecodeVerifier {
    /// Maximum module size
    max_module_size: usize,
    /// Maximum function count
    max_functions: usize,
    /// Maximum struct count
    max_structs: usize,
    /// Banned opcodes
    banned_opcodes: HashSet<u8>,
    /// Known safe modules
    safe_modules: HashSet<String>,
}

impl BytecodeVerifier {
    /// Create new verifier with default settings
    pub fn new() -> Self {
        Self {
            max_module_size: 100_000,
            max_functions: 1000,
            max_structs: 500,
            banned_opcodes: HashSet::new(),
            safe_modules: HashSet::new(),
        }
    }
    
    /// Create verifier with custom limits
    pub fn with_limits(
        max_module_size: usize,
        max_functions: usize,
        max_structs: usize,
    ) -> Self {
        Self {
            max_module_size,
            max_functions,
            max_structs,
            banned_opcodes: HashSet::new(),
            safe_modules: HashSet::new(),
        }
    }
    
    /// Verify module bytecode
    pub fn verify(&self, bytecode: &[u8]) -> VerificationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Check size limits
        if bytecode.is_empty() {
            errors.push(VerificationError {
                code: 1001,
                message: "Empty bytecode".to_string(),
                location: None,
            });
            return VerificationResult::failure(errors);
        }
        
        if bytecode.len() > self.max_module_size {
            errors.push(VerificationError {
                code: 1002,
                message: format!(
                    "Module too large: {} bytes (max {})",
                    bytecode.len(),
                    self.max_module_size
                ),
                location: None,
            });
            return VerificationResult::failure(errors);
        }
        
        // Check magic bytes (Move bytecode starts with specific bytes)
        // In real implementation, would parse actual Move binary format
        
        // Check for banned opcodes
        for (offset, byte) in bytecode.iter().enumerate() {
            if self.banned_opcodes.contains(byte) {
                errors.push(VerificationError {
                    code: 2001,
                    message: format!("Banned opcode 0x{:02X} at offset {}", byte, offset),
                    location: Some(format!("offset:{}", offset)),
                });
            }
        }
        
        if !errors.is_empty() {
            return VerificationResult::failure(errors);
        }
        
        // Extract module info (simplified)
        let bytecode_hash = HashValue::sha256(bytecode).to_hex();
        let module_name = format!("Module_{}", &bytecode_hash[..8]);
        
        let module_info = ModuleInfo {
            name: module_name,
            address: "0x0".to_string(),
            friends: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            constants: Vec::new(),
            bytecode_hash,
            bytecode_size: bytecode.len(),
        };
        
        let mut result = VerificationResult::success(module_info);
        result.warnings = warnings;
        result
    }
    
    /// Verify module dependencies
    pub fn verify_dependencies(
        &self,
        bytecode: &[u8],
        available_modules: &HashMap<ModuleId, CachedModule>,
    ) -> Result<Vec<ModuleId>, Vec<VerificationError>> {
        // In real implementation, would:
        // 1. Parse bytecode to extract import dependencies
        // 2. Check each dependency exists in available_modules
        // 3. Return list of dependencies or errors
        
        Ok(Vec::new())
    }
    
    /// Check module for security issues
    pub fn security_check(&self, bytecode: &[u8]) -> Vec<SecurityIssue> {
        let mut issues = Vec::new();
        
        // Check for reentrancy patterns
        // Check for unbounded loops
        // Check for excessive resource usage
        // (Simplified - real implementation would analyze bytecode)
        
        if bytecode.len() > 50_000 {
            issues.push(SecurityIssue {
                severity: Severity::Low,
                category: "size".to_string(),
                message: "Large module may have high gas costs".to_string(),
                recommendation: "Consider splitting into smaller modules".to_string(),
            });
        }
        
        issues
    }
    
    /// Add module to safe list
    pub fn add_safe_module(&mut self, module_id: &str) {
        self.safe_modules.insert(module_id.to_string());
    }
    
    /// Ban an opcode
    pub fn ban_opcode(&mut self, opcode: u8) {
        self.banned_opcodes.insert(opcode);
    }
}

impl Default for BytecodeVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Security issue found during analysis
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityIssue {
    pub severity: Severity,
    pub category: String,
    pub message: String,
    pub recommendation: String,
}

/// Issue severity
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// ABI generator for Move modules
pub struct AbiGenerator;

impl AbiGenerator {
    /// Generate ABI JSON from module info
    pub fn generate(module_info: &ModuleInfo) -> String {
        serde_json::to_string_pretty(module_info).unwrap_or_default()
    }
    
    /// Generate TypeScript bindings
    pub fn generate_typescript(module_info: &ModuleInfo) -> String {
        let mut ts = String::new();
        
        ts.push_str(&format!("// Auto-generated TypeScript bindings for {}\n\n", module_info.name));
        
        // Generate struct types
        for struct_info in &module_info.structs {
            ts.push_str(&format!("export interface {} {{\n", struct_info.name));
            for field in &struct_info.fields {
                ts.push_str(&format!("  {}: {};\n", field.name, Self::move_type_to_ts(&field.type_signature)));
            }
            ts.push_str("}\n\n");
        }
        
        // Generate function signatures
        for func in &module_info.functions {
            if func.visibility == "public" || func.is_entry {
                ts.push_str(&format!(
                    "export async function {}({}): Promise<void> {{\n",
                    func.name,
                    func.parameters.iter()
                        .enumerate()
                        .map(|(i, p)| format!("arg{}: {}", i, Self::move_type_to_ts(p)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                ts.push_str("  // Implementation\n");
                ts.push_str("}\n\n");
            }
        }
        
        ts
    }
    
    /// Convert Move type to TypeScript type
    fn move_type_to_ts(move_type: &str) -> &'static str {
        match move_type {
            "bool" => "boolean",
            "u8" | "u16" | "u32" | "u64" | "u128" | "u256" => "bigint",
            "address" => "string",
            "signer" => "string",
            _ if move_type.starts_with("vector") => "Array<any>",
            _ => "any",
        }
    }
    
    /// Generate Rust bindings
    pub fn generate_rust(module_info: &ModuleInfo) -> String {
        let mut rs = String::new();
        
        rs.push_str(&format!("//! Auto-generated Rust bindings for {}\n\n", module_info.name));
        rs.push_str("use serde::{Deserialize, Serialize};\n\n");
        
        // Generate struct types
        for struct_info in &module_info.structs {
            rs.push_str("#[derive(Clone, Debug, Serialize, Deserialize)]\n");
            rs.push_str(&format!("pub struct {} {{\n", struct_info.name));
            for field in &struct_info.fields {
                rs.push_str(&format!(
                    "    pub {}: {},\n",
                    field.name,
                    Self::move_type_to_rust(&field.type_signature)
                ));
            }
            rs.push_str("}\n\n");
        }
        
        rs
    }
    
    /// Convert Move type to Rust type
    fn move_type_to_rust(move_type: &str) -> &'static str {
        match move_type {
            "bool" => "bool",
            "u8" => "u8",
            "u16" => "u16",
            "u32" => "u32",
            "u64" => "u64",
            "u128" => "u128",
            "u256" => "[u8; 32]",
            "address" => "Address",
            "signer" => "Address",
            _ if move_type.starts_with("vector") => "Vec<u8>",
            _ => "Vec<u8>",
        }
    }
}

/// Module dependency analyzer
pub struct DependencyAnalyzer {
    /// Known modules and their dependencies
    dependencies: HashMap<String, HashSet<String>>,
}

impl DependencyAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }
    
    /// Add module with dependencies
    pub fn add_module(&mut self, module_id: &str, deps: Vec<String>) {
        self.dependencies.insert(
            module_id.to_string(),
            deps.into_iter().collect(),
        );
    }
    
    /// Check for circular dependencies
    pub fn check_circular(&self, module_id: &str) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        
        if self.has_cycle(module_id, &mut visited, &mut path) {
            Some(path)
        } else {
            None
        }
    }
    
    fn has_cycle(
        &self,
        module_id: &str,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        if path.contains(&module_id.to_string()) {
            path.push(module_id.to_string());
            return true;
        }
        
        if visited.contains(module_id) {
            return false;
        }
        
        visited.insert(module_id.to_string());
        path.push(module_id.to_string());
        
        if let Some(deps) = self.dependencies.get(module_id) {
            for dep in deps {
                if self.has_cycle(dep, visited, path) {
                    return true;
                }
            }
        }
        
        path.pop();
        false
    }
    
    /// Get topological order for deployment
    pub fn topological_order(&self, module_id: &str) -> Result<Vec<String>, String> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        
        self.topo_visit(module_id, &mut visited, &mut result)?;
        
        Ok(result)
    }
    
    fn topo_visit(
        &self,
        module_id: &str,
        visited: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<(), String> {
        if visited.contains(module_id) {
            return Ok(());
        }
        
        visited.insert(module_id.to_string());
        
        if let Some(deps) = self.dependencies.get(module_id) {
            for dep in deps {
                self.topo_visit(dep, visited, result)?;
            }
        }
        
        result.push(module_id.to_string());
        Ok(())
    }
}

impl Default for DependencyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_verify_empty() {
        let verifier = BytecodeVerifier::new();
        let result = verifier.verify(&[]);
        assert!(!result.is_valid);
    }
    
    #[test]
    fn test_verify_too_large() {
        let verifier = BytecodeVerifier::with_limits(100, 10, 10);
        let bytecode = vec![0u8; 200];
        let result = verifier.verify(&bytecode);
        assert!(!result.is_valid);
    }
    
    #[test]
    fn test_verify_valid() {
        let verifier = BytecodeVerifier::new();
        let bytecode = vec![0xCA, 0xFE, 0xBA, 0xBE];
        let result = verifier.verify(&bytecode);
        assert!(result.is_valid);
    }
    
    #[test]
    fn test_circular_dependency() {
        let mut analyzer = DependencyAnalyzer::new();
        analyzer.add_module("A", vec!["B".to_string()]);
        analyzer.add_module("B", vec!["C".to_string()]);
        analyzer.add_module("C", vec!["A".to_string()]);
        
        let cycle = analyzer.check_circular("A");
        assert!(cycle.is_some());
    }
    
    #[test]
    fn test_no_circular() {
        let mut analyzer = DependencyAnalyzer::new();
        analyzer.add_module("A", vec!["B".to_string()]);
        analyzer.add_module("B", vec!["C".to_string()]);
        analyzer.add_module("C", vec![]);
        
        let cycle = analyzer.check_circular("A");
        assert!(cycle.is_none());
    }
    
    #[test]
    fn test_topological_order() {
        let mut analyzer = DependencyAnalyzer::new();
        analyzer.add_module("A", vec!["B".to_string(), "C".to_string()]);
        analyzer.add_module("B", vec!["C".to_string()]);
        analyzer.add_module("C", vec![]);
        
        let order = analyzer.topological_order("A").unwrap();
        assert_eq!(order, vec!["C", "B", "A"]);
    }
}
