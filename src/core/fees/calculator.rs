use crate::core::fees::{
    dynamic::{DynamicFeeCalculator, DynamicFeeConfig, FeePriority, FeeStatistics},
    fixed::FixedFeeCalculator,
};
use crate::error::{BlockchainError, Result};
use log::info;
use serde::{Deserialize, Serialize};

/// Fee calculation mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeeMode {
    /// Fixed fee mode (legacy)
    Fixed { amount: u64 },
    /// Dynamic fee mode (new)
    Dynamic { config: DynamicFeeConfig },
}

impl Default for FeeMode {
    fn default() -> Self {
        FeeMode::Fixed { amount: 1 } // Default to legacy fixed fee for backward compatibility
    }
}

/// Unified fee calculator that supports both fixed and dynamic fee modes
#[derive(Debug, Clone)]
pub struct UnifiedFeeCalculator {
    mode: FeeMode,
    fixed_calculator: Option<FixedFeeCalculator>,
    dynamic_calculator: Option<DynamicFeeCalculator>,
}

impl UnifiedFeeCalculator {
    /// Create a new unified fee calculator
    pub fn new(mode: FeeMode) -> Result<Self> {
        let mut calculator = Self {
            mode: mode.clone(),
            fixed_calculator: None,
            dynamic_calculator: None,
        };

        calculator.initialize_calculators()?;
        Ok(calculator)
    }

    /// Initialize the appropriate calculator based on mode
    fn initialize_calculators(&mut self) -> Result<()> {
        match &self.mode {
            FeeMode::Fixed { amount } => {
                self.fixed_calculator = Some(FixedFeeCalculator::new(*amount));
                self.dynamic_calculator = None;
                info!("Initialized fixed fee calculator with {amount} coins");
            }
            FeeMode::Dynamic { config } => {
                self.dynamic_calculator = Some(DynamicFeeCalculator::new(config.clone())?);
                self.fixed_calculator = None;
                info!("Initialized dynamic fee calculator");
            }
        }
        Ok(())
    }

    /// Calculate transaction fee
    pub fn calculate_fee(&self, transaction_size: usize, priority: Option<FeePriority>) -> u64 {
        match &self.mode {
            FeeMode::Fixed { .. } => {
                if let Some(ref calculator) = self.fixed_calculator {
                    calculator.calculate_fee(transaction_size, priority)
                } else {
                    1 // Fallback to default
                }
            }
            FeeMode::Dynamic { .. } => {
                if let Some(ref calculator) = self.dynamic_calculator {
                    let priority = priority.unwrap_or(FeePriority::Normal);
                    let mempool_size = crate::storage::GLOBAL_MEMORY_POOL.len();
                    calculator.calculate_fee(priority, mempool_size)
                } else {
                    1 // Fallback to default
                }
            }
        }
    }

    /// Calculate fee with explicit mempool size (for testing)
    pub fn calculate_fee_with_mempool_size(
        &self,
        transaction_size: usize,
        priority: Option<FeePriority>,
        mempool_size: usize,
    ) -> u64 {
        match &self.mode {
            FeeMode::Fixed { .. } => {
                if let Some(ref calculator) = self.fixed_calculator {
                    calculator.calculate_fee(transaction_size, priority)
                } else {
                    1
                }
            }
            FeeMode::Dynamic { .. } => {
                if let Some(ref calculator) = self.dynamic_calculator {
                    let priority = priority.unwrap_or(FeePriority::Normal);
                    calculator.calculate_fee(priority, mempool_size)
                } else {
                    1
                }
            }
        }
    }

    /// Estimate fee for a given priority
    pub fn estimate_fee(&self, priority: FeePriority) -> u64 {
        match &self.mode {
            FeeMode::Fixed { amount } => *amount,
            FeeMode::Dynamic { .. } => {
                if let Some(ref calculator) = self.dynamic_calculator {
                    calculator.estimate_fee(priority)
                } else {
                    1
                }
            }
        }
    }

