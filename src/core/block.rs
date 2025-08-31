use crate::core::{MerkleTree, ProofOfWork, Transaction};
use crate::error::{BlockchainError, Result};
use crate::utils::{current_timestamp, deserialize, serialize};
use log::info;
use serde::{Deserialize, Serialize};
use sled::IVec;

// I need to set reasonable limits for my blockchain to prevent abuse
const MAX_BLOCK_SIZE: usize = 1_000_000; // 1MB maximum block size
const MAX_TRANSACTIONS_PER_BLOCK: usize = 4000; // Maximum transactions per block
const MAX_TRANSACTION_SIZE: usize = 100_000; // 100KB maximum transaction size
const MAX_FUTURE_TIME: i64 = 2 * 60 * 60; // 2 hours maximum future time
const MIN_COINBASE_MATURITY: usize = 100; // Coinbase outputs mature after 100 blocks

#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct Block {
    timestamp: i64,
    pre_block_hash: String,
    hash: String,
    transactions: Vec<Transaction>,
    nonce: i64,
    height: usize,
    difficulty: u32,      // Dynamic difficulty for this block
    merkle_root: Vec<u8>, // Merkle root of all transactions
}

impl Block {
    pub fn new_block(
        pre_block_hash: String,
        transactions: &[Transaction],
        height: usize,
        difficulty: u32,
    ) -> Result<Block> {
        if transactions.is_empty() {
            return Err(BlockchainError::InvalidBlock(
                "Block must contain at least one transaction".to_string(),
            ));
        }

        // I need to validate the block before creating it
        Self::validate_block_constraints(transactions)?;

        // Calculate Merkle root for the transactions
        let merkle_root = Self::calculate_merkle_root(transactions)?;

        let mut block = Block {
            timestamp: current_timestamp()?,
            pre_block_hash,
            hash: String::new(),
            transactions: transactions.to_vec(),
            nonce: 0,
            height,
            difficulty,
            merkle_root,
        };

        info!("Starting proof-of-work for block at height {height} with difficulty {difficulty}");
        let pow = ProofOfWork::new_proof_of_work(block.clone());
        let (nonce, hash) = pow.run();
        block.nonce = nonce;
        block.hash = hash.clone();
        info!("Proof-of-work completed for block: {hash} (difficulty: {difficulty})");

        Ok(block)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Block> {
        deserialize::<Block>(bytes)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        serialize(self)
    }

    pub fn get_transactions(&self) -> &[Transaction] {
        self.transactions.as_slice()
    }

    pub fn get_pre_block_hash(&self) -> String {
        self.pre_block_hash.clone()
    }

    pub fn get_hash(&self) -> &str {
        self.hash.as_str()
    }

    pub fn get_hash_bytes(&self) -> Vec<u8> {
        self.hash.as_bytes().to_vec()
    }

    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn get_difficulty(&self) -> u32 {
        self.difficulty
    }

    pub fn get_merkle_root(&self) -> &[u8] {
        &self.merkle_root
    }

    pub fn get_nonce(&self) -> i64 {
        self.nonce
    }

    pub fn hash_transactions(&self) -> Vec<u8> {
        let mut txhashs = vec![];
        for transaction in &self.transactions {
            txhashs.extend(transaction.get_id());
        }

        crate::utils::sha256_digest(txhashs.as_slice())
    }

    pub fn generate_genesis_block(transaction: &Transaction) -> Result<Block> {
        let transactions = vec![transaction.clone()];
        let genesis_difficulty = crate::core::DifficultyAdjustment::get_initial_difficulty();
        Block::new_block(String::from("None"), &transactions, 0, genesis_difficulty)
    }

    /// Create a test block with custom timestamp (for testing only)
    #[cfg(test)]
    pub fn new_test_block(
        timestamp: i64,
        pre_block_hash: String,
        transactions: &[Transaction],
        height: usize,
        difficulty: u32,
    ) -> Result<Block> {
        if transactions.is_empty() {
            return Err(BlockchainError::InvalidBlock(
                "Block must contain at least one transaction".to_string(),
            ));
        }

        // Calculate Merkle root for the transactions
        let merkle_root = Self::calculate_merkle_root(transactions)?;

        Ok(Block {
            timestamp,
            pre_block_hash,
            hash: "test_hash".to_string(), // Test hash
            transactions: transactions.to_vec(),
            nonce: 0,
            height,
            difficulty,
            merkle_root,
        })
    }

    /// Calculate Merkle root for a list of transactions
    fn calculate_merkle_root(transactions: &[Transaction]) -> Result<Vec<u8>> {
        let transaction_hashes: Vec<Vec<u8>> =
            transactions.iter().map(|tx| tx.get_id().to_vec()).collect();

        MerkleTree::calculate_merkle_root(&transaction_hashes)
    }

    /// Verify that the block's Merkle root matches its transactions
    pub fn verify_merkle_root(&self) -> Result<bool> {
        let calculated_root = Self::calculate_merkle_root(&self.transactions)?;
        Ok(calculated_root == self.merkle_root)
    }

    /// Generate a Merkle proof for a transaction in this block
    pub fn generate_merkle_proof(
        &self,
        transaction_index: usize,
    ) -> Result<crate::core::MerkleProof> {
        if transaction_index >= self.transactions.len() {
            return Err(BlockchainError::InvalidBlock(format!(
                "Transaction index {} out of bounds (max: {})",
                transaction_index,
                self.transactions.len() - 1
            )));
        }

        let merkle_tree = MerkleTree::new(&self.transactions)?;
        merkle_tree.generate_proof(transaction_index)
    }

    /// Verify a Merkle proof against this block's Merkle root
    pub fn verify_merkle_proof(&self, proof: &crate::core::MerkleProof) -> Result<bool> {
        if proof.merkle_root != self.merkle_root {
            return Ok(false);
        }

        MerkleTree::verify_proof(proof)
    }

    // I need to validate that a block meets all the constraints I've set
    fn validate_block_constraints(transactions: &[Transaction]) -> Result<()> {
        // Check transaction count limit
        if transactions.len() > MAX_TRANSACTIONS_PER_BLOCK {
            return Err(BlockchainError::InvalidBlock(format!(
                "Too many transactions in block: {} (max: {})",
                transactions.len(),
                MAX_TRANSACTIONS_PER_BLOCK
            )));
        }

        // Check individual transaction sizes and total block size
        let mut total_size = 0;
        for (i, transaction) in transactions.iter().enumerate() {
            let tx_size = transaction.serialize()?.len();
            
            // Check individual transaction size
            if tx_size > MAX_TRANSACTION_SIZE {
                return Err(BlockchainError::InvalidBlock(format!(
                    "Transaction {} too large: {} bytes (max: {} bytes)",
                    i, tx_size, MAX_TRANSACTION_SIZE
                )));
            }
            
            total_size += tx_size;
        }

        // Check total block size
        if total_size > MAX_BLOCK_SIZE {
            return Err(BlockchainError::InvalidBlock(format!(
                "Block too large: {} bytes (max: {} bytes)",
                total_size, MAX_BLOCK_SIZE
            )));
        }

        Ok(())
    }

    // I need to validate a complete block including timestamp and other rules
    pub fn validate_block(&self, prev_block_timestamp: Option<i64>) -> Result<bool> {
        // Validate timestamp
        if !self.validate_timestamp(prev_block_timestamp)? {
            return Ok(false);
        }

        // Validate block constraints
        Self::validate_block_constraints(&self.transactions)?;

        // Validate merkle root
        if !self.verify_merkle_root()? {
            log::error!("Block merkle root validation failed");
            return Ok(false);
        }

        // Validate proof of work
        if !ProofOfWork::validate(self) {
            log::error!("Block proof of work validation failed");
            return Ok(false);
        }

        // Validate that first transaction is coinbase (if any transactions)
        if !self.transactions.is_empty() && !self.transactions[0].is_coinbase() {
            log::error!("First transaction in block must be coinbase");
            return Ok(false);
        }

        // Validate that only first transaction is coinbase
        for (i, tx) in self.transactions.iter().enumerate() {
            if i > 0 && tx.is_coinbase() {
                log::error!("Only first transaction can be coinbase");
                return Ok(false);
            }
        }

        Ok(true)
    }

    // I need to validate the block timestamp to prevent time-based attacks
    fn validate_timestamp(&self, prev_block_timestamp: Option<i64>) -> Result<bool> {
        let current_time = current_timestamp()?;
        
        // Block timestamp cannot be too far in the future
        if self.timestamp > current_time + MAX_FUTURE_TIME {
            log::error!(
                "Block timestamp too far in future: {} (current: {}, max future: {})",
                self.timestamp, current_time, current_time + MAX_FUTURE_TIME
            );
            return Ok(false);
        }

        // Block timestamp must be after previous block (if provided)
        if let Some(prev_timestamp) = prev_block_timestamp {
            if self.timestamp <= prev_timestamp {
                log::error!(
                    "Block timestamp must be after previous block: {} <= {}",
                    self.timestamp, prev_timestamp
                );
                return Ok(false);
            }
        }

        Ok(true)
    }

    // I want to be able to get the block size for analysis
    pub fn get_block_size(&self) -> Result<usize> {
        Ok(self.serialize()?.len())
    }

    // I want to be able to get the total transaction fees in this block
    pub fn get_total_fees(&self) -> u64 {
        self.transactions
            .iter()
            .skip(1) // Skip coinbase transaction
            .map(|tx| tx.get_fee())
            .sum()
    }

    // I want to validate that coinbase reward is correct
    pub fn validate_coinbase_reward(&self, expected_reward: u64) -> Result<bool> {
        if self.transactions.is_empty() {
            return Err(BlockchainError::InvalidBlock(
                "Block has no transactions".to_string(),
            ));
        }

        let coinbase = &self.transactions[0];
        if !coinbase.is_coinbase() {
            return Err(BlockchainError::InvalidBlock(
                "First transaction is not coinbase".to_string(),
            ));
        }

        let coinbase_value = coinbase.get_output_value()?;
        if coinbase_value != expected_reward {
            log::error!(
                "Invalid coinbase reward: {} (expected: {})",
                coinbase_value, expected_reward
            );
            return Ok(false);
        }

        Ok(true)
    }
}

impl From<Block> for IVec {
    fn from(b: Block) -> Self {
        let bytes =
            serialize(&b).expect("Block serialization should never fail for IVec conversion");
        Self::from(bytes)
    }
}
