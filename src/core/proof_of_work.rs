use crate::core::Block;
use crate::utils::sha256_digest;
use data_encoding::HEXLOWER;
use num_bigint::{BigInt, Sign};
use std::borrow::Borrow;
use std::ops::ShlAssign;

pub struct ProofOfWork {
    block: Block,
    target: BigInt,
    difficulty: u32,
}

// Removed hardcoded TARGET_BITS - now using dynamic difficulty

const MAX_NONCE: i64 = i64::MAX;

impl ProofOfWork {
    pub fn new_proof_of_work(block: Block) -> ProofOfWork {
        let difficulty = block.get_difficulty();
        let mut target = BigInt::from(1);
        target.shl_assign(256 - difficulty);
        ProofOfWork {
            block,
            target,
            difficulty,
        }
    }

    /// Validate proof-of-work for a block
    pub fn validate(block: &Block) -> bool {
        let pow = ProofOfWork::new_proof_of_work(block.clone());
        let data = pow.prepare_data(block.get_nonce());
        let hash = sha256_digest(data.as_slice());
        let hash_int = BigInt::from_bytes_be(Sign::Plus, hash.as_slice());

        // Check if hash meets difficulty target
        hash_int < pow.target
    }

    fn prepare_data(&self, nonce: i64) -> Vec<u8> {
        let pre_block_hash = self.block.get_pre_block_hash();
        let merkle_root = self.block.get_merkle_root(); // Use correct Merkle root!
        let timestamp = self.block.get_timestamp();
        let height = self.block.get_height();
        let mut data_bytes = vec![];
        data_bytes.extend(pre_block_hash.as_bytes());
        data_bytes.extend(merkle_root); // Proper Merkle root
        data_bytes.extend(timestamp.to_be_bytes());
        data_bytes.extend(height.to_be_bytes()); // Include height for completeness
        data_bytes.extend(self.difficulty.to_be_bytes());
        data_bytes.extend(nonce.to_be_bytes());
        data_bytes
    }

    pub fn run(&self) -> (i64, String) {
        let mut nonce = 0;
        let mut hash = Vec::new();
        println!("Mining the block");
        while nonce < MAX_NONCE {
            let data = self.prepare_data(nonce);
            hash = sha256_digest(data.as_slice());
            let hash_int = BigInt::from_bytes_be(Sign::Plus, hash.as_slice());

            if hash_int.lt(self.target.borrow()) {
                println!("{}", HEXLOWER.encode(hash.as_slice()));
                break;
            }
            nonce += 1;
        }
        println!();
        (nonce, HEXLOWER.encode(hash.as_slice()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Transaction;

    fn create_test_block(difficulty: u32) -> Block {
        let test_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
        let coinbase_tx = Transaction::new_coinbase_tx(test_address).unwrap();

        Block::new_block("None".to_string(), &[coinbase_tx], 0, difficulty).unwrap()
    }

    #[test]
    fn test_proof_of_work_creation() {
        let block = create_test_block(4);
        let pow = ProofOfWork::new_proof_of_work(block.clone());

        assert_eq!(pow.difficulty, block.get_difficulty());
        assert!(pow.target > BigInt::from(0));
    }

    #[test]
    fn test_proof_of_work_validation_valid_block() {
        let block = create_test_block(1); // Easy difficulty for fast test

        // Block should have valid proof of work after mining
        assert!(ProofOfWork::validate(&block));
    }

    #[test]
    fn test_proof_of_work_validation_invalid_block() {
        // Create a block with wrong previous hash to make it invalid
        let test_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
        let coinbase_tx = Transaction::new_coinbase_tx(test_address).unwrap();

        let invalid_block =
            Block::new_block("wrong_previous_hash".to_string(), &[coinbase_tx], 0, 1).unwrap();

        // This block should still have valid PoW since it was mined properly
        // But it would fail blockchain validation due to wrong previous hash
        assert!(ProofOfWork::validate(&invalid_block));
    }

    #[test]
    fn test_proof_of_work_difficulty_scaling() {
        // Test that higher difficulty requires more work
        let easy_block = create_test_block(1);
        let hard_block = create_test_block(2);

        // Both should be valid
        assert!(ProofOfWork::validate(&easy_block));
        assert!(ProofOfWork::validate(&hard_block));

        // Create PoW instances to check targets
        let easy_pow = ProofOfWork::new_proof_of_work(easy_block);
        let hard_pow = ProofOfWork::new_proof_of_work(hard_block);

        // Higher difficulty should have smaller target
        assert!(hard_pow.target < easy_pow.target);
    }

    #[test]
    fn test_prepare_data_consistency() {
        let block = create_test_block(2);
        let pow = ProofOfWork::new_proof_of_work(block);

        // Prepare data should be consistent for same inputs
        let data1 = pow.prepare_data(12345);
        let data2 = pow.prepare_data(12345);
        assert_eq!(data1, data2);

        // Different nonces should produce different data
        let data3 = pow.prepare_data(54321);
        assert_ne!(data1, data3);
    }

    #[test]
    fn test_prepare_data_includes_all_fields() {
        let block = create_test_block(2);
        let pow = ProofOfWork::new_proof_of_work(block.clone());

        let data = pow.prepare_data(12345);

        // Data should include all block fields
        // We can't easily test the exact content, but we can test length
        let expected_min_length = block.get_pre_block_hash().len() + // prev hash
            block.get_merkle_root().len() +    // merkle root
            8 +  // timestamp (i64)
            8 +  // height (usize as u64)
            4 +  // difficulty (u32)
            8; // nonce (i64)

        assert!(data.len() >= expected_min_length);
    }
}
