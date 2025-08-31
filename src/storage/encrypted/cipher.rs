use crate::error::{BlockchainError, Result};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use serde::{Deserialize, Serialize};
use zeroize::ZeroizeOnDrop;

/// Result of encryption operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionResult {
    /// Encrypted data
    pub ciphertext: Vec<u8>,
    /// Nonce used for encryption
    pub nonce: Vec<u8>,
}

/// Secure key wrapper that automatically zeros memory on drop
#[derive(Clone, ZeroizeOnDrop)]
pub struct SecureKey {
    key: Vec<u8>,
}

impl SecureKey {
    /// Create a new secure key
    pub fn new(key: Vec<u8>) -> Self {
        Self { key }
    }

    /// Get key bytes (use carefully)
    pub fn as_bytes(&self) -> &[u8] {
        &self.key
    }

    /// Get key length
    pub fn len(&self) -> usize {
        self.key.len()
    }

    /// Check if key is empty
    pub fn is_empty(&self) -> bool {
        self.key.is_empty()
    }
}

impl std::fmt::Debug for SecureKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecureKey")
            .field("length", &self.key.len())
            .finish()
    }
}

/// AES-256-GCM cipher implementation for wallet encryption
pub struct Aes256GcmCipher {
    cipher: Aes256Gcm,
}

impl Aes256GcmCipher {
    /// Create a new cipher with the given key
    pub fn new(key: SecureKey) -> Result<Self> {
        if key.len() != 32 {
            return Err(BlockchainError::Encryption(
                "AES-256-GCM requires a 32-byte key".to_string(),
            ));
        }

        let aes_key = Key::<Aes256Gcm>::from_slice(key.as_bytes());
        let cipher = Aes256Gcm::new(aes_key);

        Ok(Self { cipher })
    }

    /// Create cipher from raw key bytes
    pub fn from_key_bytes(key_bytes: &[u8]) -> Result<Self> {
        let key = SecureKey::new(key_bytes.to_vec());
        Self::new(key)
    }

    /// Encrypt data with a random nonce
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptionResult> {
        // Generate a random nonce
        let nonce_bytes = self.generate_nonce()?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the data
        let ciphertext = self.cipher.encrypt(nonce, plaintext).map_err(|e| {
            BlockchainError::Encryption(format!("AES-256-GCM encryption failed: {e}"))
        })?;

        Ok(EncryptionResult {
            ciphertext,
            nonce: nonce_bytes,
        })
    }

    /// Encrypt data with a specific nonce
    pub fn encrypt_with_nonce(&self, plaintext: &[u8], nonce_bytes: &[u8]) -> Result<Vec<u8>> {
        if nonce_bytes.len() != 12 {
            return Err(BlockchainError::Encryption(
                "AES-256-GCM requires a 12-byte nonce".to_string(),
            ));
        }

        let nonce = Nonce::from_slice(nonce_bytes);
        let ciphertext = self.cipher.encrypt(nonce, plaintext).map_err(|e| {
            BlockchainError::Encryption(format!("AES-256-GCM encryption failed: {e}"))
        })?;

        Ok(ciphertext)
    }

    /// Decrypt data with the given nonce
    pub fn decrypt(&self, ciphertext: &[u8], nonce_bytes: &[u8]) -> Result<Vec<u8>> {
        if nonce_bytes.len() != 12 {
            return Err(BlockchainError::Encryption(
                "AES-256-GCM requires a 12-byte nonce".to_string(),
            ));
        }

        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = self.cipher.decrypt(nonce, ciphertext).map_err(|e| {
            BlockchainError::Encryption(format!("AES-256-GCM decryption failed: {e}"))
        })?;

        Ok(plaintext)
    }

    /// Generate a cryptographically secure random nonce
    fn generate_nonce(&self) -> Result<Vec<u8>> {
        use rand::RngCore;
        let mut nonce = vec![0u8; 12]; // AES-GCM uses 96-bit nonces
        rand::thread_rng().fill_bytes(&mut nonce);
        Ok(nonce)
    }

    /// Validate key length
    pub fn validate_key(key: &[u8]) -> Result<()> {
        if key.len() != 32 {
            return Err(BlockchainError::Encryption(
                "AES-256-GCM requires a 32-byte key".to_string(),
            ));
        }
        Ok(())
    }

    /// Get the required key length
    pub const fn key_length() -> usize {
        32
    }

    /// Get the nonce length
    pub const fn nonce_length() -> usize {
        12
    }

    /// Get the authentication tag length
    pub const fn tag_length() -> usize {
        16
    }

    /// Get cipher algorithm name
    pub fn algorithm_name() -> &'static str {
        "AES-256-GCM"
    }
}

