use crate::core::Block;
use crate::error::{BlockchainError, Result};
use log::info;

// Difficulty adjustment constants
const TARGET_BLOCK_TIME: u64 = 120_000; // 2 minutes in milliseconds
const DIFFICULTY_ADJUSTMENT_PERIOD: usize = 10; // Adjust every 10 blocks
const INITIAL_DIFFICULTY: u32 = 4; // Starting difficulty
const MIN_DIFFICULTY: u32 = 1; // Minimum difficulty
const MAX_DIFFICULTY: u32 = 12; // Maximum difficulty

/// Difficulty adjustment algorithm for maintaining consistent block times
pub struct DifficultyAdjustment;

impl DifficultyAdjustment {
    /// Calculate the next difficulty based on recent block times
    pub fn calculate_next_difficulty(
        recent_blocks: &[Block],
        current_height: usize,
    ) -> Result<u32> {
        // Genesis block and early blocks use initial difficulty
        if current_height < DIFFICULTY_ADJUSTMENT_PERIOD {
            return Ok(INITIAL_DIFFICULTY);
        }

        // Only adjust difficulty at specific intervals
        if current_height % DIFFICULTY_ADJUSTMENT_PERIOD != 0 {
            // Return the difficulty of the most recent block
            return Ok(recent_blocks
                .last()
                .map(|block| block.get_difficulty())
                .unwrap_or(INITIAL_DIFFICULTY));
        }

        // Need exactly DIFFICULTY_ADJUSTMENT_PERIOD blocks for calculation
        if recent_blocks.len() != DIFFICULTY_ADJUSTMENT_PERIOD {
            return Err(BlockchainError::InvalidBlock(format!(
                "Need {} blocks for difficulty adjustment, got {}",
                DIFFICULTY_ADJUSTMENT_PERIOD,
                recent_blocks.len()
            )));
        }

        let actual_time_span = Self::calculate_time_span(recent_blocks)?;
        let target_time_span = TARGET_BLOCK_TIME * DIFFICULTY_ADJUSTMENT_PERIOD as u64;
        let current_difficulty = recent_blocks
            .last()
            .expect("Recent blocks should not be empty at this point")
            .get_difficulty();

        let new_difficulty =
            Self::adjust_difficulty(current_difficulty, actual_time_span, target_time_span);

        info!("Difficulty adjustment at height {current_height}: {current_difficulty} -> {new_difficulty} (actual: {actual_time_span}ms, target: {target_time_span}ms)");

        Ok(new_difficulty)
    }

    /// Calculate the time span between the first and last block
    fn calculate_time_span(blocks: &[Block]) -> Result<u64> {
        if blocks.len() < 2 {
            return Err(BlockchainError::InvalidBlock(
                "Need at least 2 blocks to calculate time span".to_string(),
            ));
        }

        let first_timestamp = blocks
            .first()
            .expect("Blocks should not be empty for time span calculation")
            .get_timestamp();
        let last_timestamp = blocks
            .last()
            .expect("Blocks should not be empty for time span calculation")
            .get_timestamp();

        if last_timestamp <= first_timestamp {
            return Err(BlockchainError::InvalidBlock(
                "Invalid block timestamps: last block is not newer than first".to_string(),
            ));
        }

        Ok((last_timestamp - first_timestamp) as u64)
    }

    /// Adjust difficulty based on actual vs target time
    fn adjust_difficulty(current_difficulty: u32, actual_time: u64, target_time: u64) -> u32 {
        // Calculate the ratio of actual time to target time
        let time_ratio = actual_time as f64 / target_time as f64;

        // Adjust difficulty based on time ratio
        let new_difficulty = if time_ratio < 0.5 {
            // Blocks are being mined too fast - increase difficulty
            current_difficulty + 2
        } else if time_ratio < 0.75 {
            // Blocks are being mined a bit too fast - increase difficulty slightly
            current_difficulty + 1
        } else if time_ratio > 2.0 {
            // Blocks are being mined too slow - decrease difficulty significantly
            current_difficulty.saturating_sub(2)
        } else if time_ratio > 1.5 {
            // Blocks are being mined a bit too slow - decrease difficulty slightly
            current_difficulty.saturating_sub(1)
        } else {
            // Time is within acceptable range - keep current difficulty
            current_difficulty
        };

        // Clamp difficulty to valid range
        new_difficulty.clamp(MIN_DIFFICULTY, MAX_DIFFICULTY)
    }

    /// Get the initial difficulty for genesis block
    pub fn get_initial_difficulty() -> u32 {
        INITIAL_DIFFICULTY
    }

    /// Get the adjustment period
    pub fn get_adjustment_period() -> usize {
        DIFFICULTY_ADJUSTMENT_PERIOD
    }

    /// Get the target block time in milliseconds
    pub fn get_target_block_time() -> u64 {
        TARGET_BLOCK_TIME
    }

