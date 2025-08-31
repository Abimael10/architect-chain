// This file implements the transaction system - the core of how value moves in my blockchain
// I'm following Bitcoin's UTXO (Unspent Transaction Output) model for maximum compatibility
// Each transaction consumes previous outputs and creates new ones

use crate::core::{Blockchain, FeeCalculator, FeePriority, INITIAL_BLOCK_REWARD};
use crate::error::{BlockchainError, Result};
use crate::storage::UTXOSet;
use crate::utils::{
    base58_decode, deserialize, ecdsa_p256_sha256_sign_digest, ecdsa_p256_sha256_sign_verify,
    serialize, sha256_digest,
};
use crate::wallet::{hash_pub_key, validate_address, Wallets};
use data_encoding::HEXLOWER;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// I use this constant for the block reward in coinbase transactions
const SUBSIDY: u64 = INITIAL_BLOCK_REWARD;

// This represents a transaction input - it references a previous transaction output
// Think of it as "I want to spend output #2 from transaction ABC123"
#[derive(Debug, Clone, Default, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct TXInput {
    txid: Vec<u8>,      // The ID of the transaction containing the output I want to spend
    vout: usize,        // The index of the output in that transaction
    signature: Vec<u8>, // My digital signature proving I own this output
    pub_key: Vec<u8>,   // My public key (used to verify the signature)
}

impl TXInput {
    // When I create a new transaction input (before signing)
    pub fn new(txid: &[u8], vout: usize) -> TXInput {
        TXInput {
            txid: txid.to_vec(),
            vout,
            signature: vec![], // I'll add the signature later
            pub_key: vec![],   // I'll add the public key later
        }
    }

    // I use these getters to access the input data safely
    pub fn get_txid(&self) -> &[u8] {
        self.txid.as_slice()
    }

    pub fn get_vout(&self) -> usize {
        self.vout
    }

    pub fn get_pub_key(&self) -> &[u8] {
        self.pub_key.as_slice()
    }

    // I use this to check if this input belongs to a specific public key
    #[allow(dead_code)]
    fn uses_key(&self, pub_key_hash: &[u8]) -> bool {
        let locking_hash = hash_pub_key(self.pub_key.as_slice());
        locking_hash.eq(pub_key_hash)
    }
}

// This represents a transaction output - it's like a "check" that can be cashed later
// Think of it as "Pay 100 satoshis to whoever has the private key for address XYZ"
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct TXOutput {
    value: u64,            // How much cryptocurrency this output is worth (in satoshis)
    pub_key_hash: Vec<u8>, // The hash of the public key that can spend this output
}

impl TXOutput {
    pub fn new(value: u64, address: &str) -> Result<TXOutput> {
        if value == 0 {
            return Err(BlockchainError::Transaction(
                "Transaction value must be positive".to_string(),
            ));
        }

        let mut output = TXOutput {
            value,
            pub_key_hash: vec![],
        };
        output.lock(address)?;
        Ok(output)
    }

    pub fn get_value(&self) -> u64 {
        self.value
    }

    pub fn get_pub_key_hash(&self) -> &[u8] {
        self.pub_key_hash.as_slice()
    }

    fn lock(&mut self, address: &str) -> Result<()> {
        if !validate_address(address) {
            return Err(BlockchainError::InvalidAddress(address.to_string()));
        }

        let payload = base58_decode(address)?;
        if payload.len() < crate::wallet::ADDRESS_CHECK_SUM_LEN + 1 {
            return Err(BlockchainError::InvalidAddress(
                "Address too short".to_string(),
            ));
        }

        let pub_key_hash =
            payload[1..payload.len() - crate::wallet::ADDRESS_CHECK_SUM_LEN].to_vec();
        self.pub_key_hash = pub_key_hash;
        Ok(())
    }

    pub fn is_locked_with_key(&self, pub_key_hash: &[u8]) -> bool {
        self.pub_key_hash.eq(pub_key_hash)
    }
}