/// Generate a secure random key
pub fn generate_key() -> Result<SecureKey> {
    use rand::RngCore;
    let mut key_bytes = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut key_bytes);
    Ok(SecureKey::new(key_bytes))
}

/// Secure memory utilities
pub struct SecureMemory;

impl SecureMemory {
    /// Securely clear sensitive data from memory
    pub fn clear(data: &mut [u8]) {
        use zeroize::Zeroize;
        data.zeroize();
    }

    /// Securely compare two byte arrays in constant time
    pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut result = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }
        result == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_key() {
        let key_bytes = vec![1u8; 32];
        let key = SecureKey::new(key_bytes.clone());

        assert_eq!(key.len(), 32);
        assert!(!key.is_empty());
        assert_eq!(key.as_bytes(), &key_bytes);
    }

    #[test]
    fn test_cipher_creation() {
        let key = SecureKey::new(vec![0u8; 32]);
        let cipher = Aes256GcmCipher::new(key);
        assert!(cipher.is_ok());

        // Test invalid key length
        let invalid_key = SecureKey::new(vec![0u8; 16]);
        let cipher = Aes256GcmCipher::new(invalid_key);
        assert!(cipher.is_err());
    }

    #[test]
    fn test_encryption_decryption() {
        let key = SecureKey::new(vec![1u8; 32]);
        let cipher = Aes256GcmCipher::new(key).unwrap();
        let plaintext = b"Hello, World! This is a test of AES-256-GCM encryption.";

        // Test encryption
        let result = cipher.encrypt(plaintext).unwrap();
        assert!(!result.ciphertext.is_empty());
        assert_eq!(result.nonce.len(), 12);

        // Test decryption
        let decrypted = cipher.decrypt(&result.ciphertext, &result.nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encryption_with_specific_nonce() {
        let key = SecureKey::new(vec![1u8; 32]);
        let cipher = Aes256GcmCipher::new(key).unwrap();
        let plaintext = b"Hello, World!";
        let nonce = vec![2u8; 12];

        // Test encryption with specific nonce
        let ciphertext = cipher.encrypt_with_nonce(plaintext, &nonce).unwrap();
        assert!(!ciphertext.is_empty());

        // Test decryption
        let decrypted = cipher.decrypt(&ciphertext, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_keys_produce_different_results() {
        let key1 = SecureKey::new(vec![1u8; 32]);
        let key2 = SecureKey::new(vec![2u8; 32]);
        let cipher1 = Aes256GcmCipher::new(key1).unwrap();
        let cipher2 = Aes256GcmCipher::new(key2).unwrap();
        let plaintext = b"Hello, World!";
        let nonce = vec![0u8; 12];

        let ciphertext1 = cipher1.encrypt_with_nonce(plaintext, &nonce).unwrap();
        let ciphertext2 = cipher2.encrypt_with_nonce(plaintext, &nonce).unwrap();

        assert_ne!(ciphertext1, ciphertext2);
    }

    #[test]
    fn test_key_validation() {
        assert!(Aes256GcmCipher::validate_key(&[0u8; 32]).is_ok());
        assert!(Aes256GcmCipher::validate_key(&[0u8; 16]).is_err());
        assert!(Aes256GcmCipher::validate_key(&[0u8; 64]).is_err());
    }

    #[test]
    fn test_constants() {
        assert_eq!(Aes256GcmCipher::key_length(), 32);
        assert_eq!(Aes256GcmCipher::nonce_length(), 12);
        assert_eq!(Aes256GcmCipher::tag_length(), 16);
        assert_eq!(Aes256GcmCipher::algorithm_name(), "AES-256-GCM");
    }

    #[test]
    fn test_secure_memory_operations() {
        let mut data = vec![1, 2, 3, 4, 5];
        SecureMemory::clear(&mut data);
        assert_eq!(data, vec![0, 0, 0, 0, 0]);

        // Test constant time comparison
        let a = vec![1, 2, 3];
        let b = vec![1, 2, 3];
        let c = vec![1, 2, 4];

        assert!(SecureMemory::constant_time_eq(&a, &b));
        assert!(!SecureMemory::constant_time_eq(&a, &c));
        assert!(!SecureMemory::constant_time_eq(&a, &[1, 2])); // Different lengths
    }

    #[test]
    fn test_key_generation() {
        let key1 = generate_key().unwrap();
        let key2 = generate_key().unwrap();

        assert_eq!(key1.len(), 32);
        assert_eq!(key2.len(), 32);
        assert_ne!(key1.as_bytes(), key2.as_bytes()); // Should be different
    }
}
