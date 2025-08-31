use crate::error::{BlockchainError, Result};
use crate::storage::encrypted::cipher::{Aes256GcmCipher, SecureKey};
use crate::utils::{deserialize, serialize};
use crate::wallet::{Wallet, WALLET_FILE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env::current_dir;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};
// Path import removed as not needed

/// Simple configuration for wallet encryption
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct WalletEncryptionConfig {
    /// Whether encryption is enabled
    pub enabled: bool,
    /// Wallet file path
    pub wallet_file: String,
    /// Whether to create encrypted backups
    pub backup_enabled: bool,
    /// Backup directory
    pub backup_dir: String,
    /// Minimum password length
    pub min_password_length: usize,
}

impl Default for WalletEncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            wallet_file: WALLET_FILE.to_string(),
            backup_enabled: true,
            backup_dir: "wallet_backups".to_string(),
            min_password_length: 8,
        }
    }
}

/// Simple encrypted wallet container
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct EncryptedWalletData {
    /// Encrypted wallet data
    pub ciphertext: Vec<u8>,
    /// Nonce used for encryption
    pub nonce: Vec<u8>,
    /// Salt used for key derivation
    pub salt: Vec<u8>,
    /// Number of wallets
    pub wallet_count: usize,
    /// Wallet addresses (for quick lookup)
    pub addresses: Vec<String>,
    /// Creation timestamp
    pub created_at: u64,
    /// Last modified timestamp
    pub modified_at: u64,
}

/// Simplified encrypted wallets manager
pub struct EncryptedWallets {
    wallets: HashMap<String, Wallet>,
    config: WalletEncryptionConfig,
    master_key: Option<SecureKey>,
    current_salt: Option<Vec<u8>>,
    is_encrypted: bool,
    is_loaded: bool,
}

impl EncryptedWallets {
    /// Create a new encrypted wallets manager
    pub fn new(config: WalletEncryptionConfig) -> Self {
        Self {
            wallets: HashMap::new(),
            config,
            master_key: None,
            current_salt: None,
            is_encrypted: false,
            is_loaded: false,
        }
    }

    /// Initialize encryption with a password
    pub fn initialize_encryption(&mut self, password: &str) -> Result<()> {
        if !self.config.enabled {
            self.load_unencrypted()?;
            return Ok(());
        }

        // Validate password
        self.validate_password(password)?;

        let wallet_path = current_dir()?.join(&self.config.wallet_file);

        if wallet_path.exists() {
            // Load existing encrypted wallet
            self.load_encrypted(password)?;
        } else {
            // Create new encrypted wallet
            self.create_encrypted(password)?;
        }

        Ok(())
    }

    /// Simple password validation
    fn validate_password(&self, password: &str) -> Result<()> {
        if password.len() < self.config.min_password_length {
            return Err(BlockchainError::Encryption(format!(
                "Password must be at least {} characters long",
                self.config.min_password_length
            )));
        }
        Ok(())
    }

    /// Create new encrypted wallet file
    fn create_encrypted(&mut self, password: &str) -> Result<()> {
        // Generate master key using simple key derivation
        let salt = crate::storage::encrypted::generate_random_bytes(32)?;
        let key = self.derive_key_from_password(password, &salt)?;

        self.master_key = Some(key);
        self.current_salt = Some(salt);
        self.is_encrypted = true;
        self.is_loaded = true;

        log::info!("Created new encrypted wallet file");
        Ok(())
    }

    /// Simple key derivation from password and salt
    fn derive_key_from_password(&self, password: &str, salt: &[u8]) -> Result<SecureKey> {
        use argon2::{Algorithm, Argon2, Params, Version};

        // Simple Argon2 parameters
        let params = Params::new(65536, 3, 1, Some(32))
            .map_err(|e| BlockchainError::Encryption(format!("Invalid Argon2 parameters: {e}")))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        let mut key = vec![0u8; 32];
        argon2
            .hash_password_into(password.as_bytes(), salt, &mut key)
            .map_err(|e| BlockchainError::Encryption(format!("Key derivation failed: {e}")))?;

        Ok(SecureKey::new(key))
    }

    /// Load existing encrypted wallet file
    fn load_encrypted(&mut self, password: &str) -> Result<()> {
        let wallet_path = current_dir()?.join(&self.config.wallet_file);

        if !wallet_path.exists() {
            return Err(BlockchainError::Wallet(
                "Encrypted wallet file does not exist".to_string(),
            ));
        }

        // Read encrypted wallet file
        let mut file = File::open(&wallet_path)
            .map_err(|e| BlockchainError::Wallet(format!("Failed to open wallet file: {e}")))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| BlockchainError::Wallet(format!("Failed to read wallet file: {e}")))?;

        // Deserialize encrypted wallet data
        let encrypted_wallet: EncryptedWalletData = deserialize(&contents).map_err(|e| {
            BlockchainError::Wallet(format!("Failed to deserialize wallet data: {e}"))
        })?;

