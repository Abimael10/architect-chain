//! Data storage and persistence
//!
//! This module manages data persistence including UTXO sets,
//! memory pools for pending transactions, blockchain data storage,
//! and encrypted storage capabilities.

pub mod encrypted;
pub mod memory_pool;
pub mod utxo_set;

pub use encrypted::{EncryptedWallets, WalletEncryptionConfig, WalletEncryptionSettings};
pub use memory_pool::{BlockInTransit, MemoryPool};
pub use utxo_set::UTXOSet;

use once_cell::sync::Lazy;

/// Global memory pool instance for fee calculation access
pub static GLOBAL_MEMORY_POOL: Lazy<MemoryPool> = Lazy::new(MemoryPool::new);
