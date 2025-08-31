use crate::error::{BlockchainError, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Priority levels for transaction fees
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeePriority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

impl std::fmt::Display for FeePriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeePriority::Low => write!(f, "low"),
            FeePriority::Normal => write!(f, "normal"),
            FeePriority::High => write!(f, "high"),
            FeePriority::Urgent => write!(f, "urgent"),
        }
    }
}

impl std::str::FromStr for FeePriority {
    type Err = BlockchainError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "low" => Ok(FeePriority::Low),
            "normal" => Ok(FeePriority::Normal),
            "high" => Ok(FeePriority::High),
            "urgent" => Ok(FeePriority::Urgent),
            _ => Err(BlockchainError::Config(format!("Invalid priority: {s}"))),
        }
    }
}

/// Configuration for dynamic fee calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicFeeConfig {
    /// Base fee amount (minimum fee)
    pub base_fee: u64,
    /// Maximum fee cap
    pub max_fee: u64,
    /// Mempool size threshold for congestion detection
    pub congestion_threshold: usize,
    /// Multipliers for different priority levels
    pub priority_multipliers: HashMap<FeePriority, f64>,
    /// Base coinbase reward
    pub coinbase_reward: u64,
}

impl DynamicFeeConfig {
    /// Create a new dynamic fee config with base fee
    pub fn with_base_fee(base_fee: u64) -> Self {
        Self {
            base_fee,
            max_fee: base_fee * 10, // Default max is 10x base
            congestion_threshold: 20,
            priority_multipliers: Self::default_priority_multipliers(),
            coinbase_reward: crate::core::INITIAL_BLOCK_REWARD,
        }
    }

    /// Default priority multipliers
    pub fn default_priority_multipliers() -> HashMap<FeePriority, f64> {
        let mut multipliers = HashMap::new();
        multipliers.insert(FeePriority::Low, 0.5);
        multipliers.insert(FeePriority::Normal, 1.0);
        multipliers.insert(FeePriority::High, 2.0);
        multipliers.insert(FeePriority::Urgent, 3.0);
        multipliers
    }

    /// Validate configuration parameters
    pub fn validate(&self) -> Result<()> {
        if self.base_fee == 0 {
            return Err(BlockchainError::Config(
                "Base fee cannot be zero".to_string(),
            ));
        }

        if self.max_fee < self.base_fee {
            return Err(BlockchainError::Config(
                "Maximum fee cannot be less than base fee".to_string(),
            ));
        }

        if self.congestion_threshold == 0 {
            return Err(BlockchainError::Config(
                "Congestion threshold cannot be zero".to_string(),
            ));
        }

        // Validate priority multipliers
        for priority in [
            FeePriority::Low,
            FeePriority::Normal,
            FeePriority::High,
            FeePriority::Urgent,
        ] {
            if !self.priority_multipliers.contains_key(&priority) {
                return Err(BlockchainError::Config(format!(
                    "Missing priority multiplier for {priority:?}"
                )));
            }
        }

        Ok(())
    }
}

impl Default for DynamicFeeConfig {
    fn default() -> Self {
        Self::with_base_fee(1)
    }
}

/// Dynamic fee calculator that adjusts fees based on network congestion and priority
#[derive(Debug, Clone)]
pub struct DynamicFeeCalculator {
    config: DynamicFeeConfig,
}

