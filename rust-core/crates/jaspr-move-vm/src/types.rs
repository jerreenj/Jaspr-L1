//! Move type helpers

use jaspr_types::Address;
use serde::{Deserialize, Serialize};

/// Move type tag representation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeTag {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    Address,
    Signer,
    Vector(Box<TypeTag>),
    Struct(StructTag),
}

/// Struct type tag
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StructTag {
    pub address: String,
    pub module: String,
    pub name: String,
    pub type_args: Vec<TypeTag>,
}

impl StructTag {
    /// Create new struct tag
    pub fn new(address: &str, module: &str, name: &str) -> Self {
        Self {
            address: address.to_string(),
            module: module.to_string(),
            name: name.to_string(),
            type_args: Vec::new(),
        }
    }
    
    /// With type arguments
    pub fn with_type_args(mut self, args: Vec<TypeTag>) -> Self {
        self.type_args = args;
        self
    }
    
    /// To string representation
    pub fn to_string(&self) -> String {
        let args = if self.type_args.is_empty() {
            String::new()
        } else {
            format!("<{}>", self.type_args.iter()
                .map(|t| type_tag_to_string(t))
                .collect::<Vec<_>>()
                .join(", "))
        };
        format!("{}::{}::{}{}", self.address, self.module, self.name, args)
    }
}

/// Convert type tag to string
pub fn type_tag_to_string(tag: &TypeTag) -> String {
    match tag {
        TypeTag::Bool => "bool".to_string(),
        TypeTag::U8 => "u8".to_string(),
        TypeTag::U16 => "u16".to_string(),
        TypeTag::U32 => "u32".to_string(),
        TypeTag::U64 => "u64".to_string(),
        TypeTag::U128 => "u128".to_string(),
        TypeTag::U256 => "u256".to_string(),
        TypeTag::Address => "address".to_string(),
        TypeTag::Signer => "signer".to_string(),
        TypeTag::Vector(inner) => format!("vector<{}>", type_tag_to_string(inner)),
        TypeTag::Struct(st) => st.to_string(),
    }
}

/// Parse type tag from string
pub fn parse_type_tag(s: &str) -> Result<TypeTag, String> {
    let s = s.trim();
    match s {
        "bool" => Ok(TypeTag::Bool),
        "u8" => Ok(TypeTag::U8),
        "u16" => Ok(TypeTag::U16),
        "u32" => Ok(TypeTag::U32),
        "u64" => Ok(TypeTag::U64),
        "u128" => Ok(TypeTag::U128),
        "u256" => Ok(TypeTag::U256),
        "address" => Ok(TypeTag::Address),
        "signer" => Ok(TypeTag::Signer),
        _ if s.starts_with("vector<") && s.ends_with(">") => {
            let inner = &s[7..s.len()-1];
            Ok(TypeTag::Vector(Box::new(parse_type_tag(inner)?)))
        }
        _ if s.contains("::") => {
            // Parse struct tag
            let parts: Vec<&str> = s.split("::").collect();
            if parts.len() >= 3 {
                let struct_tag = StructTag::new(parts[0], parts[1], parts[2]);
                Ok(TypeTag::Struct(struct_tag))
            } else {
                Err(format!("Invalid struct type: {}", s))
            }
        }
        _ => Err(format!("Unknown type: {}", s)),
    }
}

/// Move value representation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MoveValue {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    U256([u8; 32]),
    Address([u8; 32]),
    Signer([u8; 32]),
    Vector(Vec<MoveValue>),
    Struct(Vec<MoveValue>),
}

impl MoveValue {
    /// Serialize to BCS-like bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        // Simplified serialization
        match self {
            Self::Bool(b) => vec![if *b { 1 } else { 0 }],
            Self::U8(v) => vec![*v],
            Self::U16(v) => v.to_le_bytes().to_vec(),
            Self::U32(v) => v.to_le_bytes().to_vec(),
            Self::U64(v) => v.to_le_bytes().to_vec(),
            Self::U128(v) => v.to_le_bytes().to_vec(),
            Self::U256(v) => v.to_vec(),
            Self::Address(v) => v.to_vec(),
            Self::Signer(v) => v.to_vec(),
            Self::Vector(items) => {
                let mut bytes = Vec::new();
                bytes.extend_from_slice(&(items.len() as u32).to_le_bytes());
                for item in items {
                    bytes.extend(item.to_bytes());
                }
                bytes
            }
            Self::Struct(fields) => {
                let mut bytes = Vec::new();
                for field in fields {
                    bytes.extend(field.to_bytes());
                }
                bytes
            }
        }
    }
    
    /// Create U64 value
    pub fn u64(v: u64) -> Self {
        Self::U64(v)
    }
    
    /// Create U128 value
    pub fn u128(v: u128) -> Self {
        Self::U128(v)
    }
    
    /// Create address value
    pub fn address(addr: &Address) -> Self {
        Self::Address(*addr.as_bytes())
    }
    
    /// Create bool value
    pub fn bool(v: bool) -> Self {
        Self::Bool(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_primitive_types() {
        assert_eq!(parse_type_tag("bool").unwrap(), TypeTag::Bool);
        assert_eq!(parse_type_tag("u64").unwrap(), TypeTag::U64);
        assert_eq!(parse_type_tag("address").unwrap(), TypeTag::Address);
    }
    
    #[test]
    fn test_parse_vector() {
        let tag = parse_type_tag("vector<u8>").unwrap();
        assert!(matches!(tag, TypeTag::Vector(_)));
    }
    
    #[test]
    fn test_struct_tag() {
        let tag = StructTag::new("0x1", "Coin", "Coin")
            .with_type_args(vec![TypeTag::Struct(StructTag::new("0x1", "JASPR", "JASPR"))]);
        
        let s = tag.to_string();
        assert!(s.contains("0x1::Coin::Coin"));
    }
}
