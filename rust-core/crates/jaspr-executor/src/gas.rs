//! Gas metering

use jaspr_types::Gas;

/// Gas costs for various operations
#[derive(Clone, Debug)]
pub struct GasConfig {
    /// Base cost for any transaction
    pub tx_base: Gas,
    /// Cost per byte of transaction data
    pub tx_data_byte: Gas,
    /// Cost for native transfer
    pub transfer: Gas,
    /// Cost for staking operation
    pub stake: Gas,
    /// Cost for unstaking operation
    pub unstake: Gas,
    /// Cost for module deployment per byte
    pub deploy_per_byte: Gas,
    /// Cost for script call base
    pub script_call_base: Gas,
    /// Cost for account creation
    pub create_account: Gas,
    /// Cost per storage write
    pub storage_write: Gas,
    /// Cost per storage read
    pub storage_read: Gas,
}

impl Default for GasConfig {
    fn default() -> Self {
        Self {
            tx_base: 21000,
            tx_data_byte: 16,
            transfer: 21000,
            stake: 50000,
            unstake: 50000,
            deploy_per_byte: 200,
            script_call_base: 10000,
            create_account: 25000,
            storage_write: 5000,
            storage_read: 200,
        }
    }
}

/// Gas meter for tracking gas usage
pub struct GasMeter {
    config: GasConfig,
    limit: Gas,
    used: Gas,
}

impl GasMeter {
    /// Create new gas meter with limit
    pub fn new(config: GasConfig, limit: Gas) -> Self {
        Self {
            config,
            limit,
            used: 0,
        }
    }
    
    /// Get remaining gas
    pub fn remaining(&self) -> Gas {
        self.limit.saturating_sub(self.used)
    }
    
    /// Get used gas
    pub fn used(&self) -> Gas {
        self.used
    }
    
    /// Get gas limit
    pub fn limit(&self) -> Gas {
        self.limit
    }
    
    /// Charge gas (returns error if out of gas)
    pub fn charge(&mut self, amount: Gas) -> Result<(), GasError> {
        if self.used + amount > self.limit {
            return Err(GasError::OutOfGas {
                needed: amount,
                remaining: self.remaining(),
            });
        }
        self.used += amount;
        Ok(())
    }
    
    /// Charge for transaction base cost
    pub fn charge_tx_base(&mut self) -> Result<(), GasError> {
        self.charge(self.config.tx_base)
    }
    
    /// Charge for transaction data
    pub fn charge_tx_data(&mut self, data_len: usize) -> Result<(), GasError> {
        self.charge(self.config.tx_data_byte * data_len as Gas)
    }
    
    /// Charge for transfer
    pub fn charge_transfer(&mut self) -> Result<(), GasError> {
        self.charge(self.config.transfer)
    }
    
    /// Charge for stake
    pub fn charge_stake(&mut self) -> Result<(), GasError> {
        self.charge(self.config.stake)
    }
    
    /// Charge for unstake
    pub fn charge_unstake(&mut self) -> Result<(), GasError> {
        self.charge(self.config.unstake)
    }
    
    /// Charge for module deployment
    pub fn charge_deploy(&mut self, bytecode_len: usize) -> Result<(), GasError> {
        self.charge(self.config.deploy_per_byte * bytecode_len as Gas)
    }
    
    /// Charge for script call
    pub fn charge_script_call(&mut self) -> Result<(), GasError> {
        self.charge(self.config.script_call_base)
    }
    
    /// Charge for account creation
    pub fn charge_create_account(&mut self) -> Result<(), GasError> {
        self.charge(self.config.create_account)
    }
    
    /// Charge for storage write
    pub fn charge_storage_write(&mut self) -> Result<(), GasError> {
        self.charge(self.config.storage_write)
    }
    
    /// Charge for storage read
    pub fn charge_storage_read(&mut self) -> Result<(), GasError> {
        self.charge(self.config.storage_read)
    }
    
    /// Check if out of gas
    pub fn is_out_of_gas(&self) -> bool {
        self.used >= self.limit
    }
}

/// Gas error
#[derive(Debug, Clone, thiserror::Error)]
pub enum GasError {
    #[error("Out of gas: needed {needed}, remaining {remaining}")]
    OutOfGas { needed: Gas, remaining: Gas },
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gas_meter() {
        let mut meter = GasMeter::new(GasConfig::default(), 100000);
        
        assert_eq!(meter.remaining(), 100000);
        assert_eq!(meter.used(), 0);
        
        meter.charge(50000).unwrap();
        assert_eq!(meter.remaining(), 50000);
        assert_eq!(meter.used(), 50000);
        
        // Should fail when out of gas
        assert!(meter.charge(60000).is_err());
    }
    
    #[test]
    fn test_charge_operations() {
        let mut meter = GasMeter::new(GasConfig::default(), 1_000_000);
        
        meter.charge_tx_base().unwrap();
        meter.charge_transfer().unwrap();
        meter.charge_stake().unwrap();
        
        assert!(meter.used() > 0);
    }
}
