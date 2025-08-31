//! Utility functions and helpers
//!
//! This module contains cryptographic utilities, encoding functions,
//! and other helper functions used throughout the blockchain.

pub mod crypto;
pub mod serialization;

pub use crypto::{
    base58_decode, base58_encode, current_timestamp, ecdsa_p256_sha256_sign_digest,
    ecdsa_p256_sha256_sign_verify, new_key_pair, ripemd160_digest, sha256_digest,
};

pub use serialization::{deserialize, serialize};
