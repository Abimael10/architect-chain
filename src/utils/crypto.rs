use ring::digest::{Context, SHA256};
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_FIXED, ECDSA_P256_SHA256_FIXED_SIGNING};
use ripemd::{Digest as RipemdDigest, Ripemd160};

use crate::error::{BlockchainError, Result};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_timestamp() -> Result<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| BlockchainError::Crypto(format!("System time error: {e}")))?
        .as_millis();

    // Ensure the timestamp fits in i64
    if duration > i64::MAX as u128 {
        return Err(BlockchainError::Crypto("Timestamp overflow".to_string()));
    }

    Ok(duration as i64)
}

pub fn sha256_digest(data: &[u8]) -> Vec<u8> {
    let mut context = Context::new(&SHA256);
    context.update(data);
    let digest = context.finish();
    digest.as_ref().to_vec()
}

pub fn ripemd160_digest(data: &[u8]) -> Vec<u8> {
    let mut hasher = Ripemd160::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

pub fn base58_encode(data: &[u8]) -> String {
    bs58::encode(data).into_string()
}

pub fn base58_decode(data: &str) -> Result<Vec<u8>> {
    bs58::decode(data)
        .into_vec()
        .map_err(|e| BlockchainError::InvalidAddress(format!("Invalid base58 encoding: {e}")))
}

pub fn new_key_pair() -> Result<Vec<u8>> {
    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
        .map_err(|e| BlockchainError::Crypto(format!("Failed to generate ECDSA key pair: {e}")))?
        .as_ref()
        .to_vec();
    Ok(pkcs8)
}

pub fn ecdsa_p256_sha256_sign_digest(pkcs8: &[u8], message: &[u8]) -> Result<Vec<u8>> {
    let rng = ring::rand::SystemRandom::new();
    let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8, &rng)
        .map_err(|e| {
            BlockchainError::Crypto(format!("Failed to create key pair from PKCS8: {e}"))
        })?;
    let signature = key_pair
        .sign(&rng, message)
        .map_err(|e| BlockchainError::Crypto(format!("Failed to sign message: {e}")))?
        .as_ref()
        .to_vec();
    Ok(signature)
}

pub fn ecdsa_p256_sha256_sign_verify(public_key: &[u8], signature: &[u8], message: &[u8]) -> bool {
    let peer_public_key =
        ring::signature::UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, public_key);
    let result = peer_public_key.verify(message, signature.as_ref());
    result.is_ok()
}
