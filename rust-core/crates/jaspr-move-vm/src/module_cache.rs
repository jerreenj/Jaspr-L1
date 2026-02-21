//! Module bytecode cache

use jaspr_types::Address;
use parking_lot::RwLock;
use std::collections::HashMap;

/// Module identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModuleId {
    pub address: Address,
    pub name: String,
}

impl ModuleId {
    pub fn new(address: Address, name: String) -> Self {
        Self { address, name }
    }
    
    pub fn to_string(&self) -> String {
        format!("{}::{}", self.address.to_hex(), self.name)
    }
}

/// Cached module data
#[derive(Clone, Debug)]
pub struct CachedModule {
    /// Module bytecode
    pub bytecode: Vec<u8>,
    /// Module ABI (JSON)
    pub abi: Option<String>,
    /// Exposed functions
    pub functions: Vec<String>,
    /// Struct definitions
    pub structs: Vec<String>,
}

impl CachedModule {
    pub fn new(bytecode: Vec<u8>) -> Self {
        Self {
            bytecode,
            abi: None,
            functions: Vec::new(),
            structs: Vec::new(),
        }
    }
    
    pub fn with_abi(mut self, abi: String) -> Self {
        self.abi = Some(abi);
        self
    }
}

/// Module cache for storing compiled modules
pub struct ModuleCache {
    /// Cached modules by ID
    modules: RwLock<HashMap<ModuleId, CachedModule>>,
    /// Maximum cache size
    max_size: usize,
}

impl ModuleCache {
    /// Create new module cache
    pub fn new(max_size: usize) -> Self {
        Self {
            modules: RwLock::new(HashMap::new()),
            max_size,
        }
    }
    
    /// Get module from cache
    pub fn get(&self, id: &ModuleId) -> Option<CachedModule> {
        self.modules.read().get(id).cloned()
    }
    
    /// Put module in cache
    pub fn put(&self, id: ModuleId, module: CachedModule) {
        let mut modules = self.modules.write();
        
        // Evict if at capacity (simple LRU would be better)
        if modules.len() >= self.max_size {
            if let Some(key) = modules.keys().next().cloned() {
                modules.remove(&key);
            }
        }
        
        modules.insert(id, module);
    }
    
    /// Check if module exists
    pub fn contains(&self, id: &ModuleId) -> bool {
        self.modules.read().contains_key(id)
    }
    
    /// Remove module from cache
    pub fn remove(&self, id: &ModuleId) -> Option<CachedModule> {
        self.modules.write().remove(id)
    }
    
    /// Get all modules for an address
    pub fn get_modules_for_address(&self, address: &Address) -> Vec<(String, CachedModule)> {
        self.modules.read()
            .iter()
            .filter(|(id, _)| &id.address == address)
            .map(|(id, m)| (id.name.clone(), m.clone()))
            .collect()
    }
    
    /// Clear cache
    pub fn clear(&self) {
        self.modules.write().clear();
    }
    
    /// Get cache size
    pub fn size(&self) -> usize {
        self.modules.read().len()
    }
}

impl Default for ModuleCache {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_module_cache() {
        let cache = ModuleCache::new(10);
        let addr = Address::from_public_key(b"test");
        let id = ModuleId::new(addr, "TestModule".to_string());
        
        // Put module
        cache.put(id.clone(), CachedModule::new(vec![1, 2, 3]));
        
        // Get module
        let module = cache.get(&id).unwrap();
        assert_eq!(module.bytecode, vec![1, 2, 3]);
        
        // Contains
        assert!(cache.contains(&id));
        
        // Remove
        cache.remove(&id);
        assert!(!cache.contains(&id));
    }
    
    #[test]
    fn test_cache_eviction() {
        let cache = ModuleCache::new(2);
        let addr = Address::from_public_key(b"test");
        
        cache.put(
            ModuleId::new(addr, "Module1".to_string()),
            CachedModule::new(vec![1]),
        );
        cache.put(
            ModuleId::new(addr, "Module2".to_string()),
            CachedModule::new(vec![2]),
        );
        cache.put(
            ModuleId::new(addr, "Module3".to_string()),
            CachedModule::new(vec![3]),
        );
        
        // Should have evicted one
        assert_eq!(cache.size(), 2);
    }
}
