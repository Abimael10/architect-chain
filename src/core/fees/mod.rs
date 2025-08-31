//! Fee calculation system for the blockchain
//!
//! This module provides both fixed and dynamic fee calculation capabilities:
//! - Fixed fees: Legacy system with constant fee amounts
//! - Dynamic fees: New system that adjusts fees based on network congestion and priority
//!
//! The system maintains complete backward compatibility while providing enhanced
//! fee market functionality.

pub mod calculator;
pub mod dynamic;
pub mod fixed;

// Re-export main types for convenience
pub use calculator::{FeeMode, LegacyFeeCalculator, UnifiedFeeCalculator};
pub use dynamic::{DynamicFeeCalculator, DynamicFeeConfig, FeePriority, FeeStatistics};
pub use fixed::FixedFeeCalculator;

use crate::error::{BlockchainError, Result};
use once_cell::sync::Lazy;
use std::sync::RwLock;

/// Global fee calculator instance
static GLOBAL_FEE_CALCULATOR: Lazy<RwLock<UnifiedFeeCalculator>> =
    Lazy::new(|| RwLock::new(UnifiedFeeCalculator::default()));

/// Fee calculation utilities and global access functions
pub struct FeeCalculator;

impl FeeCalculator {
    /// Calculate fee using the global fee calculator
    pub fn calculate_fee(transaction_size: usize, priority: Option<FeePriority>) -> u64 {
        match GLOBAL_FEE_CALCULATOR.read() {
            Ok(calculator) => calculator.calculate_fee(transaction_size, priority),
            Err(_) => {
                log::error!("Failed to acquire fee calculator lock, using default fee");
                1 // Fallback to default fee
            }
        }
    }

    /// Estimate fee for a given priority
    pub fn estimate_fee(priority: FeePriority) -> u64 {
        match GLOBAL_FEE_CALCULATOR.read() {
            Ok(calculator) => calculator.estimate_fee(priority),
            Err(_) => {
                log::error!("Failed to acquire fee calculator lock, using default fee");
                1
            }
        }
    }

    /// Validate a fee amount
    pub fn validate_fee(fee: u64, priority: Option<FeePriority>) -> Result<()> {
        match GLOBAL_FEE_CALCULATOR.read() {
            Ok(calculator) => calculator.validate_fee(fee, priority),
            Err(_) => {
                log::error!("Failed to acquire fee calculator lock");
                Err(BlockchainError::Config(
                    "Fee calculator lock error".to_string(),
                ))
            }
        }
    }

    /// Calculate coinbase reward
    pub fn calculate_coinbase_reward(collected_fees: u64) -> u64 {
        match GLOBAL_FEE_CALCULATOR.read() {
            Ok(calculator) => calculator.calculate_coinbase_reward(collected_fees),
            Err(_) => {
                log::error!("Failed to acquire fee calculator lock, using default reward");
                crate::core::INITIAL_BLOCK_REWARD + collected_fees // Default calculation
            }
        }
    }

    /// Get current fee mode
    pub fn get_fee_mode() -> FeeMode {
        match GLOBAL_FEE_CALCULATOR.read() {
            Ok(calculator) => calculator.get_mode().clone(),
            Err(_) => {
                log::error!("Failed to acquire fee calculator lock, returning default mode");
                FeeMode::default()
            }
        }
    }

    /// Switch fee mode
    pub fn switch_fee_mode(new_mode: FeeMode) -> Result<()> {
        match GLOBAL_FEE_CALCULATOR.write() {
            Ok(mut calculator) => calculator.switch_mode(new_mode),
            Err(_) => {
                log::error!("Failed to acquire fee calculator write lock");
                Err(BlockchainError::Config(
                    "Fee calculator lock error".to_string(),
                ))
            }
        }
    }

    /// Check if dynamic fees are enabled
    pub fn is_dynamic_enabled() -> bool {
        match GLOBAL_FEE_CALCULATOR.read() {
            Ok(calculator) => calculator.is_dynamic_enabled(),
            Err(_) => false,
        }
    }