    /// Validate a fee amount
    pub fn validate_fee(&self, fee: u64, priority: Option<FeePriority>) -> Result<()> {
        match &self.mode {
            FeeMode::Fixed { .. } => {
                if let Some(ref calculator) = self.fixed_calculator {
                    calculator.validate_fee(fee)
                } else {
                    Ok(())
                }
            }
            FeeMode::Dynamic { .. } => {
                if let Some(ref calculator) = self.dynamic_calculator {
                    let priority = priority.unwrap_or(FeePriority::Normal);
                    let mempool_size = crate::storage::GLOBAL_MEMORY_POOL.len();
                    calculator.validate_fee(fee, priority, mempool_size)
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Calculate coinbase reward
    pub fn calculate_coinbase_reward(&self, collected_fees: u64) -> u64 {
        match &self.mode {
            FeeMode::Fixed { .. } => {
                if let Some(ref calculator) = self.fixed_calculator {
                    calculator.calculate_coinbase_reward(collected_fees)
                } else {
                    crate::core::INITIAL_BLOCK_REWARD + collected_fees // Use proper monetary constant
                }
            }
            FeeMode::Dynamic { .. } => {
                if let Some(ref calculator) = self.dynamic_calculator {
                    calculator.calculate_coinbase_reward(collected_fees)
                } else {
                    crate::core::INITIAL_BLOCK_REWARD + collected_fees // Use proper monetary constant
                }
            }
        }
    }

    /// Get current fee mode
    pub fn get_mode(&self) -> &FeeMode {
        &self.mode
    }

    /// Switch to a new fee mode
    pub fn switch_mode(&mut self, new_mode: FeeMode) -> Result<()> {
        info!("Switching fee mode from {:?} to {:?}", self.mode, new_mode);
        self.mode = new_mode;
        self.initialize_calculators()?;
        Ok(())
    }

    /// Check if dynamic fees are enabled
    pub fn is_dynamic_enabled(&self) -> bool {
        matches!(self.mode, FeeMode::Dynamic { .. })
    }

    /// Check if fixed fees are enabled
    pub fn is_fixed_enabled(&self) -> bool {
        matches!(self.mode, FeeMode::Fixed { .. })
    }

    /// Get fee statistics (only available for dynamic mode)
    pub fn get_fee_statistics(&self) -> Option<FeeStatistics> {
        match &self.mode {
            FeeMode::Dynamic { .. } => {
                if let Some(ref calculator) = self.dynamic_calculator {
                    let mempool_size = crate::storage::GLOBAL_MEMORY_POOL.len();
                    Some(calculator.get_fee_statistics(mempool_size))
                } else {
                    None
                }
            }
            FeeMode::Fixed { .. } => None,
        }
    }

    /// Get current configuration as a displayable string
    pub fn get_config_summary(&self) -> String {
        match &self.mode {
            FeeMode::Fixed { amount } => {
                format!("Fixed fee: {amount} coins")
            }
            FeeMode::Dynamic { config } => {
                format!(
                    "Dynamic fees: base {} coins, max {} coins, threshold {} transactions",
                    config.base_fee, config.max_fee, config.congestion_threshold
                )
            }
        }
    }

    /// Update dynamic fee configuration (only works in dynamic mode)
    pub fn update_dynamic_config(&mut self, new_config: DynamicFeeConfig) -> Result<()> {
        match &mut self.mode {
            FeeMode::Dynamic { config } => {
                *config = new_config.clone();
                if let Some(ref mut calculator) = self.dynamic_calculator {
                    calculator.update_config(new_config)?;
                }
                info!("Updated dynamic fee configuration");
                Ok(())
            }
            FeeMode::Fixed { .. } => Err(BlockchainError::Config(
                "Cannot update dynamic config in fixed fee mode".to_string(),
            )),
        }
    }

    /// Update fixed fee amount (only works in fixed mode)
    pub fn update_fixed_fee(&mut self, new_amount: u64) -> Result<()> {
        match &mut self.mode {
            FeeMode::Fixed { amount } => {
                *amount = new_amount;
                if let Some(ref mut calculator) = self.fixed_calculator {
                    calculator.set_fee_amount(new_amount);
                }
                info!("Updated fixed fee to {new_amount} coins");
                Ok(())
            }
            FeeMode::Dynamic { .. } => Err(BlockchainError::Config(
                "Cannot update fixed fee in dynamic fee mode".to_string(),
            )),
        }
    }
}

impl Default for UnifiedFeeCalculator {
    fn default() -> Self {
        Self::new(FeeMode::default()).expect("Failed to create default fee calculator")
    }
}

/// Legacy compatibility functions for existing code
pub struct LegacyFeeCalculator;

impl LegacyFeeCalculator {
    /// Legacy fee calculation for backward compatibility
    pub fn calculate_fee(transaction_size: usize, fee_rate: u64) -> Result<u64> {
        if transaction_size == 0 {
            return Err(BlockchainError::Transaction(
                "Transaction size cannot be zero".to_string(),
            ));
        }

        // For backward compatibility, use simple multiplication
        Ok(transaction_size as u64 * fee_rate)
    }

    /// Legacy fee validation
    pub fn validate_fee_rate(fee_rate: u64) -> Result<()> {
        const MIN_FEE_RATE: u64 = 1;
        const MAX_FEE_RATE: u64 = 1000;

        if fee_rate < MIN_FEE_RATE {
            return Err(BlockchainError::Transaction(format!(
                "Fee rate {fee_rate} below minimum {MIN_FEE_RATE} sat/byte"
            )));
        }

        if fee_rate > MAX_FEE_RATE {
            return Err(BlockchainError::Transaction(format!(
                "Fee rate {fee_rate} above maximum {MAX_FEE_RATE} sat/byte"
            )));
        }

        Ok(())
    }

    /// Legacy coinbase reward calculation
    pub fn calculate_coinbase_reward(collected_fees: u64) -> u64 {
        crate::core::INITIAL_BLOCK_REWARD + collected_fees // Base reward + fees
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_mode_calculator() {
        let calculator = UnifiedFeeCalculator::new(FeeMode::Fixed { amount: 2 }).unwrap();

        assert!(calculator.is_fixed_enabled());
        assert!(!calculator.is_dynamic_enabled());

        // Should always return fixed amount
        assert_eq!(calculator.calculate_fee(100, None), 2);
        assert_eq!(calculator.calculate_fee(1000, Some(FeePriority::High)), 2);
    }

    #[test]
    fn test_dynamic_mode_calculator() {
        let config = DynamicFeeConfig::default();
        let calculator = UnifiedFeeCalculator::new(FeeMode::Dynamic { config }).unwrap();

        assert!(!calculator.is_fixed_enabled());
        assert!(calculator.is_dynamic_enabled());

        // Should calculate based on priority and congestion
        let fee_normal =
            calculator.calculate_fee_with_mempool_size(100, Some(FeePriority::Normal), 5);
        let fee_high = calculator.calculate_fee_with_mempool_size(100, Some(FeePriority::High), 5);

        assert!(fee_high >= fee_normal);
    }

    #[test]
    fn test_mode_switching() {
        let mut calculator = UnifiedFeeCalculator::new(FeeMode::Fixed { amount: 1 }).unwrap();

        assert!(calculator.is_fixed_enabled());

        // Switch to dynamic mode
        let dynamic_config = DynamicFeeConfig::default();
        calculator
            .switch_mode(FeeMode::Dynamic {
                config: dynamic_config,
            })
            .unwrap();

        assert!(calculator.is_dynamic_enabled());

        // Switch back to fixed mode
        calculator
            .switch_mode(FeeMode::Fixed { amount: 3 })
            .unwrap();

        assert!(calculator.is_fixed_enabled());
        assert_eq!(calculator.calculate_fee(100, None), 3);
    }

    #[test]
    fn test_fee_estimation() {
        let config = DynamicFeeConfig::default();
        let calculator = UnifiedFeeCalculator::new(FeeMode::Dynamic { config }).unwrap();

        let estimate_low = calculator.estimate_fee(FeePriority::Low);
        let estimate_high = calculator.estimate_fee(FeePriority::High);

        assert!(estimate_high >= estimate_low);
    }

    #[test]
    fn test_coinbase_reward_calculation() {
        let calculator = UnifiedFeeCalculator::new(FeeMode::Fixed { amount: 1 }).unwrap();
        let reward = calculator.calculate_coinbase_reward(5);
        assert_eq!(reward, crate::core::INITIAL_BLOCK_REWARD + 5); // Base reward + 5 fees
    }

    #[test]
    fn test_config_summary() {
        let fixed_calculator = UnifiedFeeCalculator::new(FeeMode::Fixed { amount: 2 }).unwrap();
        let summary = fixed_calculator.get_config_summary();
        assert!(summary.contains("Fixed fee: 2 coins"));

        let dynamic_calculator = UnifiedFeeCalculator::new(FeeMode::Dynamic {
            config: DynamicFeeConfig::default(),
        })
        .unwrap();
        let summary = dynamic_calculator.get_config_summary();
        assert!(summary.contains("Dynamic fees"));
    }

    #[test]
    fn test_legacy_compatibility() {
        // Test that legacy functions still work
        assert_eq!(LegacyFeeCalculator::calculate_fee(100, 2).unwrap(), 200);
        assert!(LegacyFeeCalculator::validate_fee_rate(10).is_ok());
        assert_eq!(
            LegacyFeeCalculator::calculate_coinbase_reward(5),
            crate::core::INITIAL_BLOCK_REWARD + 5
        );
    }
}
