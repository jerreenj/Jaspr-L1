//! Standard library modules for JasprChain Move

use crate::module_cache::{ModuleCache, ModuleId, CachedModule};
use jaspr_types::Address;

/// Framework address (0x1)
pub const FRAMEWORK_ADDRESS: [u8; 32] = {
    let mut arr = [0u8; 32];
    arr[31] = 1;
    arr
};

/// Standard library module definitions
pub struct StdlibModules;

impl StdlibModules {
    /// Get framework address
    pub fn framework_address() -> Address {
        Address::new(FRAMEWORK_ADDRESS)
    }
    
    /// Load standard library into cache
    pub fn load_into_cache(cache: &ModuleCache) {
        let framework = Self::framework_address();
        
        // Core modules (placeholder bytecode - real implementation would compile Move source)
        let modules = vec![
            ("Vector", Self::vector_module_abi()),
            ("Signer", Self::signer_module_abi()),
            ("Account", Self::account_module_abi()),
            ("Coin", Self::coin_module_abi()),
            ("JASPR", Self::jaspr_module_abi()),
            ("Staking", Self::staking_module_abi()),
            ("Timestamp", Self::timestamp_module_abi()),
            ("Event", Self::event_module_abi()),
        ];
        
        for (name, abi) in modules {
            let id = ModuleId::new(framework, name.to_string());
            let module = CachedModule::new(vec![]) // Placeholder bytecode
                .with_abi(abi);
            cache.put(id, module);
        }
    }
    
