//! Transaction types for JasprChain

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use crate::{Address, HashValue, Hasher, Gas, Amount, Nonce, ChainId};

/// Transaction type enum
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum TransactionType {
    /// Transfer native tokens
    Transfer,
    /// Stake tokens to validator
    Stake,
    /// Unstake tokens from validator
    Unstake,
    /// Deploy Move module
    ModuleDeploy,
    /// Execute Move script/function
    ScriptCall,
    /// Create new account
    CreateAccount,
}

/// Transaction payload variants
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum TransactionPayload {
    /// Native token transfer
    Transfer {
        recipient: Address,
        amount: Amount,
    },
    
    /// Stake to validator
    Stake {
        validator: Address,
        amount: Amount,
    },
    
    /// Unstake from validator
    Unstake {
        validator: Address,
        amount: Amount,
    },
    
    /// Deploy Move module
    ModuleDeploy {
        /// Module bytecode
        bytecode: Vec<u8>,
        /// Module ABI (JSON)
        abi: String,
    },
    
    /// Call Move function
    ScriptCall {
        /// Module address
        module_address: Address,
        /// Module name
        module_name: String,
        /// Function name
        function_name: String,
        /// Type arguments (serialized)
        type_args: Vec<String>,
        /// Function arguments (serialized)
        args: Vec<Vec<u8>>,
    },
    
    /// Create account
    CreateAccount {
        /// New account address
        new_address: Address,
        /// Initial balance to transfer
        initial_balance: Amount,
    },
}

impl TransactionPayload {
    /// Get transaction type
    pub fn tx_type(&self) -> TransactionType {
        match self {
            Self::Transfer { .. } => TransactionType::Transfer,
            Self::Stake { .. } => TransactionType::Stake,
            Self::Unstake { .. } => TransactionType::Unstake,
            Self::ModuleDeploy { .. } => TransactionType::ModuleDeploy,
            Self::ScriptCall { .. } => TransactionType::ScriptCall,
            Self::CreateAccount { .. } => TransactionType::CreateAccount,
        }
    }
}

/// Unsigned transaction
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Transaction {
    /// Sender address
    pub sender: Address,
    
    /// Transaction payload
    pub payload: TransactionPayload,
    
    /// Sequence number (nonce)
    pub nonce: Nonce,
    
    /// Maximum gas willing to pay
    pub max_gas: Gas,
    
    /// Gas price per unit
    pub gas_price: u64,
    
    /// Chain ID for replay protection
    pub chain_id: ChainId,
    
    /// Expiration timestamp (0 = no expiry)
    pub expiration_time: u64,
}

impl Transaction {
    /// Create new transaction
    pub fn new(
        sender: Address,
        payload: TransactionPayload,
        nonce: Nonce,
        max_gas: Gas,
        gas_price: u64,
        chain_id: ChainId,
    ) -> Self {
        Self {
            sender,
            payload,
            nonce,
            max_gas,
            gas_price,
            chain_id,
            expiration_time: 0,
        }
    }

    /// Compute transaction hash (for signing)
    pub fn signing_hash(&self) -> HashValue {
        let serialized = borsh::to_vec(self).expect("Serialization should not fail");
        HashValue::sha256(&serialized)
    }

    /// Get transaction type
    pub fn tx_type(&self) -> TransactionType {
        self.payload.tx_type()
    }
}

/// Signed transaction
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct SignedTransaction {
    /// Inner transaction
    pub transaction: Transaction,
    
    /// Ed25519 signature (64 bytes)
    pub signature: Vec<u8>,
    
    /// Public key of signer (32 bytes)
    pub public_key: Vec<u8>,
}

impl SignedTransaction {
    /// Create new signed transaction
    pub fn new(transaction: Transaction, signature: Vec<u8>, public_key: Vec<u8>) -> Self {
        Self {
            transaction,
            signature,
            public_key,
        }
    }

    /// Compute transaction hash
    pub fn hash(&self) -> HashValue {
        let serialized = borsh::to_vec(self).expect("Serialization should not fail");
        HashValue::sha256(&serialized)
    }

    /// Get sender
    pub fn sender(&self) -> Address {
        self.transaction.sender
    }

    /// Get nonce
    pub fn nonce(&self) -> Nonce {
        self.transaction.nonce
    }

    /// Get max gas
    pub fn max_gas(&self) -> Gas {
        self.transaction.max_gas
    }

    /// Get transaction type
    pub fn tx_type(&self) -> TransactionType {
        self.transaction.tx_type()
    }

    /// Verify signature matches transaction and public key
    pub fn verify_signature(&self) -> bool {
        // In production, use ed25519-dalek to verify
        // For now, verify public key matches sender
        let derived_address = Address::from_public_key(&self.public_key);
        derived_address == self.transaction.sender && self.signature.len() == 64
    }

    /// Get signing hash
    pub fn signing_hash(&self) -> HashValue {
        self.transaction.signing_hash()
    }
}

impl Hasher for SignedTransaction {
    fn hash(&self) -> HashValue {
        SignedTransaction::hash(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tx() -> Transaction {
        Transaction::new(
            Address::from_public_key(b"test_sender"),
            TransactionPayload::Transfer {
                recipient: Address::from_public_key(b"test_recipient"),
                amount: 1000,
            },
            1,
            100000,
            100,
            1,
        )
    }

    #[test]
    fn test_transaction_hash() {
        let tx = create_test_tx();
        let hash = tx.signing_hash();
        assert!(!hash.is_zero());
    }

    #[test]
    fn test_transaction_type() {
        let tx = create_test_tx();
        assert_eq!(tx.tx_type(), TransactionType::Transfer);
    }

    #[test]
    fn test_signed_transaction() {
        let tx = create_test_tx();
        let signed = SignedTransaction::new(
            tx,
            vec![0u8; 64], // Mock signature
            b"test_sender".to_vec(),
        );
        
        let hash = signed.hash();
        assert!(!hash.is_zero());
    }
}
