/// Educational blockchain monetary system
///
/// This module defines the monetary constants and utilities for the educational blockchain.
/// It follows Bitcoin's satoshi-based system but with educational-friendly values.
///
/// ## Monetary Units
/// - **Satoshi**: The smallest unit (like Bitcoin)
/// - **Coin**: 100,000,000 satoshis (like Bitcoin)
/// - **Block Reward**: 50 coins initially (will halve every 210,000 blocks)
/// - **Minimum Fee**: 1,000 satoshis (0.00001 coins)
///
/// ## Educational Considerations
/// - Values are larger than real Bitcoin for easier understanding
/// - Block rewards are substantial to demonstrate mining incentives
/// - Fees are meaningful but not prohibitive for learning
///
/// Number of satoshis in one coin (same as Bitcoin)
pub const SATOSHIS_PER_COIN: u64 = 100_000_000;

/// Initial block reward in satoshis (50 coins)
/// This is the same as Bitcoin's original block reward
pub const INITIAL_BLOCK_REWARD: u64 = 50 * SATOSHIS_PER_COIN;

/// Minimum transaction fee in satoshis (0.00001 coins)
/// This is reasonable for educational purposes
pub const MIN_TRANSACTION_FEE: u64 = 1_000;

/// Default transaction fee in satoshis (0.0001 coins)
/// This is the default fee for simple transactions
pub const DEFAULT_TRANSACTION_FEE: u64 = 10_000;

/// Maximum transaction fee in satoshis (0.01 coins)
/// This prevents accidentally setting extremely high fees
pub const MAX_TRANSACTION_FEE: u64 = 1_000_000;

/// Dust threshold in satoshis (0.00000546 coins)
/// Outputs smaller than this are considered "dust" and discouraged
pub const DUST_THRESHOLD: u64 = 546;

/// Educational constants for easy understanding
pub mod educational {
    use super::*;

    /// Small amount for testing (0.001 coins)
    pub const SMALL_AMOUNT: u64 = SATOSHIS_PER_COIN / 1_000;

    /// Medium amount for testing (0.1 coins)
    pub const MEDIUM_AMOUNT: u64 = SATOSHIS_PER_COIN / 10;

    /// Large amount for testing (10 coins)
    pub const LARGE_AMOUNT: u64 = 10 * SATOSHIS_PER_COIN;
}

/// Utility functions for monetary conversions
pub mod conversions {
    use super::*;

    /// Convert coins to satoshis
    ///
    /// # Examples
    /// ```
    /// use architect_chain::core::monetary::conversions::coins_to_satoshis;
    /// assert_eq!(coins_to_satoshis(1.0), 100_000_000);
    /// assert_eq!(coins_to_satoshis(0.5), 50_000_000);
    /// ```
    pub fn coins_to_satoshis(coins: f64) -> u64 {
        (coins * SATOSHIS_PER_COIN as f64) as u64
    }

    /// Convert satoshis to coins
    ///
    /// # Examples
    /// ```
    /// use architect_chain::core::monetary::conversions::satoshis_to_coins;
    /// assert_eq!(satoshis_to_coins(100_000_000), 1.0);
    /// assert_eq!(satoshis_to_coins(50_000_000), 0.5);
    /// ```
    pub fn satoshis_to_coins(satoshis: u64) -> f64 {
        satoshis as f64 / SATOSHIS_PER_COIN as f64
    }

    /// Format satoshis as a human-readable string
    ///
    /// # Examples
    /// ```
    /// use architect_chain::core::monetary::conversions::format_satoshis;
    /// assert_eq!(format_satoshis(100_000_000), "1.00000000 coins");
    /// assert_eq!(format_satoshis(1_000), "0.00001000 coins");
    /// ```
    pub fn format_satoshis(satoshis: u64) -> String {
        format!("{:.8} coins", satoshis_to_coins(satoshis))
    }

    /// Validate that an amount is above the dust threshold
    pub fn is_above_dust_threshold(amount: u64) -> bool {
        amount >= DUST_THRESHOLD
    }

    /// Validate that a fee is within reasonable bounds
    pub fn is_valid_fee(fee: u64) -> bool {
        (MIN_TRANSACTION_FEE..=MAX_TRANSACTION_FEE).contains(&fee)
    }
}

#[cfg(test)]
mod tests {
    use super::conversions::*;
    use super::*;

    #[test]
    fn test_monetary_constants() {
        assert_eq!(SATOSHIS_PER_COIN, 100_000_000);
        assert_eq!(INITIAL_BLOCK_REWARD, 50 * SATOSHIS_PER_COIN);
        // Validate fee ordering at compile time
        const _: () = assert!(MIN_TRANSACTION_FEE < DEFAULT_TRANSACTION_FEE);
        const _: () = assert!(DEFAULT_TRANSACTION_FEE < MAX_TRANSACTION_FEE);
    }

    #[test]
    fn test_conversions() {
        // Test coins to satoshis
        assert_eq!(coins_to_satoshis(1.0), SATOSHIS_PER_COIN);
        assert_eq!(coins_to_satoshis(0.5), SATOSHIS_PER_COIN / 2);
        assert_eq!(
            coins_to_satoshis(2.5),
            SATOSHIS_PER_COIN * 2 + SATOSHIS_PER_COIN / 2
        );

        // Test satoshis to coins
        assert_eq!(satoshis_to_coins(SATOSHIS_PER_COIN), 1.0);
        assert_eq!(satoshis_to_coins(SATOSHIS_PER_COIN / 2), 0.5);

        // Test round trip
        let original = 1.23456789;
        let satoshis = coins_to_satoshis(original);
        let back_to_coins = satoshis_to_coins(satoshis);
        assert!((original - back_to_coins).abs() < 0.00000001);
    }

    #[test]
    fn test_validation() {
        // Test dust threshold
        assert!(!is_above_dust_threshold(DUST_THRESHOLD - 1));
        assert!(is_above_dust_threshold(DUST_THRESHOLD));
        assert!(is_above_dust_threshold(DUST_THRESHOLD + 1));

        // Test fee validation
        assert!(!is_valid_fee(MIN_TRANSACTION_FEE - 1));
        assert!(is_valid_fee(MIN_TRANSACTION_FEE));
        assert!(is_valid_fee(DEFAULT_TRANSACTION_FEE));
        assert!(is_valid_fee(MAX_TRANSACTION_FEE));
        assert!(!is_valid_fee(MAX_TRANSACTION_FEE + 1));
    }

    #[test]
    fn test_formatting() {
        assert_eq!(format_satoshis(SATOSHIS_PER_COIN), "1.00000000 coins");
        assert_eq!(format_satoshis(SATOSHIS_PER_COIN / 2), "0.50000000 coins");
        assert_eq!(format_satoshis(1_000), "0.00001000 coins");
    }

    #[test]
    fn test_educational_constants() {
        use educational::*;

        assert_eq!(SMALL_AMOUNT, SATOSHIS_PER_COIN / 1_000);
        assert_eq!(MEDIUM_AMOUNT, SATOSHIS_PER_COIN / 10);
        assert_eq!(LARGE_AMOUNT, 10 * SATOSHIS_PER_COIN);

        // Ensure they're in logical order at compile time
        const _: () = assert!(SMALL_AMOUNT < MEDIUM_AMOUNT);
        const _: () = assert!(MEDIUM_AMOUNT < LARGE_AMOUNT);
    }
}