    /// Get fee statistics (only available in dynamic mode)
    pub fn get_fee_statistics() -> Option<FeeStatistics> {
        match GLOBAL_FEE_CALCULATOR.read() {
            Ok(calculator) => calculator.get_fee_statistics(),
            Err(_) => None,
        }
    }

    /// Get configuration summary
    pub fn get_config_summary() -> String {
        match GLOBAL_FEE_CALCULATOR.read() {
            Ok(calculator) => calculator.get_config_summary(),
            Err(_) => "Fee calculator unavailable".to_string(),
        }
    }

    /// Update dynamic fee configuration
    pub fn update_dynamic_config(config: DynamicFeeConfig) -> Result<()> {
        match GLOBAL_FEE_CALCULATOR.write() {
            Ok(mut calculator) => calculator.update_dynamic_config(config),
            Err(_) => {
                log::error!("Failed to acquire fee calculator write lock");
                Err(BlockchainError::Config(
                    "Fee calculator lock error".to_string(),
                ))
            }
        }
    }

    /// Update fixed fee amount
    pub fn update_fixed_fee(amount: u64) -> Result<()> {
        match GLOBAL_FEE_CALCULATOR.write() {
            Ok(mut calculator) => calculator.update_fixed_fee(amount),
            Err(_) => {
                log::error!("Failed to acquire fee calculator write lock");
                Err(BlockchainError::Config(
                    "Fee calculator lock error".to_string(),
                ))
            }
        }
    }

    /// Initialize fee calculator with specific mode
    pub fn initialize(mode: FeeMode) -> Result<()> {
        match GLOBAL_FEE_CALCULATOR.write() {
            Ok(mut calculator) => {
                *calculator = UnifiedFeeCalculator::new(mode)?;
                log::info!("Initialized global fee calculator");
                Ok(())
            }
            Err(_) => {
                log::error!("Failed to acquire fee calculator write lock for initialization");
                Err(BlockchainError::Config(
                    "Fee calculator lock error".to_string(),
                ))
            }
        }
    }

    // Legacy compatibility constants and functions (updated for new monetary system)
    pub const COINBASE_REWARD: u64 = crate::core::INITIAL_BLOCK_REWARD;
    pub const EDUCATIONAL_FEE: u64 = crate::core::DEFAULT_TRANSACTION_FEE;
    pub const DEFAULT_FEE_RATE: u64 = 1;
    pub const MIN_FEE_RATE: u64 = 1;
    pub const MAX_FEE_RATE: u64 = 1000;

    /// Legacy fee calculation for backward compatibility
    pub fn calculate_legacy_fee(transaction_size: usize, fee_rate: u64) -> Result<u64> {
        LegacyFeeCalculator::calculate_fee(transaction_size, fee_rate)
    }

    /// Legacy fee rate validation
    pub fn validate_fee_rate(fee_rate: u64) -> Result<()> {
        LegacyFeeCalculator::validate_fee_rate(fee_rate)
    }

    /// Legacy fee amount validation
    pub fn validate_fee_amount(fee: u64, transaction_size: usize) -> Result<()> {
        if transaction_size == 0 {
            return Err(BlockchainError::Transaction(
                "Cannot validate fee for zero-size transaction".to_string(),
            ));
        }

        let fee_rate = fee / transaction_size as u64;
        Self::validate_fee_rate(fee_rate)
    }

    /// Legacy fee rate calculation
    pub fn calculate_fee_rate(fee: u64, transaction_size: usize) -> Result<u64> {
        if transaction_size == 0 {
            return Err(BlockchainError::Transaction(
                "Transaction size cannot be zero for fee rate calculation".to_string(),
            ));
        }

        Ok(fee / transaction_size as u64)
    }

    /// Legacy transaction size estimation
    pub fn estimate_transaction_size(input_count: usize, output_count: usize) -> usize {
        // Simplified estimation for educational blockchain
        let base_size = 10;
        let input_size = input_count * 50;
        let output_size = output_count * 20;
        let fee_size = 8;

        base_size + input_size + output_size + fee_size
    }

    /// Legacy total fees calculation
    pub fn calculate_total_fees<'a, I>(transactions: I) -> u64
    where
        I: Iterator<Item = &'a crate::core::Transaction>,
    {
        transactions
            .filter(|tx| !tx.is_coinbase())
            .map(|tx| tx.get_fee())
            .sum()
    }

