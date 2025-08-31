use crate::error::{BlockchainError, Result};
use log::info;

/// Legacy fixed fee calculator for backward compatibility
#[derive(Debug, Clone)]
pub struct FixedFeeCalculator {
    /// Fixed fee amount in coins
    pub fee_amount: u64,
    /// Base coinbase reward
    pub coinbase_reward: u64,
}

impl FixedFeeCalculator {
    /// Create a new fixed fee calculator
    pub fn new(fee_amount: u64) -> Self {
        Self {
            fee_amount,
            coinbase_reward: crate::core::INITIAL_BLOCK_REWARD, // Use proper monetary constant
        }
    }

    /// Calculate fee (always returns the fixed amount)
    pub fn calculate_fee(
        &self,
        _transaction_size: usize,
        _priority: Option<crate::core::fees::FeePriority>,
    ) -> u64 {
        info!("Using fixed fee: {} coins", self.fee_amount);
        self.fee_amount
    }

    /// Validate fee amount (always valid for fixed fees)
    pub fn validate_fee(&self, fee: u64) -> Result<()> {
        if fee == self.fee_amount {
            Ok(())
        } else {
            Err(BlockchainError::Transaction(format!(
                "Invalid fee: expected {}, got {}",
                self.fee_amount, fee
            )))
        }
    }

    /// Calculate coinbase reward with collected fees
    pub fn calculate_coinbase_reward(&self, collected_fees: u64) -> u64 {
        self.coinbase_reward + collected_fees
    }

    /// Get the fixed fee amount
    pub fn get_fee_amount(&self) -> u64 {
        self.fee_amount
    }

    /// Set a new fixed fee amount
    pub fn set_fee_amount(&mut self, amount: u64) {
        self.fee_amount = amount;
        info!("Updated fixed fee to: {amount} coins");
    }
}

impl Default for FixedFeeCalculator {
    fn default() -> Self {
        Self::new(1) // Default 1 coin fee for educational purposes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_fee_calculation() {
        let calculator = FixedFeeCalculator::new(2);

        // Should always return fixed amount regardless of parameters
        assert_eq!(calculator.calculate_fee(100, None), 2);
        assert_eq!(calculator.calculate_fee(1000, None), 2);
    }

    #[test]
    fn test_fee_validation() {
        let calculator = FixedFeeCalculator::new(1);

        assert!(calculator.validate_fee(1).is_ok());
        assert!(calculator.validate_fee(2).is_err());
    }

    #[test]
    fn test_coinbase_reward() {
        let calculator = FixedFeeCalculator::new(1);
        let reward = calculator.calculate_coinbase_reward(5);
        assert_eq!(reward, crate::core::INITIAL_BLOCK_REWARD + 5); // Base reward + 5 fees
    }

    #[test]
    fn test_fee_amount_update() {
        let mut calculator = FixedFeeCalculator::new(1);
        assert_eq!(calculator.get_fee_amount(), 1);

        calculator.set_fee_amount(3);
        assert_eq!(calculator.get_fee_amount(), 3);
    }
}