    /// Validate that a difficulty value is within acceptable bounds
    pub fn validate_difficulty(difficulty: u32) -> Result<()> {
        if !(MIN_DIFFICULTY..=MAX_DIFFICULTY).contains(&difficulty) {
            return Err(BlockchainError::InvalidBlock(format!("Difficulty {difficulty} is outside valid range [{MIN_DIFFICULTY}, {MAX_DIFFICULTY}]")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_block(height: usize, timestamp: i64, difficulty: u32) -> Block {
        // Create a dummy transaction for the test block
        use crate::core::Transaction;

        let dummy_tx = Transaction::new_coinbase_tx("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa")
            .expect("Failed to create test transaction");

        // Use the test constructor with custom timestamp
        Block::new_test_block(
            timestamp,
            "test_prev_hash".to_string(),
            &[dummy_tx],
            height,
            difficulty,
        )
        .expect("Failed to create test block")
    }

    #[test]
    fn test_initial_difficulty() {
        let result = DifficultyAdjustment::calculate_next_difficulty(&[], 0).unwrap();
        assert_eq!(result, INITIAL_DIFFICULTY);
    }

    #[test]
    fn test_difficulty_adjustment_fast_blocks() {
        let blocks = vec![
            create_test_block(0, 0, 4),
            create_test_block(1, 10000, 4), // 10 seconds
            create_test_block(2, 20000, 4), // 10 seconds
            create_test_block(3, 30000, 4), // 10 seconds
            create_test_block(4, 40000, 4), // 10 seconds
            create_test_block(5, 50000, 4), // 10 seconds
            create_test_block(6, 60000, 4), // 10 seconds
            create_test_block(7, 70000, 4), // 10 seconds
            create_test_block(8, 80000, 4), // 10 seconds
            create_test_block(9, 90000, 4), // 10 seconds
        ];

        // Total time: 90 seconds, target: 1200 seconds (10 * 120)
        // Ratio: 90/1200 = 0.075 < 0.5, should increase difficulty by 2
        let result = DifficultyAdjustment::calculate_next_difficulty(&blocks, 10).unwrap();
        assert_eq!(result, 6); // 4 + 2
    }

    #[test]
    fn test_difficulty_adjustment_slow_blocks() {
        let blocks = vec![
            create_test_block(0, 0, 4),
            create_test_block(1, 200_000, 4),   // 200 seconds
            create_test_block(2, 400_000, 4),   // 200 seconds
            create_test_block(3, 600_000, 4),   // 200 seconds
            create_test_block(4, 800_000, 4),   // 200 seconds
            create_test_block(5, 1_000_000, 4), // 200 seconds
            create_test_block(6, 1_200_000, 4), // 200 seconds
            create_test_block(7, 1_400_000, 4), // 200 seconds
            create_test_block(8, 1_600_000, 4), // 200 seconds
            create_test_block(9, 1_800_000, 4), // 200 seconds
        ];

        // Total time: 1800 seconds, target: 1200 seconds (10 * 120)
        // Ratio: 1800/1200 = 1.5, exactly at boundary, should keep current difficulty
        let result = DifficultyAdjustment::calculate_next_difficulty(&blocks, 10).unwrap();
        assert_eq!(result, 4); // No change at exactly 1.5
    }

    #[test]
    fn test_difficulty_bounds() {
        // Test minimum difficulty bound
        let blocks = vec![
            create_test_block(0, 0, MIN_DIFFICULTY),
            create_test_block(1, 500_000, MIN_DIFFICULTY), // Very slow blocks
            create_test_block(2, 1_000_000, MIN_DIFFICULTY),
            create_test_block(3, 1_500_000, MIN_DIFFICULTY),
            create_test_block(4, 2_000_000, MIN_DIFFICULTY),
            create_test_block(5, 2_500_000, MIN_DIFFICULTY),
            create_test_block(6, 3_000_000, MIN_DIFFICULTY),
            create_test_block(7, 3_500_000, MIN_DIFFICULTY),
            create_test_block(8, 4_000_000, MIN_DIFFICULTY),
            create_test_block(9, 4_500_000, MIN_DIFFICULTY),
        ];

        let result = DifficultyAdjustment::calculate_next_difficulty(&blocks, 10).unwrap();
        assert_eq!(result, MIN_DIFFICULTY); // Should not go below minimum

        // Test maximum difficulty bound
        let blocks = vec![
            create_test_block(0, 0, MAX_DIFFICULTY),
            create_test_block(1, 1000, MAX_DIFFICULTY), // Very fast blocks
            create_test_block(2, 2000, MAX_DIFFICULTY),
            create_test_block(3, 3000, MAX_DIFFICULTY),
            create_test_block(4, 4000, MAX_DIFFICULTY),
            create_test_block(5, 5000, MAX_DIFFICULTY),
            create_test_block(6, 6000, MAX_DIFFICULTY),
            create_test_block(7, 7000, MAX_DIFFICULTY),
            create_test_block(8, 8000, MAX_DIFFICULTY),
            create_test_block(9, 9000, MAX_DIFFICULTY),
        ];

        let result = DifficultyAdjustment::calculate_next_difficulty(&blocks, 10).unwrap();
        assert_eq!(result, MAX_DIFFICULTY); // Should not go above maximum
    }
}