// This is the main transaction structure - it represents a transfer of value
// A transaction takes some inputs (previous outputs) and creates new outputs
#[derive(Debug, Clone, Default, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct Transaction {
    id: Vec<u8>,         // Unique identifier for this transaction (hash of its contents)
    vin: Vec<TXInput>,   // List of inputs (what I'm spending)
    vout: Vec<TXOutput>, // List of outputs (where the money is going)
    fee: u64,            // Transaction fee in satoshis (paid to miners)
}

impl Transaction {
    // When I create a coinbase transaction (the reward for mining a block)
    pub fn new_coinbase_tx(to: &str) -> Result<Transaction> {
        Self::new_coinbase_tx_with_reward(to, SUBSIDY)
    }

    // When I create a coinbase transaction with a specific reward amount
    pub fn new_coinbase_tx_with_reward(to: &str, reward: u64) -> Result<Transaction> {
        // I create an output that pays the reward to the miner
        let txout = TXOutput::new(reward, to)?;
        // Coinbase transactions have a special input with no previous transaction
        let tx_input = TXInput {
            signature: Uuid::new_v4().as_bytes().to_vec(), // Random data instead of a real signature
            ..Default::default()
        };

        let mut tx = Transaction {
            id: vec![],
            vin: vec![tx_input],
            vout: vec![txout],
            fee: 0, // Coinbase transactions don't pay fees (they create new money)
        };

        // I calculate the transaction ID by hashing its contents
        tx.id = tx.hash();
        Ok(tx)
    }

    /// Create a coinbase transaction with collected fees using the fee calculator
    pub fn new_coinbase_tx_with_collected_fees(
        to: &str,
        collected_fees: u64,
    ) -> Result<Transaction> {
        let total_reward = FeeCalculator::calculate_coinbase_reward(collected_fees);
        Self::new_coinbase_tx_with_reward(to, total_reward)
    }

    pub fn new_utxo_transaction(
        from: &str,
        to: &str,
        amount: u64,
        utxo_set: &UTXOSet,
    ) -> Result<Transaction> {
        // Use normal priority for backward compatibility
        Self::new_utxo_transaction_with_priority(from, to, amount, FeePriority::Normal, utxo_set)
    }

    /// Create a UTXO transaction with a specific priority (new dynamic fee system)
    pub fn new_utxo_transaction_with_priority(
        from: &str,
        to: &str,
        amount: u64,
        priority: FeePriority,
        utxo_set: &UTXOSet,
    ) -> Result<Transaction> {
        // Validate inputs
        if amount == 0 {
            return Err(BlockchainError::Transaction(
                "Amount must be positive".to_string(),
            ));
        }

        if !validate_address(from) {
            return Err(BlockchainError::InvalidAddress(format!(
                "Invalid from address: {from}"
            )));
        }

        if !validate_address(to) {
            return Err(BlockchainError::InvalidAddress(format!(
                "Invalid to address: {to}"
            )));
        }

        let wallets = Wallets::new();
        let wallet = wallets.get_wallet(from).ok_or_else(|| {
            BlockchainError::Wallet(format!("Wallet not found for address: {from}"))
        })?;
        let public_key_hash = hash_pub_key(wallet.get_public_key());

        let (accumulated, valid_outputs) =
            utxo_set.find_spendable_outputs(public_key_hash.as_slice(), amount);

        // Calculate fee using the new fee system
        let estimated_size = FeeCalculator::estimate_transaction_size(valid_outputs.len(), 2); // Estimate 2 outputs (to + change)
        let fee_amount = FeeCalculator::calculate_fee(estimated_size, Some(priority));

        // Check if we have enough funds for amount + fee
        let total_needed = amount + fee_amount;
        if accumulated < total_needed {
            return Err(BlockchainError::InsufficientFunds {
                required: total_needed,
                available: accumulated,
            });
        }

        let mut inputs = vec![];
        for (txid_hex, outs) in valid_outputs {
            let txid = HEXLOWER.decode(txid_hex.as_bytes()).map_err(|e| {
                BlockchainError::Transaction(format!("Invalid transaction ID: {e}"))
            })?;
            for out in outs {
                let input = TXInput {
                    txid: txid.clone(),
                    vout: out,
                    signature: vec![],
                    pub_key: wallet.get_public_key().to_vec(),
                };
                inputs.push(input);
            }
        }

        let mut outputs = vec![TXOutput::new(amount, to)?];

        // Calculate change after deducting amount and fee
        let change = accumulated - amount - fee_amount;
        if change > 0 {
            outputs.push(TXOutput::new(change, from)?); // Change output
        }

        let mut tx = Transaction {
            id: vec![],
            vin: inputs,
            vout: outputs,
            fee: fee_amount,
        };

        tx.id = tx.hash();

        tx.sign(utxo_set.get_blockchain(), wallet.get_pkcs8())?;
        Ok(tx)
    }

