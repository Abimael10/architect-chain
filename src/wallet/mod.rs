//! Wallet management and cryptographic operations
//!
//! This module handles wallet creation, key management, address generation,
//! and cryptographic operations for the blockchain.

#[allow(clippy::module_inception)]
pub mod wallet;
pub mod wallets;

pub use wallet::{convert_address, hash_pub_key, validate_address, Wallet, ADDRESS_CHECK_SUM_LEN};
pub use wallets::{Wallets, WALLET_FILE};
