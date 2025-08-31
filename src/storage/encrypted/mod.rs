//! Wallet encryption system for private key protection
//!
//! This module provides encryption capabilities specifically for wallet data,
//! focusing on protecting private keys and sensitive wallet information.
//!
//! Note: Blockchain data itself is public and doesn't need encryption.
//! Only private keys and wallet data require protection.

pub mod cipher;
pub mod wallet_encryption;

pub use cipher::{Aes256GcmCipher, EncryptionResult, SecureKey};
pub use wallet_encryption::{EncryptedWallets, WalletEncryptionConfig};

use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Simple encryption configuration for wallets only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletEncryptionSettings {
    /// Whether wallet encryption is enabled
    pub enabled: bool,
    /// Minimum password length
    pub min_password_length: usize,
    /// Whether to create encrypted backups
    pub backup_enabled: bool,
}

impl Default for WalletEncryptionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            min_password_length: 8,
            backup_enabled: true,
        }
    }
}

/// Generate cryptographically secure random bytes
pub fn generate_random_bytes(length: usize) -> Result<Vec<u8>> {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut bytes = vec![0u8; length];
    rng.fill_bytes(&mut bytes);
    Ok(bytes)
}

/// Securely clear sensitive data from memory
pub fn secure_clear(data: &mut [u8]) {
    use zeroize::Zeroize;
    data.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_encryption_settings_default() {
        let settings = WalletEncryptionSettings::default();
        assert!(!settings.enabled);
        assert_eq!(settings.min_password_length, 8);
        assert!(settings.backup_enabled);
    }

    #[test]
    fn test_generate_random_bytes() {
        let bytes1 = generate_random_bytes(32).unwrap();
        let bytes2 = generate_random_bytes(32).unwrap();

        assert_eq!(bytes1.len(), 32);
        assert_eq!(bytes2.len(), 32);
        assert_ne!(bytes1, bytes2); // Should be different
    }

    #[test]
    fn test_secure_clear() {
        let mut data = vec![1, 2, 3, 4, 5];
        secure_clear(&mut data);
        assert_eq!(data, vec![0, 0, 0, 0, 0]);
    }
}