    /// Create a UTXO transaction with a specific fee rate (legacy compatibility)
    pub fn new_utxo_transaction_with_fee(
        from: &str,
        to: &str,
        amount: u64,
        fee_rate: u64,
        utxo_set: &UTXOSet,
    ) -> Result<Transaction> {
        // For backward compatibility, validate the fee rate and use legacy calculation
        FeeCalculator::validate_fee_rate(fee_rate)?;

        // Estimate transaction size and calculate legacy fee
        let estimated_size = FeeCalculator::estimate_transaction_size(2, 2); // Rough estimate
        let legacy_fee = FeeCalculator::calculate_legacy_fee(estimated_size, fee_rate)?;

        // Create transaction using the new priority system but with calculated legacy fee
        Self::new_utxo_transaction_with_explicit_fee(from, to, amount, legacy_fee, utxo_set)
    }

    /// Create a UTXO transaction with an explicit fee amount
    pub fn new_utxo_transaction_with_explicit_fee(
        from: &str,
        to: &str,
        amount: u64,
        fee_amount: u64,
        utxo_set: &UTXOSet,
    ) -> Result<Transaction> {
        // Validate inputs
        if amount == 0 {
            return Err(BlockchainError::Transaction(
                "Amount must be positive".to_string(),
            ));
        }

        if !validate_address(from) {
            return Err(BlockchainError::InvalidAddress(format!(
                "Invalid from address: {from}"
            )));
        }

        if !validate_address(to) {
            return Err(BlockchainError::InvalidAddress(format!(
                "Invalid to address: {to}"
            )));
        }

        let wallets = Wallets::new();
        let wallet = wallets.get_wallet(from).ok_or_else(|| {
            BlockchainError::Wallet(format!("Wallet not found for address: {from}"))
        })?;
        let public_key_hash = hash_pub_key(wallet.get_public_key());

        let (accumulated, valid_outputs) =
            utxo_set.find_spendable_outputs(public_key_hash.as_slice(), amount);

        // Check if we have enough funds for amount + fee
        let total_needed = amount + fee_amount;
        if accumulated < total_needed {
            return Err(BlockchainError::InsufficientFunds {
                required: total_needed,
                available: accumulated,
            });
        }

        let mut inputs = vec![];
        for (txid_hex, outs) in valid_outputs {
            let txid = HEXLOWER.decode(txid_hex.as_bytes()).map_err(|e| {
                BlockchainError::Transaction(format!("Invalid transaction ID: {e}"))
            })?;
            for out in outs {
                let input = TXInput {
                    txid: txid.clone(),
                    vout: out,
                    signature: vec![],
                    pub_key: wallet.get_public_key().to_vec(),
                };
                inputs.push(input);
            }
        }

        let mut outputs = vec![TXOutput::new(amount, to)?];

        // Calculate change after deducting amount and fee
        let change = accumulated - amount - fee_amount;
        if change > 0 {
            outputs.push(TXOutput::new(change, from)?); // Change output
        }

        let mut tx = Transaction {
            id: vec![],
            vin: inputs,
            vout: outputs,
            fee: fee_amount,
        };

        tx.id = tx.hash();

        tx.sign(utxo_set.get_blockchain(), wallet.get_pkcs8())?;
        Ok(tx)
    }

