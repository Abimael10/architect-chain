//! # Architect Chain - My Complete Blockchain Implementation
//!
//! This is my fully functional blockchain that I built from scratch in Rust.
//! When I come back to this code, here's what I need to remember:
//!
//! ## What I Built
//! - **Complete Blockchain**: Full proof-of-work consensus with dynamic difficulty
//! - **UTXO Model**: Bitcoin-compatible transaction system with digital signatures
//! - **P2P Network**: TCP-based networking with peer discovery and block sync
//! - **Wallet System**: ECDSA P-256 key management with Bitcoin-style addresses
//! - **Fee System**: Both fixed and dynamic fee calculation with priority levels
//! - **Multi-Node**: Isolated databases allowing multiple nodes on one machine
//!
//! ## How I Organized My Code
//! - `core/`: The heart of my blockchain (blocks, transactions, mining, consensus)
//! - `wallet/`: Key management, address generation, transaction signing
//! - `network/`: P2P communication, peer discovery, message handling
//! - `storage/`: Database operations, UTXO indexing, memory pool
//! - `config/`: Configuration management and feature flags
//! - `utils/`: Cryptographic functions and utility helpers
//! - `cli/`: Command-line interface for all blockchain operations
//!
//! ## Key Design Decisions I Made
//! - Used Sled embedded database for simplicity and reliability
//! - Followed Bitcoin's UTXO model for transaction compatibility
//! - Implemented ECDSA P-256 for modern cryptographic security
//! - Built modular architecture for easy testing and extension
//! - Added comprehensive error handling throughout
//!
//! ## When I Need to Understand Something
//! 1. Start with `main.rs` to see the CLI commands
//! 2. Look at `core/blockchain.rs` for the main blockchain logic
//! 3. Check `core/transaction.rs` for how value transfers work
//! 4. Review `network/server.rs` for P2P communication
//! 5. Examine `wallet/wallet.rs` for key management
//!
//! Remember: I built this to be educational but production-quality!
//! Every component has comprehensive tests and proper error handling.

pub mod cli;
pub mod config;
pub mod core;
pub mod error;
pub mod network;
pub mod storage;
pub mod utils;
pub mod wallet;

#[cfg(test)]
pub mod testnet;

// Re-export commonly used types for convenience
pub use cli::{Command, Opt};
pub use config::{Config, GLOBAL_CONFIG};
pub use core::{
    Block, Blockchain, DynamicFeeConfig, FeeCalculator, FeeMode, FeePriority, FeeStatistics,
    ProofOfWork, TXInput, TXOutput, Transaction,
};
pub use error::{BlockchainError, Result};
pub use network::{send_tx, Node, Nodes, Server, SimplePeerManager, CENTRAL_NODE};
pub use storage::{BlockInTransit, MemoryPool, UTXOSet};
pub use utils::{
    base58_decode, base58_encode, current_timestamp, ecdsa_p256_sha256_sign_digest,
    ecdsa_p256_sha256_sign_verify, new_key_pair, ripemd160_digest, sha256_digest,
};
pub use wallet::{
    convert_address, hash_pub_key, validate_address, Wallet, Wallets, ADDRESS_CHECK_SUM_LEN,
};