    /// Vector module ABI
    fn vector_module_abi() -> String {
        r#"{
            "name": "Vector",
            "functions": [
                {"name": "empty", "visibility": "public", "type_params": ["T"], "params": [], "returns": ["vector<T>"]},
                {"name": "length", "visibility": "public", "type_params": ["T"], "params": ["&vector<T>"], "returns": ["u64"]},
                {"name": "push_back", "visibility": "public", "type_params": ["T"], "params": ["&mut vector<T>", "T"], "returns": []},
                {"name": "pop_back", "visibility": "public", "type_params": ["T"], "params": ["&mut vector<T>"], "returns": ["T"]},
                {"name": "borrow", "visibility": "public", "type_params": ["T"], "params": ["&vector<T>", "u64"], "returns": ["&T"]},
                {"name": "borrow_mut", "visibility": "public", "type_params": ["T"], "params": ["&mut vector<T>", "u64"], "returns": ["&mut T"]}
            ]
        }"#.to_string()
    }
    
    /// Signer module ABI
    fn signer_module_abi() -> String {
        r#"{
            "name": "Signer",
            "functions": [
                {"name": "address_of", "visibility": "public", "type_params": [], "params": ["&signer"], "returns": ["address"]}
            ]
        }"#.to_string()
    }
    
    /// Account module ABI
    fn account_module_abi() -> String {
        r#"{
            "name": "Account",
            "structs": [
                {"name": "Account", "fields": [{"name": "sequence_number", "type": "u64"}]}
            ],
            "functions": [
                {"name": "create_account", "visibility": "public", "type_params": [], "params": ["&signer", "address"], "returns": []},
                {"name": "exists_at", "visibility": "public", "type_params": [], "params": ["address"], "returns": ["bool"]},
                {"name": "get_sequence_number", "visibility": "public", "type_params": [], "params": ["address"], "returns": ["u64"]}
            ]
        }"#.to_string()
    }
    
    /// Coin module ABI
    fn coin_module_abi() -> String {
        r#"{
            "name": "Coin",
            "structs": [
                {"name": "Coin", "type_params": ["CoinType"], "fields": [{"name": "value", "type": "u64"}]},
                {"name": "CoinStore", "type_params": ["CoinType"], "fields": [{"name": "coin", "type": "Coin<CoinType>"}]},
                {"name": "CoinInfo", "type_params": ["CoinType"], "fields": [
                    {"name": "name", "type": "string"},
                    {"name": "symbol", "type": "string"},
                    {"name": "decimals", "type": "u8"},
                    {"name": "supply", "type": "u128"}
                ]}
            ],
            "functions": [
                {"name": "register", "visibility": "public", "type_params": ["CoinType"], "params": ["&signer"], "returns": []},
                {"name": "balance", "visibility": "public", "type_params": ["CoinType"], "params": ["address"], "returns": ["u64"]},
                {"name": "transfer", "visibility": "public", "type_params": ["CoinType"], "params": ["&signer", "address", "u64"], "returns": []},
                {"name": "withdraw", "visibility": "public", "type_params": ["CoinType"], "params": ["&signer", "u64"], "returns": ["Coin<CoinType>"]},
                {"name": "deposit", "visibility": "public", "type_params": ["CoinType"], "params": ["address", "Coin<CoinType>"], "returns": []}
            ]
        }"#.to_string()
    }
    
    /// JASPR native token module ABI
    fn jaspr_module_abi() -> String {
        r#"{
            "name": "JASPR",
            "structs": [
                {"name": "JASPR", "fields": []}
            ],
            "functions": [
                {"name": "initialize", "visibility": "public(friend)", "type_params": [], "params": ["&signer"], "returns": []},
                {"name": "mint", "visibility": "public(friend)", "type_params": [], "params": ["address", "u64"], "returns": []},
                {"name": "burn", "visibility": "public(friend)", "type_params": [], "params": ["&signer", "u64"], "returns": []}
            ]
        }"#.to_string()
    }
    
    /// Staking module ABI
    fn staking_module_abi() -> String {
        r#"{
            "name": "Staking",
            "structs": [
                {"name": "StakePool", "fields": [
                    {"name": "validator", "type": "address"},
                    {"name": "total_stake", "type": "u128"},
                    {"name": "commission_rate", "type": "u64"}
                ]},
                {"name": "Delegation", "fields": [
                    {"name": "validator", "type": "address"},
                    {"name": "amount", "type": "u128"},
                    {"name": "unlock_time", "type": "u64"}
                ]}
            ],
            "functions": [
                {"name": "stake", "visibility": "public entry", "type_params": [], "params": ["&signer", "address", "u64"], "returns": []},
                {"name": "unstake", "visibility": "public entry", "type_params": [], "params": ["&signer", "address", "u64"], "returns": []},
                {"name": "claim_rewards", "visibility": "public entry", "type_params": [], "params": ["&signer"], "returns": []},
                {"name": "get_stake", "visibility": "public", "type_params": [], "params": ["address", "address"], "returns": ["u128"]},
                {"name": "get_rewards", "visibility": "public", "type_params": [], "params": ["address"], "returns": ["u64"]}
            ]
        }"#.to_string()
    }
    
    /// Timestamp module ABI
    fn timestamp_module_abi() -> String {
        r#"{
            "name": "Timestamp",
            "functions": [
                {"name": "now_microseconds", "visibility": "public", "type_params": [], "params": [], "returns": ["u64"]},
                {"name": "now_seconds", "visibility": "public", "type_params": [], "params": [], "returns": ["u64"]}
            ]
        }"#.to_string()
    }
    
    /// Event module ABI
    fn event_module_abi() -> String {
        r#"{
            "name": "Event",
            "structs": [
                {"name": "EventHandle", "type_params": ["T"], "fields": [
                    {"name": "counter", "type": "u64"},
                    {"name": "guid", "type": "vector<u8>"}
                ]}
            ],
            "functions": [
                {"name": "new_event_handle", "visibility": "public", "type_params": ["T"], "params": ["&signer"], "returns": ["EventHandle<T>"]},
                {"name": "emit_event", "visibility": "public", "type_params": ["T"], "params": ["&mut EventHandle<T>", "T"], "returns": []}
            ]
        }"#.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_load_stdlib() {
        let cache = ModuleCache::new(100);
        StdlibModules::load_into_cache(&cache);
        
        let framework = StdlibModules::framework_address();
        
        // Check modules are loaded
        assert!(cache.contains(&ModuleId::new(framework, "Coin".to_string())));
        assert!(cache.contains(&ModuleId::new(framework, "Staking".to_string())));
        assert!(cache.contains(&ModuleId::new(framework, "JASPR".to_string())));
    }
    
    #[test]
    fn test_framework_address() {
        let addr = StdlibModules::framework_address();
        let hex = addr.to_hex();
        assert!(hex.ends_with("01"));
    }
}
