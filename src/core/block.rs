use crate::core::{MerkleTree, ProofOfWork, Transaction};
use crate::error::{BlockchainError, Result};
use crate::utils::{current_timestamp, deserialize, serialize};
use log::info;
use serde::{Deserialize, Serialize};
use sled::IVec;

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
}

impl From<Block> for IVec {
    fn from(b: Block) -> Self {
        let bytes =
            serialize(&b).expect("Block serialization should never fail for IVec conversion");
        Self::from(bytes)
    }
}