impl DynamicFeeCalculator {
    /// Create a new dynamic fee calculator
    pub fn new(config: DynamicFeeConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Calculate fee based on priority and current mempool size
    pub fn calculate_fee(&self, priority: FeePriority, mempool_size: usize) -> u64 {
        let base = self.config.base_fee as f64;
        let priority_multiplier = self.get_priority_multiplier(priority);
        let congestion_multiplier = self.calculate_congestion_multiplier(mempool_size);

        let calculated_fee = base * priority_multiplier * congestion_multiplier;
        let final_fee = calculated_fee.max(0.0) as u64; // Ensure non-negative before casting

        // Apply caps
        let capped_fee = final_fee.max(self.config.base_fee).min(self.config.max_fee);

        info!(
            "Calculated fee: {} coins (priority: {}, mempool: {}, base: {}, priority_mult: {:.2}, congestion_mult: {:.2})",
            capped_fee, priority, mempool_size, self.config.base_fee, priority_multiplier, congestion_multiplier
        );

        capped_fee
    }

    /// Get priority multiplier for a given priority level
    fn get_priority_multiplier(&self, priority: FeePriority) -> f64 {
        self.config
            .priority_multipliers
            .get(&priority)
            .copied()
            .unwrap_or(1.0)
    }

    /// Calculate congestion multiplier based on mempool size
    fn calculate_congestion_multiplier(&self, mempool_size: usize) -> f64 {
        if mempool_size <= self.config.congestion_threshold {
            1.0 // No congestion
        } else {
            // Linear increase up to 3x for high congestion
            let congestion_ratio = mempool_size as f64 / self.config.congestion_threshold as f64;
            let multiplier = 1.0 + (congestion_ratio - 1.0) * 2.0;
            multiplier.min(3.0) // Cap at 3x
        }
    }

    /// Estimate fee for a given priority (uses current mempool size)
    pub fn estimate_fee(&self, priority: FeePriority) -> u64 {
        // Get current mempool size from global memory pool
        let mempool_size = crate::storage::GLOBAL_MEMORY_POOL.len();
        self.calculate_fee(priority, mempool_size)
    }

    /// Validate that a fee is appropriate for the given conditions
    pub fn validate_fee(&self, fee: u64, priority: FeePriority, mempool_size: usize) -> Result<()> {
        let expected_fee = self.calculate_fee(priority, mempool_size);

        // Allow some tolerance for fee validation (±10%)
        let tolerance = ((expected_fee as f64 * 0.1).max(0.0)) as u64; // Ensure non-negative
        let min_acceptable = expected_fee.saturating_sub(tolerance);
        let max_acceptable = expected_fee + tolerance;

        if fee < min_acceptable || fee > max_acceptable {
            warn!("Fee validation failed: provided {fee}, expected {expected_fee} (±{tolerance})");
            return Err(BlockchainError::Transaction(format!(
                "Invalid fee: provided {fee}, expected {expected_fee} (±{tolerance})"
            )));
        }

        Ok(())
    }

    /// Calculate coinbase reward with collected fees
    pub fn calculate_coinbase_reward(&self, collected_fees: u64) -> u64 {
        self.config.coinbase_reward + collected_fees
    }

    /// Get current configuration
    pub fn get_config(&self) -> &DynamicFeeConfig {
        &self.config
    }

    /// Update configuration (validates before applying)
    pub fn update_config(&mut self, new_config: DynamicFeeConfig) -> Result<()> {
        new_config.validate()?;
        self.config = new_config;
        info!("Updated dynamic fee configuration");
        Ok(())
    }

    /// Get fee statistics for monitoring
    pub fn get_fee_statistics(&self, mempool_size: usize) -> FeeStatistics {
        FeeStatistics {
            base_fee: self.config.base_fee,
            max_fee: self.config.max_fee,
            current_congestion_multiplier: self.calculate_congestion_multiplier(mempool_size),
            mempool_size,
            congestion_threshold: self.config.congestion_threshold,
            estimated_fees: {
                let mut fees = HashMap::new();
                for priority in [
                    FeePriority::Low,
                    FeePriority::Normal,
                    FeePriority::High,
                    FeePriority::Urgent,
                ] {
                    fees.insert(priority, self.calculate_fee(priority, mempool_size));
                }
                fees
            },
        }
    }
}

/// Fee statistics for monitoring and display
#[derive(Debug, Clone)]
pub struct FeeStatistics {
    pub base_fee: u64,
    pub max_fee: u64,
    pub current_congestion_multiplier: f64,
    pub mempool_size: usize,
    pub congestion_threshold: usize,
    pub estimated_fees: HashMap<FeePriority, u64>,
}

impl std::fmt::Display for FeeStatistics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Fee Statistics:")?;
        writeln!(f, "  Base Fee: {} coins", self.base_fee)?;
        writeln!(f, "  Max Fee: {} coins", self.max_fee)?;
        writeln!(f, "  Mempool Size: {} transactions", self.mempool_size)?;
        writeln!(
            f,
            "  Congestion Threshold: {} transactions",
            self.congestion_threshold
        )?;
        writeln!(
            f,
            "  Congestion Multiplier: {:.2}x",
            self.current_congestion_multiplier
        )?;
        writeln!(f, "  Estimated Fees:")?;
        for (priority, fee) in &self.estimated_fees {
            writeln!(f, "    {priority}: {fee} coins")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> DynamicFeeConfig {
        DynamicFeeConfig {
            base_fee: 1,
            max_fee: 10,
            congestion_threshold: 10,
            priority_multipliers: DynamicFeeConfig::default_priority_multipliers(),
            coinbase_reward: crate::core::INITIAL_BLOCK_REWARD,
        }
    }

    #[test]
    fn test_dynamic_fee_calculation_no_congestion() {
        let calculator = DynamicFeeCalculator::new(create_test_config()).unwrap();

        // No congestion, should use base fee with priority multipliers
        assert_eq!(calculator.calculate_fee(FeePriority::Low, 5), 1); // 0.5 * 1 = 0.5, rounded up to base
        assert_eq!(calculator.calculate_fee(FeePriority::Normal, 5), 1);
        assert_eq!(calculator.calculate_fee(FeePriority::High, 5), 2);
        assert_eq!(calculator.calculate_fee(FeePriority::Urgent, 5), 3);
    }

    #[test]
    fn test_dynamic_fee_calculation_with_congestion() {
        let calculator = DynamicFeeCalculator::new(create_test_config()).unwrap();

        // High congestion should increase fees
        let fee_no_congestion = calculator.calculate_fee(FeePriority::Normal, 5);
        let fee_with_congestion = calculator.calculate_fee(FeePriority::Normal, 30);

        assert!(fee_with_congestion > fee_no_congestion);
    }

    #[test]
    fn test_fee_caps() {
        let calculator = DynamicFeeCalculator::new(create_test_config()).unwrap();

        // Should not exceed max fee
        let fee = calculator.calculate_fee(FeePriority::Urgent, 1000);
        assert!(fee <= 10);

        // Should not go below base fee
        let fee = calculator.calculate_fee(FeePriority::Low, 0);
        assert!(fee >= 1);
    }

    #[test]
    fn test_congestion_multiplier() {
        let calculator = DynamicFeeCalculator::new(create_test_config()).unwrap();

        // No congestion
        assert_eq!(calculator.calculate_congestion_multiplier(5), 1.0);

        // Some congestion
        let multiplier = calculator.calculate_congestion_multiplier(20);
        assert!(multiplier > 1.0 && multiplier <= 3.0);

        // High congestion should cap at 3x
        let multiplier = calculator.calculate_congestion_multiplier(1000);
        assert_eq!(multiplier, 3.0);
    }

    #[test]
    fn test_config_validation() {
        // Valid config
        assert!(create_test_config().validate().is_ok());

        // Invalid configs
        let mut invalid_config = create_test_config();
        invalid_config.base_fee = 0;
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = create_test_config();
        invalid_config.max_fee = 0;
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_fee_validation() {
        let calculator = DynamicFeeCalculator::new(create_test_config()).unwrap();

        let expected_fee = calculator.calculate_fee(FeePriority::Normal, 10);

        // Exact fee should be valid
        assert!(calculator
            .validate_fee(expected_fee, FeePriority::Normal, 10)
            .is_ok());

        // Fee within tolerance should be valid
        let tolerance = (expected_fee as f64 * 0.05) as u64; // 5% tolerance
        assert!(calculator
            .validate_fee(expected_fee + tolerance, FeePriority::Normal, 10)
            .is_ok());

        // Fee outside tolerance should be invalid
        let large_deviation = expected_fee * 2;
        assert!(calculator
            .validate_fee(large_deviation, FeePriority::Normal, 10)
            .is_err());
    }

    #[test]
    fn test_priority_parsing() {
        assert_eq!("low".parse::<FeePriority>().unwrap(), FeePriority::Low);
        assert_eq!(
            "normal".parse::<FeePriority>().unwrap(),
            FeePriority::Normal
        );
        assert_eq!("high".parse::<FeePriority>().unwrap(), FeePriority::High);
        assert_eq!(
            "urgent".parse::<FeePriority>().unwrap(),
            FeePriority::Urgent
        );

        assert!("invalid".parse::<FeePriority>().is_err());
    }

    #[test]
    fn test_fee_statistics() {
        let calculator = DynamicFeeCalculator::new(create_test_config()).unwrap();
        let stats = calculator.get_fee_statistics(15);

        assert_eq!(stats.base_fee, 1);
        assert_eq!(stats.max_fee, 10);
        assert_eq!(stats.mempool_size, 15);
        assert!(stats.current_congestion_multiplier > 1.0);
        assert_eq!(stats.estimated_fees.len(), 4);
    }
}