        // Derive key from password
        let master_key = self.derive_key_from_password(password, &encrypted_wallet.salt)?;
        let cipher = Aes256GcmCipher::new(master_key.clone())?;

        // Decrypt wallet data
        let decrypted_data =
            cipher.decrypt(&encrypted_wallet.ciphertext, &encrypted_wallet.nonce)?;

        // Deserialize wallets
        self.wallets = deserialize(&decrypted_data)
            .map_err(|e| BlockchainError::Wallet(format!("Failed to deserialize wallets: {e}")))?;

        self.master_key = Some(master_key);
        self.current_salt = Some(encrypted_wallet.salt);
        self.is_encrypted = true;
        self.is_loaded = true;

        log::info!(
            "Loaded encrypted wallet file with {} wallets",
            self.wallets.len()
        );
        Ok(())
    }

    /// Load unencrypted wallet file (legacy support)
    fn load_unencrypted(&mut self) -> Result<()> {
        let wallet_path = current_dir()?.join(&self.config.wallet_file);

        if !wallet_path.exists() {
            self.is_loaded = true;
            return Ok(());
        }

        let mut file = File::open(&wallet_path)
            .map_err(|e| BlockchainError::Wallet(format!("Failed to open wallet file: {e}")))?;

        let metadata = file
            .metadata()
            .map_err(|e| BlockchainError::Wallet(format!("Failed to read file metadata: {e}")))?;

        let mut buf = vec![0; metadata.len() as usize];
        file.read_exact(&mut buf)
            .map_err(|e| BlockchainError::Wallet(format!("Failed to read wallet file: {e}")))?;

        self.wallets = deserialize(&buf)
            .map_err(|e| BlockchainError::Wallet(format!("Failed to deserialize wallets: {e}")))?;

        self.is_loaded = true;
        log::info!(
            "Loaded unencrypted wallet file with {} wallets",
            self.wallets.len()
        );
        Ok(())
    }

    /// Save encrypted wallet file
    fn save_encrypted(&self) -> Result<()> {
        if !self.is_encrypted {
            return self.save_unencrypted();
        }

        let master_key = self
            .master_key
            .as_ref()
            .ok_or_else(|| BlockchainError::Wallet("No master key available".to_string()))?;

        // Serialize wallets
        let wallet_data = serialize(&self.wallets)
            .map_err(|e| BlockchainError::Wallet(format!("Failed to serialize wallets: {e}")))?;

        // Encrypt wallet data
        let cipher = Aes256GcmCipher::new(master_key.clone())?;
        let encryption_result = cipher.encrypt(&wallet_data)?;

        // Get the salt used for key derivation
        let salt = if let Some(existing_salt) = self.get_current_salt() {
            existing_salt
        } else {
            crate::storage::encrypted::generate_random_bytes(32)?
        };

        // Create encrypted wallet data
        let encrypted_wallet = EncryptedWalletData {
            ciphertext: encryption_result.ciphertext,
            nonce: encryption_result.nonce,
            salt,
            wallet_count: self.wallets.len(),
            addresses: self.wallets.keys().cloned().collect(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            modified_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Write to file
        let wallet_path = current_dir()?.join(&self.config.wallet_file);
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&wallet_path)
            .map_err(|e| BlockchainError::Wallet(format!("Failed to create wallet file: {e}")))?;

        let mut writer = BufWriter::new(file);
        let encrypted_bytes = serialize(&encrypted_wallet).map_err(|e| {
            BlockchainError::Wallet(format!("Failed to serialize encrypted wallet: {e}"))
        })?;

        writer
            .write_all(&encrypted_bytes)
            .map_err(|e| BlockchainError::Wallet(format!("Failed to write wallet file: {e}")))?;

        writer
            .flush()
            .map_err(|e| BlockchainError::Wallet(format!("Failed to flush wallet file: {e}")))?;

        // Create backup if enabled
        if self.config.backup_enabled {
            self.create_backup()?;
        }

        log::info!(
            "Saved encrypted wallet file with {} wallets",
            self.wallets.len()
        );
        Ok(())
    }

    /// Save unencrypted wallet file (legacy support)
    fn save_unencrypted(&self) -> Result<()> {
        let wallet_path = current_dir()?.join(&self.config.wallet_file);
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&wallet_path)
            .map_err(|e| BlockchainError::Wallet(format!("Failed to create wallet file: {e}")))?;

        let mut writer = BufWriter::new(file);
        let wallet_bytes = serialize(&self.wallets)
            .map_err(|e| BlockchainError::Wallet(format!("Failed to serialize wallets: {e}")))?;

        writer
            .write_all(&wallet_bytes)
            .map_err(|e| BlockchainError::Wallet(format!("Failed to write wallet file: {e}")))?;

        writer
            .flush()
            .map_err(|e| BlockchainError::Wallet(format!("Failed to flush wallet file: {e}")))?;

        log::info!(
            "Saved unencrypted wallet file with {} wallets",
            self.wallets.len()
        );
        Ok(())
    }

    /// Create a backup of the wallet file
    fn create_backup(&self) -> Result<()> {
        let backup_dir = current_dir()?.join(&self.config.backup_dir);
        std::fs::create_dir_all(&backup_dir).map_err(|e| {
            BlockchainError::Wallet(format!("Failed to create backup directory: {e}"))
        })?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let backup_file = backup_dir.join(format!("wallet_backup_{timestamp}.dat"));
        let source_file = current_dir()?.join(&self.config.wallet_file);

        std::fs::copy(&source_file, &backup_file)
            .map_err(|e| BlockchainError::Wallet(format!("Failed to create backup: {e}")))?;

        log::info!("Created wallet backup: {backup_file:?}");
        Ok(())
    }

    /// Create a new wallet
    pub fn create_wallet(&mut self) -> Result<String> {
        if !self.is_loaded {
            return Err(BlockchainError::Wallet(
                "Wallets not loaded. Call initialize_encryption first.".to_string(),
            ));
        }

        let wallet = Wallet::new()?;
        let address = wallet.get_address();
        self.wallets.insert(address.clone(), wallet);

        // Save immediately
        if self.is_encrypted {
            self.save_encrypted()?;
        } else {
            self.save_unencrypted()?;
        }

        log::info!("Created new wallet with address: {address}");
        Ok(address)
    }

    /// Get wallet by address
    pub fn get_wallet(&self, address: &str) -> Option<&Wallet> {
        self.wallets.get(address)
    }

    /// Get all wallet addresses
    pub fn get_addresses(&self) -> Vec<String> {
        self.wallets.keys().cloned().collect()
    }

    /// Get number of wallets
    pub fn wallet_count(&self) -> usize {
        self.wallets.len()
    }

    /// Check if encryption is enabled
    pub fn is_encryption_enabled(&self) -> bool {
        self.is_encrypted
    }

    /// Get current salt for key derivation
    fn get_current_salt(&self) -> Option<Vec<u8>> {
        self.current_salt.clone()
    }
}