    /// Legacy satoshi conversion functions
    pub fn satoshis_to_coins(satoshis: u64) -> f64 {
        satoshis as f64 / 100_000_000.0
    }

    pub fn coins_to_satoshis(coins: f64) -> u64 {
        (coins * 100_000_000.0) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_fee_calculator_fixed_mode() {
        // Initialize with fixed mode
        FeeCalculator::initialize(FeeMode::Fixed { amount: 2 }).unwrap();

        assert!(!FeeCalculator::is_dynamic_enabled());
        assert_eq!(FeeCalculator::calculate_fee(100, None), 2);
        assert_eq!(FeeCalculator::estimate_fee(FeePriority::High), 2);
    }

    #[test]
    fn test_global_fee_calculator_dynamic_mode() {
        // Initialize with dynamic mode
        let config = DynamicFeeConfig::default();
        FeeCalculator::initialize(FeeMode::Dynamic { config }).unwrap();

        assert!(FeeCalculator::is_dynamic_enabled());

        let fee_normal = FeeCalculator::estimate_fee(FeePriority::Normal);
        let fee_high = FeeCalculator::estimate_fee(FeePriority::High);

        assert!(fee_high >= fee_normal);
    }

    #[test]
    fn test_fee_mode_switching() {
        // Start with fixed mode
        FeeCalculator::initialize(FeeMode::Fixed { amount: 1 }).unwrap();
        assert!(!FeeCalculator::is_dynamic_enabled());

        // Switch to dynamic mode
        let config = DynamicFeeConfig::default();
        FeeCalculator::switch_fee_mode(FeeMode::Dynamic { config }).unwrap();
        assert!(FeeCalculator::is_dynamic_enabled());

        // Switch back to fixed mode
        FeeCalculator::switch_fee_mode(FeeMode::Fixed { amount: 3 }).unwrap();
        assert!(!FeeCalculator::is_dynamic_enabled());
        assert_eq!(FeeCalculator::calculate_fee(100, None), 3);
    }

    #[test]
    fn test_coinbase_reward_calculation() {
        FeeCalculator::initialize(FeeMode::Fixed { amount: 1 }).unwrap();
        let reward = FeeCalculator::calculate_coinbase_reward(5);
        assert_eq!(reward, crate::core::INITIAL_BLOCK_REWARD + 5); // Base reward + 5 fees
    }

    #[test]
    fn test_fee_validation() {
        FeeCalculator::initialize(FeeMode::Fixed { amount: 2 }).unwrap();
        assert!(FeeCalculator::validate_fee(2, None).is_ok());
        assert!(FeeCalculator::validate_fee(1, None).is_err());
    }

    #[test]
    fn test_legacy_compatibility() {
        // Test that legacy functions still work
        assert_eq!(FeeCalculator::calculate_legacy_fee(100, 2).unwrap(), 200);
        assert!(FeeCalculator::validate_fee_rate(10).is_ok());
        assert_eq!(FeeCalculator::estimate_transaction_size(2, 2), 158);
    }

    #[test]
    fn test_config_summary() {
        FeeCalculator::initialize(FeeMode::Fixed { amount: 5 }).unwrap();
        let summary = FeeCalculator::get_config_summary();
        assert!(summary.contains("Fixed fee: 5 coins"));
    }

    #[test]
    fn test_fee_statistics_dynamic_mode() {
        let config = DynamicFeeConfig::default();
        FeeCalculator::initialize(FeeMode::Dynamic { config }).unwrap();

        let stats = FeeCalculator::get_fee_statistics();
        assert!(stats.is_some());

        let stats = stats.unwrap();
        assert_eq!(stats.base_fee, 1);
        assert!(stats.estimated_fees.contains_key(&FeePriority::Normal));
    }

    #[test]
    fn test_fee_statistics_fixed_mode() {
        FeeCalculator::initialize(FeeMode::Fixed { amount: 1 }).unwrap();

        let stats = FeeCalculator::get_fee_statistics();
        assert!(stats.is_none()); // Not available in fixed mode
    }
}
