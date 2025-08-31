use crate::core::{Block, Blockchain, TXOutput};
use crate::error::{BlockchainError, Result};
use crate::utils::{deserialize, serialize};
use data_encoding::HEXLOWER;
use std::collections::HashMap;

const UTXO_TREE: &str = "chainstate";

pub struct UTXOSet {
    blockchain: Blockchain,
}

impl UTXOSet {
    pub fn new(blockchain: Blockchain) -> UTXOSet {
        UTXOSet { blockchain }
    }

    pub fn get_blockchain(&self) -> &Blockchain {
        &self.blockchain
    }

    pub fn find_spendable_outputs(
        &self,
        pub_key_hash: &[u8],
        amount: u64,
    ) -> (u64, HashMap<String, Vec<usize>>) {
        // For backward compatibility, wrap the Result version
        self.find_spendable_outputs_safe(pub_key_hash, amount)
            .unwrap_or_else(|e| {
                log::error!("Error finding spendable outputs: {e}");
                (0, HashMap::new())
            })
    }

    pub fn find_spendable_outputs_safe(
        &self,
        pub_key_hash: &[u8],
        amount: u64,
    ) -> Result<(u64, HashMap<String, Vec<usize>>)> {
        let mut unspent_outputs: HashMap<String, Vec<usize>> = HashMap::new();
        let mut accmulated = 0;
        let db = self.blockchain.get_db();
        let utxo_tree = db
            .open_tree(UTXO_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open UTXO tree: {e}")))?;

        for item in utxo_tree.iter() {
            let (k, v) = item.map_err(|e| {
                BlockchainError::Database(format!("Failed to iterate UTXO tree: {e}"))
            })?;
            let txid_hex = HEXLOWER.encode(k.to_vec().as_slice());
            let outs: Vec<TXOutput> = deserialize(v.to_vec().as_slice()).map_err(|e| {
                BlockchainError::Serialization(format!("Failed to deserialize TXOutput: {e}"))
            })?;

            for (idx, out) in outs.iter().enumerate() {
                if out.is_locked_with_key(pub_key_hash) && accmulated < amount {
                    accmulated += out.get_value();
                    if let Some(output_list) = unspent_outputs.get_mut(txid_hex.as_str()) {
                        output_list.push(idx);
                    } else {
                        unspent_outputs.insert(txid_hex.clone(), vec![idx]);
                    }
                }
            }
        }
        Ok((accmulated, unspent_outputs))
    }

    pub fn find_utxo(&self, pub_key_hash: &[u8]) -> Vec<TXOutput> {
        // For backward compatibility, wrap the Result version
        self.find_utxo_safe(pub_key_hash).unwrap_or_else(|e| {
            log::error!("Error finding UTXOs: {e}");
            vec![]
        })
    }

    pub fn find_utxo_safe(&self, pub_key_hash: &[u8]) -> Result<Vec<TXOutput>> {
        let db = self.blockchain.get_db();
        let utxo_tree = db
            .open_tree(UTXO_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open UTXO tree: {e}")))?;
        let mut utxos = vec![];

        for item in utxo_tree.iter() {
            let (_, v) = item.map_err(|e| {
                BlockchainError::Database(format!("Failed to iterate UTXO tree: {e}"))
            })?;
            let outs: Vec<TXOutput> = deserialize(v.to_vec().as_slice()).map_err(|e| {
                BlockchainError::Serialization(format!("Failed to deserialize TXOutput: {e}"))
            })?;

            for out in outs.iter() {
                if out.is_locked_with_key(pub_key_hash) {
                    utxos.push(out.clone())
                }
            }
        }
        Ok(utxos)
    }

    pub fn count_transactions(&self) -> u64 {
        // For backward compatibility, return 0 on error
        self.count_transactions_safe().unwrap_or_else(|e| {
            log::error!("Error counting transactions: {e}");
            0
        })
    }

    pub fn count_transactions_safe(&self) -> Result<u64> {
        let db = self.blockchain.get_db();
        let utxo_tree = db
            .open_tree(UTXO_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open UTXO tree: {e}")))?;
        let mut counter = 0;

        for item in utxo_tree.iter() {
            item.map_err(|e| {
                BlockchainError::Database(format!("Failed to iterate UTXO tree: {e}"))
            })?;
            counter += 1;
        }
        Ok(counter)
    }

    pub fn reindex(&self) {
        // For backward compatibility, ignore errors but log them
        if let Err(e) = self.reindex_safe() {
            log::error!("Error during UTXO reindex: {e}");
        }
    }

    pub fn reindex_safe(&self) -> Result<()> {
        let db = self.blockchain.get_db();
        let utxo_tree = db
            .open_tree(UTXO_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open UTXO tree: {e}")))?;

        utxo_tree
            .clear()
            .map_err(|e| BlockchainError::Database(format!("Failed to clear UTXO tree: {e}")))?;

        let utxo_map = self.blockchain.find_utxo();
        for (txid_hex, outs) in &utxo_map {
            let txid = HEXLOWER.decode(txid_hex.as_bytes()).map_err(|e| {
                BlockchainError::Serialization(format!("Failed to decode transaction ID: {e}"))
            })?;
            let value = serialize(outs).map_err(|e| {
                BlockchainError::Serialization(format!("Failed to serialize outputs: {e}"))
            })?;
            utxo_tree
                .insert(txid.as_slice(), value)
                .map_err(|e| BlockchainError::Database(format!("Failed to insert UTXO: {e}")))?;
        }
        Ok(())
    }

    pub fn update(&self, block: &Block) {
        // For backward compatibility, ignore errors but log them
        if let Err(e) = self.update_safe(block) {
            log::error!("Error updating UTXO set: {e}");
        }
    }

    pub fn update_safe(&self, block: &Block) -> Result<()> {
        let db = self.blockchain.get_db();
        let utxo_tree = db
            .open_tree(UTXO_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open UTXO tree: {e}")))?;

        for tx in block.get_transactions() {
            if !tx.is_coinbase() {
                for vin in tx.get_vin() {
                    let mut updated_outs = vec![];

                    let outs_bytes = utxo_tree
                        .get(vin.get_txid())
                        .map_err(|e| BlockchainError::Database(format!("Failed to get UTXO: {e}")))?
                        .ok_or_else(|| BlockchainError::Database("UTXO not found".to_string()))?;

                    let outs: Vec<TXOutput> = deserialize(outs_bytes.as_ref()).map_err(|e| {
                        BlockchainError::Serialization(format!(
                            "Failed to deserialize TXOutput: {e}"
                        ))
                    })?;

                    for (idx, out) in outs.iter().enumerate() {
                        if idx != vin.get_vout() {
                            updated_outs.push(out.clone())
                        }
                    }

                    if updated_outs.is_empty() {
                        utxo_tree.remove(vin.get_txid()).map_err(|e| {
                            BlockchainError::Database(format!("Failed to remove UTXO: {e}"))
                        })?;
                    } else {
                        let outs_bytes = serialize(&updated_outs).map_err(|e| {
                            BlockchainError::Serialization(format!(
                                "Failed to serialize TXOutput: {e}"
                            ))
                        })?;
                        utxo_tree.insert(vin.get_txid(), outs_bytes).map_err(|e| {
                            BlockchainError::Database(format!("Failed to update UTXO: {e}"))
                        })?;
                    }
                }
            }

            let mut new_outputs = vec![];
            for out in tx.get_vout() {
                new_outputs.push(out.clone())
            }

            let outs_bytes = serialize(&new_outputs).map_err(|e| {
                BlockchainError::Serialization(format!("Failed to serialize TXOutput: {e}"))
            })?;
            utxo_tree.insert(tx.get_id(), outs_bytes).map_err(|e| {
                BlockchainError::Database(format!("Failed to insert new UTXO: {e}"))
            })?;
        }
        Ok(())
    }
}