impl Drop for EncryptedWallets {
    fn drop(&mut self) {
        // Clear sensitive data
        self.wallets.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_encrypted_wallets_creation() {
        let temp_dir = tempdir().unwrap();
        let config = WalletEncryptionConfig {
            wallet_file: temp_dir
                .path()
                .join("test_wallet.dat")
                .to_str()
                .unwrap()
                .to_string(),
            ..Default::default()
        };

        let wallets = EncryptedWallets::new(config);
        assert!(!wallets.is_encryption_enabled());
        assert_eq!(wallets.wallet_count(), 0);
    }

    #[test]
    fn test_password_validation() {
        let config = WalletEncryptionConfig::default();
        let wallets = EncryptedWallets::new(config);

        // Short password should fail
        assert!(wallets.validate_password("short").is_err());

        // Long enough password should pass
        assert!(wallets.validate_password("long_enough_password").is_ok());
    }

    #[test]
    fn test_encrypted_wallet_operations() {
        let temp_dir = tempdir().unwrap();
        let mut config = WalletEncryptionConfig {
            wallet_file: temp_dir
                .path()
                .join("test_wallet.dat")
                .to_str()
                .unwrap()
                .to_string(),
            ..Default::default()
        };
        config.enabled = true;

        let mut wallets = EncryptedWallets::new(config);

        // Initialize encryption
        assert!(wallets.initialize_encryption("TestPassword123").is_ok());
        assert!(wallets.is_encryption_enabled());

        // Create wallet
        let address = wallets.create_wallet().unwrap();
        assert_eq!(wallets.wallet_count(), 1);
        assert!(wallets.get_wallet(&address).is_some());

        // Test addresses
        let addresses = wallets.get_addresses();
        assert_eq!(addresses.len(), 1);
        assert_eq!(addresses[0], address);
    }

    #[test]
    fn test_wallet_persistence() {
        let temp_dir = tempdir().unwrap();
        let wallet_file = temp_dir.path().join("test_wallet.dat");

        let address = {
            let mut config = WalletEncryptionConfig {
                wallet_file: wallet_file.to_str().unwrap().to_string(),
                ..Default::default()
            };
            config.enabled = true;

            let mut wallets = EncryptedWallets::new(config);
            wallets.initialize_encryption("TestPassword123").unwrap();
            wallets.create_wallet().unwrap()
        };

        // Load wallets again
        let config = WalletEncryptionConfig {
            wallet_file: wallet_file.to_str().unwrap().to_string(),
            enabled: true,
            ..Default::default()
        };

        let mut wallets2 = EncryptedWallets::new(config);
        wallets2.initialize_encryption("TestPassword123").unwrap();

        assert_eq!(wallets2.wallet_count(), 1);
        assert!(wallets2.get_wallet(&address).is_some());
    }
}