    fn trimmed_copy(&self) -> Transaction {
        let mut inputs = vec![];
        let mut outputs = vec![];
        for input in &self.vin {
            let txinput = TXInput::new(input.get_txid(), input.get_vout());
            inputs.push(txinput);
        }
        for output in &self.vout {
            outputs.push(output.clone());
        }
        Transaction {
            id: self.id.clone(),
            vin: inputs,
            vout: outputs,
            fee: self.fee,
        }
    }

    fn sign(&mut self, blockchain: &Blockchain, pkcs8: &[u8]) -> Result<()> {
        let mut tx_copy = self.trimmed_copy();

        for (idx, vin) in self.vin.iter_mut().enumerate() {
            let prev_tx = blockchain.find_transaction(vin.get_txid()).ok_or_else(|| {
                BlockchainError::Transaction("Previous transaction not found".to_string())
            })?;

            if vin.vout >= prev_tx.vout.len() {
                return Err(BlockchainError::Transaction(
                    "Invalid output index".to_string(),
                ));
            }

            tx_copy.vin[idx].signature = vec![];
            tx_copy.vin[idx].pub_key = prev_tx.vout[vin.vout].pub_key_hash.clone();
            tx_copy.id = tx_copy.hash();
            tx_copy.vin[idx].pub_key = vec![];

            let signature = ecdsa_p256_sha256_sign_digest(pkcs8, tx_copy.get_id())?;
            vin.signature = signature;
        }
        Ok(())
    }

    pub fn verify(&self, blockchain: &Blockchain) -> bool {
        if self.is_coinbase() {
            return true;
        }

        let mut tx_copy = self.trimmed_copy();
        for (idx, vin) in self.vin.iter().enumerate() {
            let prev_tx = match blockchain.find_transaction(vin.get_txid()) {
                Some(tx) => tx,
                None => {
                    log::error!("Previous transaction not found during verification");
                    return false;
                }
            };

            if vin.vout >= prev_tx.vout.len() {
                log::error!("Invalid output index during verification");
                return false;
            }

            tx_copy.vin[idx].signature = vec![];
            tx_copy.vin[idx].pub_key = prev_tx.vout[vin.vout].pub_key_hash.clone();
            tx_copy.id = tx_copy.hash();
            tx_copy.vin[idx].pub_key = vec![];

            let verify = ecdsa_p256_sha256_sign_verify(
                vin.pub_key.as_slice(),
                vin.signature.as_slice(),
                tx_copy.get_id(),
            );
            if !verify {
                return false;
            }
        }
        true
    }

    pub fn is_coinbase(&self) -> bool {
        self.vin.len() == 1 && self.vin[0].pub_key.is_empty()
    }

    fn hash(&mut self) -> Vec<u8> {
        let tx_copy = Transaction {
            id: vec![],
            vin: self.vin.clone(),
            vout: self.vout.clone(),
            fee: self.fee,
        };
        // Use proper error handling instead of expect
        match tx_copy.serialize() {
            Ok(serialized) => sha256_digest(&serialized),
            Err(_) => {
                // Fallback hash for serialization errors
                log::error!("Transaction serialization failed during hash calculation");
                sha256_digest(b"transaction_serialization_error")
            }
        }
    }

    pub fn get_id(&self) -> &[u8] {
        self.id.as_slice()
    }

    pub fn get_id_bytes(&self) -> Vec<u8> {
        self.id.clone()
    }

    pub fn get_vin(&self) -> &[TXInput] {
        self.vin.as_slice()
    }

    pub fn get_vout(&self) -> &[TXOutput] {
        self.vout.as_slice()
    }

    pub fn get_fee(&self) -> u64 {
        self.fee
    }

    pub fn set_fee(&mut self, fee: u64) {
        self.fee = fee;
    }

    /// Calculate the fee rate (satoshis per byte) for this transaction
    pub fn calculate_fee_rate(&self) -> Result<u64> {
        let size = self.serialize()?.len();
        crate::core::FeeCalculator::calculate_fee_rate(self.fee, size)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        serialize(self)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Transaction> {
        deserialize(bytes)
    }
}
