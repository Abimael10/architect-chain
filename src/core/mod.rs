//! Core blockchain functionality
//!
//! This module contains the fundamental blockchain components including
//! blocks, transactions, blockchain management, and proof-of-work consensus.

pub mod block;
pub mod blockchain;
pub mod difficulty;
pub mod fees;
pub mod merkle;
pub mod monetary;
pub mod proof_of_work;
pub mod transaction;

pub use block::Block;
pub use blockchain::{Blockchain, BlockchainIterator};
pub use difficulty::DifficultyAdjustment;
pub use fees::{DynamicFeeConfig, FeeCalculator, FeeMode, FeePriority, FeeStatistics};
pub use merkle::{MerkleProof, MerkleTree, ProofElement};
pub use monetary::{
    DEFAULT_TRANSACTION_FEE, INITIAL_BLOCK_REWARD, MIN_TRANSACTION_FEE, SATOSHIS_PER_COIN,
};
pub use proof_of_work::ProofOfWork;
pub use transaction::{TXInput, TXOutput, Transaction};
