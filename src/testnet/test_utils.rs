//! Test utilities for blockchain testing

use crate::core::{Block, Blockchain, Transaction};
use crate::error::Result;
use crate::wallet::Wallets;
use tempfile::TempDir;

/// Test configuration for blockchain testing
pub struct TestConfig {
    pub initial_difficulty: u32,
    pub block_time_target: u64,
    pub genesis_reward: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            initial_difficulty: 1,      // Easy difficulty for fast testing
            block_time_target: 1,       // 1 second for fast tests
            genesis_reward: 5000000000, // 50 coins
        }
    }
}

/// Create a temporary directory for testing
pub fn create_temp_dir() -> Result<TempDir> {
    tempfile::tempdir().map_err(|e| crate::error::BlockchainError::Io(e.to_string()))
}

/// Create a test blockchain with temporary storage
pub fn create_test_blockchain() -> Result<(Blockchain, TempDir)> {
    let temp_dir = create_temp_dir()?;
    let db_path = temp_dir.path().join("test_blockchain");

    let test_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"; // Test address
    let blockchain =
        Blockchain::create_blockchain_with_path(test_address, db_path.to_str().unwrap())?;

    Ok((blockchain, temp_dir))
}

/// Create multiple test blockchains for network testing
pub fn create_test_network(node_count: usize) -> Result<Vec<(Blockchain, TempDir)>> {
    let mut nodes = Vec::new();

    for i in 0..node_count {
        let temp_dir = create_temp_dir()?;
        let db_path = temp_dir.path().join(format!("test_node_{i}"));

        let test_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"; // Use valid test address
        let blockchain =
            Blockchain::create_blockchain_with_path(test_address, db_path.to_str().unwrap())?;

        nodes.push((blockchain, temp_dir));
    }

    Ok(nodes)
}

/// Create test wallets for testing
pub fn create_test_wallets(count: usize) -> Result<(Wallets, Vec<String>)> {
    let mut wallets = Wallets::new();
    let mut addresses = Vec::new();

    for _ in 0..count {
        let address = wallets.create_wallet()?;
        addresses.push(address);
    }

    Ok((wallets, addresses))
}

/// Create a test transaction
pub fn create_test_transaction(
    from: &str,
    to: &str,
    amount: u64,
    blockchain: &Blockchain,
) -> Result<Transaction> {
    use crate::storage::UTXOSet;

    let utxo_set = UTXOSet::new(blockchain.clone());
    Transaction::new_utxo_transaction(from, to, amount, &utxo_set)
}

/// Mine a test block with custom difficulty
pub fn mine_test_block(
    blockchain: &Blockchain,
    transactions: &[Transaction],
    miner_address: &str,
) -> Result<Block> {
    blockchain.mine_block_with_fees(transactions, miner_address)
}

/// Validate blockchain integrity
pub fn validate_blockchain_integrity(blockchain: &Blockchain) -> Result<bool> {
    let mut iterator = blockchain.iterator();
    let mut prev_hash = "None".to_string();

    while let Some(block) = iterator.next() {
        // Check block linkage
        if block.get_pre_block_hash() != prev_hash {
            return Ok(false);
        }

        // Validate proof of work
        if !crate::core::ProofOfWork::validate(&block) {
            return Ok(false);
        }

        // Validate merkle root
        if !block.verify_merkle_root()? {
            return Ok(false);
        }

        prev_hash = block.get_hash().to_string();
    }

    Ok(true)
}

/// Create a fork scenario for testing
pub fn create_fork_scenario(
    blockchain: &Blockchain,
    fork_point: usize,
    fork_length: usize,
    miner_address: &str,
) -> Result<Vec<Block>> {
    let mut fork_blocks = Vec::new();

    // Get the block at fork point
    let mut iterator = blockchain.iterator();
    let mut current_block = None;
    let mut height = blockchain.get_best_height()?;

    // Navigate to fork point
    while height > fork_point {
        current_block = iterator.next();
        height -= 1;
    }

    if let Some(fork_base) = current_block {
        let mut prev_hash = fork_base.get_hash().to_string();

        // Create fork blocks
        for i in 0..fork_length {
            let coinbase_tx = Transaction::new_coinbase_tx(miner_address)?;
            let block = Block::new_block(
                prev_hash,
                &[coinbase_tx],
                fork_point + i + 1,
                1, // Easy difficulty for testing
            )?;

            prev_hash = block.get_hash().to_string();
            fork_blocks.push(block);
        }
    }

    Ok(fork_blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_blockchain() {
        let result = create_test_blockchain();
        assert!(result.is_ok());

        let (blockchain, _temp_dir) = result.unwrap();
        assert_eq!(blockchain.get_best_height().unwrap(), 0);
    }

    #[test]
    fn test_create_test_network() {
        let result = create_test_network(3);
        assert!(result.is_ok());

        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 3);

        for (blockchain, _) in nodes {
            assert_eq!(blockchain.get_best_height().unwrap(), 0);
        }
    }

    #[test]
    fn test_create_test_wallets() {
        let result = create_test_wallets(5);
        assert!(result.is_ok());

        let (_wallets, addresses) = result.unwrap();
        assert_eq!(addresses.len(), 5);

        // All addresses should be unique
        for i in 0..addresses.len() {
            for j in i + 1..addresses.len() {
                assert_ne!(addresses[i], addresses[j]);
            }
        }
    }

    #[test]
    fn test_validate_blockchain_integrity() {
        let (blockchain, _temp_dir) = create_test_blockchain().unwrap();
        let is_valid = validate_blockchain_integrity(&blockchain).unwrap();
        assert!(is_valid);
    }
}
