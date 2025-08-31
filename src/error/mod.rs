//! Error handling for the blockchain
//!
//! This module provides comprehensive error types for all blockchain operations.

use std::fmt;

/// Result type alias for blockchain operations
pub type Result<T> = std::result::Result<T, BlockchainError>;

/// Comprehensive error types for blockchain operations
#[derive(Debug, Clone)]
pub enum BlockchainError {
    /// Database-related errors
    Database(String),
    /// Cryptographic operation errors
    Crypto(String),
    /// Network communication errors
    Network(String),
    /// Transaction validation errors
    Transaction(String),
    /// Wallet operation errors
    Wallet(String),
    /// Configuration errors
    Config(String),
    /// Serialization/deserialization errors
    Serialization(String),
    /// File I/O errors
    Io(String),
    /// Invalid address format
    InvalidAddress(String),
    /// Insufficient funds for transaction
    InsufficientFunds { required: u64, available: u64 },
    /// Block validation errors
    InvalidBlock(String),
    /// Mining errors
    Mining(String),
    /// Encryption/decryption errors
    Encryption(String),
}

impl fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockchainError::Database(msg) => write!(f, "Database error: {msg}"),
            BlockchainError::Crypto(msg) => write!(f, "Cryptographic error: {msg}"),
            BlockchainError::Network(msg) => write!(f, "Network error: {msg}"),
            BlockchainError::Transaction(msg) => write!(f, "Transaction error: {msg}"),
            BlockchainError::Wallet(msg) => write!(f, "Wallet error: {msg}"),
            BlockchainError::Config(msg) => write!(f, "Configuration error: {msg}"),
            BlockchainError::Serialization(msg) => write!(f, "Serialization error: {msg}"),
            BlockchainError::Io(msg) => write!(f, "I/O error: {msg}"),
            BlockchainError::InvalidAddress(addr) => write!(f, "Invalid address: {addr}"),
            BlockchainError::InsufficientFunds {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient funds: required {required}, available {available}"
                )
            }
            BlockchainError::InvalidBlock(msg) => write!(f, "Invalid block: {msg}"),
            BlockchainError::Mining(msg) => write!(f, "Mining error: {msg}"),
            BlockchainError::Encryption(msg) => write!(f, "Encryption error: {msg}"),
        }
    }
}

impl std::error::Error for BlockchainError {}

impl From<std::io::Error> for BlockchainError {
    fn from(err: std::io::Error) -> Self {
        BlockchainError::Io(err.to_string())
    }
}

impl From<sled::Error> for BlockchainError {
    fn from(err: sled::Error) -> Self {
        BlockchainError::Database(err.to_string())
    }
}

impl From<bincode::error::EncodeError> for BlockchainError {
    fn from(err: bincode::error::EncodeError) -> Self {
        BlockchainError::Serialization(err.to_string())
    }
}

impl From<bincode::error::DecodeError> for BlockchainError {
    fn from(err: bincode::error::DecodeError) -> Self {
        BlockchainError::Serialization(err.to_string())
    }
}
