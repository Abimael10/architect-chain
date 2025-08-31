use crate::error::{BlockchainError, Result};
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
use serde::{Deserialize, Serialize};

const VERSION: u8 = 0x00;
pub const ADDRESS_CHECK_SUM_LEN: usize = 4;

#[derive(Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct Wallet {
    pkcs8: Vec<u8>,
    public_key: Vec<u8>,
}

impl Default for Wallet {
    fn default() -> Self {
        Self::new().expect("Failed to create default wallet")
    }
}

impl Wallet {
    pub fn new() -> Result<Wallet> {
        let pkcs8 = crate::utils::new_key_pair()?;
        let rng = SystemRandom::new();
        let key_pair =
            EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng)
                .map_err(|e| {
                    BlockchainError::Crypto(format!("Failed to create key pair from PKCS8: {e}"))
                })?;
        let public_key = key_pair.public_key().as_ref().to_vec();
        Ok(Wallet { pkcs8, public_key })
    }

    pub fn get_address(&self) -> String {
        let pub_key_hash = hash_pub_key(self.public_key.as_slice());
        let mut payload: Vec<u8> = vec![];
        payload.push(VERSION);
        payload.extend(pub_key_hash.as_slice());
        let checksum = checksum(payload.as_slice());
        payload.extend(checksum.as_slice());
        // version + pub_key_hash + checksum
        crate::utils::base58_encode(payload.as_slice())
    }

    pub fn get_public_key(&self) -> &[u8] {
        self.public_key.as_slice()
    }

    pub fn get_pkcs8(&self) -> &[u8] {
        self.pkcs8.as_slice()
    }
}

pub fn hash_pub_key(pub_key: &[u8]) -> Vec<u8> {
    let pub_key_sha256 = crate::utils::sha256_digest(pub_key);
    crate::utils::ripemd160_digest(pub_key_sha256.as_slice())
}

fn checksum(payload: &[u8]) -> Vec<u8> {
    let first_sha = crate::utils::sha256_digest(payload);
    let second_sha = crate::utils::sha256_digest(first_sha.as_slice());
    second_sha[0..ADDRESS_CHECK_SUM_LEN].to_vec()
}

pub fn validate_address(address: &str) -> bool {
    let payload = match crate::utils::base58_decode(address) {
        Ok(payload) => payload,
        Err(_) => return false, // Invalid base58 encoding
    };

    // Check if payload is long enough
    if payload.len() < ADDRESS_CHECK_SUM_LEN + 1 {
        return false;
    }

    let actual_checksum = payload[payload.len() - ADDRESS_CHECK_SUM_LEN..].to_vec();
    let version = payload[0];
    let pub_key_hash = payload[1..payload.len() - ADDRESS_CHECK_SUM_LEN].to_vec();

    let mut target_vec = vec![];
    target_vec.push(version);
    target_vec.extend(pub_key_hash);
    let target_checksum = checksum(target_vec.as_slice());
    actual_checksum.eq(target_checksum.as_slice())
}

pub fn convert_address(pub_hash_key: &[u8]) -> String {
    let mut payload: Vec<u8> = vec![];
    payload.push(VERSION);
    payload.extend(pub_hash_key);
    let checksum = checksum(payload.as_slice());
    payload.extend(checksum.as_slice());
    crate::utils::base58_encode(payload.as_slice())
}
