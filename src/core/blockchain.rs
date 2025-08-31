// This is the core blockchain implementation - the heart of my cryptocurrency
// I'm using Sled as an embedded database to store blocks and maintain the chain
// The blockchain follows Bitcoin's design with UTXO model and proof-of-work consensus

use crate::core::{Block, DifficultyAdjustment, FeeCalculator, TXOutput, Transaction};
use crate::error::{BlockchainError, Result};
use data_encoding::HEXLOWER;
use log::info;
use sled::{Db, Tree};
use std::collections::HashMap;
use std::env::current_dir;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

// I use these constants to organize my database storage
const TIP_BLOCK_HASH_KEY: &str = "tip_block_hash"; // Key to store the hash of the latest block
const BLOCKS_TREE: &str = "blocks"; // Tree name for storing all blocks

// This is my main blockchain structure that holds the entire chain state
#[derive(Clone)]
pub struct Blockchain {
    // I use Arc<RwLock<String>> so multiple threads can safely read/write the tip hash
    tip_hash: Arc<RwLock<String>>, // Hash of the most recent block in the chain
    db: Db,                        // The Sled database instance that stores all my blocks
    db_path: PathBuf,              // Path to the database file on disk
}

impl Blockchain {
    // When I want to create a brand new blockchain with a genesis block
    pub fn create_blockchain(genesis_address: &str) -> Result<Blockchain> {
        Self::create_blockchain_with_path(genesis_address, &Self::default_db_path()?)
    }

    // When I want to open an existing blockchain from the default location
    pub fn new_blockchain() -> Result<Blockchain> {
        Self::new_blockchain_with_path(&Self::default_db_path()?)
    }

    // When I want to create a blockchain for a specific node (multi-node setup)
    pub fn create_blockchain_with_node_id(
        genesis_address: &str,
        node_id: &str,
    ) -> Result<Blockchain> {
        let db_path = Self::node_db_path(node_id)?;
        Self::create_blockchain_with_path(genesis_address, &db_path)
    }

    // When I want to open an existing blockchain for a specific node
    pub fn new_blockchain_with_node_id(node_id: &str) -> Result<Blockchain> {
        let db_path = Self::node_db_path(node_id)?;
        Self::new_blockchain_with_path(&db_path)
    }

    // I use this to get the default database path (./data/)
    fn default_db_path() -> Result<String> {
        Ok(current_dir()?.join("data").to_string_lossy().to_string())
    }

    // I use this to get a node-specific database path (./data/node_2001/)
    // This allows multiple nodes to run on the same machine with isolated databases
    fn node_db_path(node_id: &str) -> Result<String> {
        Ok(current_dir()?
            .join("data")
            .join(format!("node_{node_id}"))
            .to_string_lossy()
            .to_string())
    }

    // This is where I actually create a new blockchain with a genesis block
    pub fn create_blockchain_with_path(genesis_address: &str, db_path: &str) -> Result<Blockchain> {
        let path = PathBuf::from(db_path);
        // I open the Sled database at the specified path
        let db = sled::open(&path)
            .map_err(|e| BlockchainError::Database(format!("Failed to open database: {e}")))?;
        // I create a tree specifically for storing blocks
        let blocks_tree = db
            .open_tree(BLOCKS_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open blocks tree: {e}")))?;

        // I check if there's already a blockchain in this database
        let data = blocks_tree
            .get(TIP_BLOCK_HASH_KEY)
            .map_err(|e| BlockchainError::Database(format!("Failed to get tip hash: {e}")))?;

        let tip_hash = if let Some(data) = data {
            // If there's already a blockchain, I use the existing tip hash
            String::from_utf8(data.to_vec())
                .map_err(|e| BlockchainError::Database(format!("Invalid tip hash format: {e}")))?
        } else {
            // If no blockchain exists, I create the genesis block
            info!("Creating genesis block for address: {genesis_address}");
            let coinbase_tx = Transaction::new_coinbase_tx(genesis_address)?;
            let block = Block::generate_genesis_block(&coinbase_tx)?;
            Self::update_blocks_tree(&blocks_tree, &block)?;
            String::from(block.get_hash())
        };

        // I return the new blockchain instance
        Ok(Blockchain {
            tip_hash: Arc::new(RwLock::new(tip_hash)),
            db,
            db_path: path,
        })
    }

    pub fn new_blockchain_with_path(db_path: &str) -> Result<Blockchain> {
        let path = PathBuf::from(db_path);
        let db = sled::open(&path)
            .map_err(|e| BlockchainError::Database(format!("Failed to open database: {e}")))?;
        let blocks_tree = db
            .open_tree(BLOCKS_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open blocks tree: {e}")))?;

        let tip_bytes = blocks_tree
            .get(TIP_BLOCK_HASH_KEY)
            .map_err(|e| BlockchainError::Database(format!("Failed to get tip hash: {e}")))?
            .ok_or_else(|| {
                BlockchainError::Database(
                    "No existing blockchain found. Create one first.".to_string(),
                )
            })?;

        let tip_hash = String::from_utf8(tip_bytes.to_vec())
            .map_err(|e| BlockchainError::Database(format!("Invalid tip hash format: {e}")))?;

        Ok(Blockchain {
            tip_hash: Arc::new(RwLock::new(tip_hash)),
            db,
            db_path: path,
        })
    }

    fn update_blocks_tree(blocks_tree: &Tree, block: &Block) -> Result<()> {
        let block_hash = block.get_hash();
        let block_data = block.serialize()?;

        blocks_tree
            .transaction(|tx_db| {
                tx_db.insert(block_hash, block_data.as_slice())?;
                tx_db.insert(TIP_BLOCK_HASH_KEY, block_hash)?;
                Ok(())
            })
            .map_err(|e: sled::transaction::TransactionError| {
                BlockchainError::Database(format!("Failed to update blocks tree: {e}"))
            })?;

        Ok(())
    }

    pub fn get_db(&self) -> &Db {
        &self.db
    }

    pub fn get_db_path(&self) -> &PathBuf {
        &self.db_path
    }

    pub fn get_tip_hash(&self) -> String {
        self.tip_hash
            .read()
            .expect("Failed to acquire read lock on tip_hash - this should never happen")
            .clone()
    }

    pub fn set_tip_hash(&self, new_tip_hash: &str) {
        let mut tip_hash = self
            .tip_hash
            .write()
            .expect("Failed to acquire write lock on tip_hash - this should never happen");
        *tip_hash = String::from(new_tip_hash)
    }

    // When I want to mine a block without collecting fees (backward compatibility)
    pub fn mine_block(&self, transactions: &[Transaction]) -> Result<Block> {
        // This method is kept for backward compatibility
        // For fee-enabled mining, I use mine_block_with_fees instead
        self.mine_block_internal(transactions, None)
    }

    // When I want to mine a block and collect transaction fees for a miner
    pub fn mine_block_with_fees(
        &self,
        transactions: &[Transaction],
        miner_address: &str,
    ) -> Result<Block> {
        self.mine_block_internal(transactions, Some(miner_address))
    }

    // This is the core mining logic that does the actual work
    fn mine_block_internal(
        &self,
        transactions: &[Transaction],
        miner_address: Option<&str>,
    ) -> Result<Block> {
        // First, I validate all transactions to make sure they're legitimate
        for (i, transaction) in transactions.iter().enumerate() {
            if !transaction.verify(self) {
                return Err(BlockchainError::Transaction(format!(
                    "Invalid transaction at index {i}"
                )));
            }
        }

        // Critical: I need to check for double-spending within this block
        // This prevents the same UTXO from being spent multiple times in one block
        if let Err(e) = self.check_for_double_spending(transactions) {
            return Err(e);
        }

        // I get the current blockchain height to determine the next block's height
        let best_height = self.get_best_height()?;
        let next_height = best_height + 1;

        // I calculate the appropriate difficulty for this block based on recent mining times
        let difficulty = self.calculate_next_difficulty(next_height)?;

        // I prepare the list of transactions that will go into this block
        let mut block_transactions = Vec::new();

        // If a miner address is provided, I create a coinbase transaction with fees
        if let Some(miner_addr) = miner_address {
            // I calculate the total fees from all transactions in this block
            let total_fees = FeeCalculator::calculate_total_fees(transactions.iter());
            // I calculate the total reward (base reward + fees) for the miner
            let coinbase_reward = FeeCalculator::calculate_coinbase_reward(total_fees);

            info!(
                "Mining block with {} total fees collected ({})",
                total_fees,
                if total_fees > 0 {
                    format!("{:.8} coins", FeeCalculator::satoshis_to_coins(total_fees))
                } else {
                    "no fees".to_string()
                }
            );

            // I create the coinbase transaction that pays the miner
            let coinbase_tx =
                Transaction::new_coinbase_tx_with_reward(miner_addr, coinbase_reward)?;
            block_transactions.push(coinbase_tx);
        }

        // I add all the user transactions to the block
        block_transactions.extend_from_slice(transactions);

        info!(
            "Mining block at height {} with {} transactions (difficulty: {})",
            next_height,
            block_transactions.len(),
            difficulty
        );

        let block = Block::new_block(
            self.get_tip_hash(),
            &block_transactions,
            next_height,
            difficulty,
        )?;
        let block_hash = block.get_hash();

        let blocks_tree = self
            .db
            .open_tree(BLOCKS_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open blocks tree: {e}")))?;
        Self::update_blocks_tree(&blocks_tree, &block)?;
        self.set_tip_hash(block_hash);

        if miner_address.is_some() {
            let total_fees = FeeCalculator::calculate_total_fees(transactions.iter());
            info!("Successfully mined block: {block_hash} (difficulty: {difficulty}, fees: {total_fees} satoshis)");
        } else {
            info!("Successfully mined block: {block_hash} (difficulty: {difficulty})");
        }

        Ok(block)
    }

    pub fn iterator(&self) -> BlockchainIterator {
        BlockchainIterator::new(self.get_tip_hash(), self.db.clone())
    }

    /// Calculate the next difficulty based on recent block times
    pub fn calculate_next_difficulty(&self, height: usize) -> Result<u32> {
        // For early blocks, use initial difficulty
        if height < DifficultyAdjustment::get_adjustment_period() {
            return Ok(DifficultyAdjustment::get_initial_difficulty());
        }

        // Get recent blocks for difficulty calculation
        let recent_blocks = self.get_recent_blocks(DifficultyAdjustment::get_adjustment_period())?;

        // Use the difficulty adjustment algorithm
        DifficultyAdjustment::calculate_next_difficulty(&recent_blocks, height)
    }

    /// Get the most recent N blocks from the blockchain
    fn get_recent_blocks(&self, count: usize) -> Result<Vec<Block>> {
        let mut blocks = Vec::new();
        let mut iterator = self.iterator();

        // Collect the requested number of blocks
        for _ in 0..count {
            if let Some(block) = iterator.next() {
                blocks.push(block);
            } else {
                break;
            }
        }

        // Reverse to get chronological order (oldest first)
        blocks.reverse();
        Ok(blocks)
    }

    // ( K -> txid_hex, V -> Vec<TXOutput )
    pub fn find_utxo(&self) -> HashMap<String, Vec<TXOutput>> {
        let mut utxo: HashMap<String, Vec<TXOutput>> = HashMap::new();
        let mut spent_txos: HashMap<String, Vec<usize>> = HashMap::new();

        let mut iterator = self.iterator();
        while let Some(block) = iterator.next() {
            'outer: for tx in block.get_transactions() {
                let txid_hex = HEXLOWER.encode(tx.get_id());
                for (idx, out) in tx.get_vout().iter().enumerate() {
                    if let Some(outs) = spent_txos.get(txid_hex.as_str()) {
                        for spend_out_idx in outs {
                            if idx.eq(spend_out_idx) {
                                continue 'outer;
                            }
                        }
                    }
                    if let Some(utxo_list) = utxo.get_mut(txid_hex.as_str()) {
                        utxo_list.push(out.clone());
                    } else {
                        utxo.insert(txid_hex.clone(), vec![out.clone()]);
                    }
                }
                if tx.is_coinbase() {
                    continue;
                }

                for txin in tx.get_vin() {
                    let txid_hex = HEXLOWER.encode(txin.get_txid());
                    if let Some(spent_list) = spent_txos.get_mut(txid_hex.as_str()) {
                        spent_list.push(txin.get_vout());
                    } else {
                        spent_txos.insert(txid_hex, vec![txin.get_vout()]);
                    }
                }
            }
        }
        utxo
    }

    pub fn find_transaction(&self, txid: &[u8]) -> Option<Transaction> {
        let mut iterator = self.iterator();
        while let Some(block) = iterator.next() {
            for transaction in block.get_transactions() {
                if txid.eq(transaction.get_id()) {
                    return Some(transaction.clone());
                }
            }
        }
        None
    }

    pub fn add_block(&self, block: &Block) -> Result<()> {
        let block_tree = self
            .db
            .open_tree(BLOCKS_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open blocks tree: {e}")))?;

        if block_tree
            .get(block.get_hash())
            .map_err(|e| {
                BlockchainError::Database(format!("Failed to check block existence: {e}"))
            })?
            .is_some()
        {
            return Ok(()); // Block already exists
        }

        let block_data = block.serialize()?;

        block_tree
            .transaction(|tx_db| {
                tx_db.insert(block.get_hash(), block_data.as_slice())?;

                let tip_block_bytes = tx_db.get(self.get_tip_hash())?.ok_or_else(|| {
                    sled::Error::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Tip hash not found",
                    ))
                })?;
                let tip_block = Block::deserialize(tip_block_bytes.as_ref()).map_err(|_| {
                    sled::Error::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Failed to deserialize tip block",
                    ))
                })?;

                if block.get_height() > tip_block.get_height() {
                    tx_db.insert(TIP_BLOCK_HASH_KEY, block.get_hash())?;
                    self.set_tip_hash(block.get_hash());
                }
                Ok(())
            })
            .map_err(|e: sled::transaction::TransactionError| {
                BlockchainError::Database(format!("Failed to add block: {e}"))
            })?;

        Ok(())
    }

    pub fn get_best_height(&self) -> Result<usize> {
        let block_tree = self
            .db
            .open_tree(BLOCKS_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open blocks tree: {e}")))?;
        let tip_block_bytes = block_tree
            .get(self.get_tip_hash())
            .map_err(|e| BlockchainError::Database(format!("Failed to get tip block: {e}")))?
            .ok_or_else(|| BlockchainError::Database("Tip hash not found".to_string()))?;
        let tip_block = Block::deserialize(tip_block_bytes.as_ref())?;
        Ok(tip_block.get_height())
    }

    pub fn get_block_by_bytes(&self, block_hash: &[u8]) -> Result<Option<Block>> {
        let block_tree = self
            .db
            .open_tree(BLOCKS_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open blocks tree: {e}")))?;

        if let Some(block_bytes) = block_tree
            .get(block_hash)
            .map_err(|e| BlockchainError::Database(format!("Failed to get block: {e}")))?
        {
            let block = Block::deserialize(block_bytes.as_ref())?;
            return Ok(Some(block));
        }
        Ok(None)
    }

    pub fn get_block_hashes(&self) -> Vec<Vec<u8>> {
        let mut iterator = self.iterator();
        let mut blocks = vec![];
        while let Some(block) = iterator.next() {
            blocks.push(block.get_hash_bytes());
        }
        blocks
    }

    /// Check if a block exists in the blockchain
    pub fn block_exists(&self, block_hash: &str) -> Result<bool> {
        let block_tree = self
            .db
            .open_tree(BLOCKS_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open blocks tree: {e}")))?;

        let exists = block_tree
            .get(block_hash)
            .map_err(|e| {
                BlockchainError::Database(format!("Failed to check block existence: {e}"))
            })?
            .is_some();

        Ok(exists)
    }

    /// Get a block by hash (string version)
    pub fn get_block(&self, block_hash: &str) -> Result<Option<Block>> {
        let block_tree = self
            .db
            .open_tree(BLOCKS_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open blocks tree: {e}")))?;

        if let Some(block_bytes) = block_tree
            .get(block_hash)
            .map_err(|e| BlockchainError::Database(format!("Failed to get block: {e}")))?
        {
            let block = Block::deserialize(block_bytes.as_ref())?;
            return Ok(Some(block));
        }
        Ok(None)
    }

    /// Get the height of a specific block
    pub fn get_block_height(&self, block_hash: &str) -> Result<usize> {
        if let Some(block) = self.get_block(block_hash)? {
            Ok(block.get_height())
        } else {
            Err(BlockchainError::InvalidBlock(format!(
                "Block not found: {block_hash}"
            )))
        }
    }

    /// Check if a block is in the main chain
    pub fn is_in_main_chain(&self, block_hash: &str) -> Result<bool> {
        // Walk back from current tip to see if we encounter this block
        let mut current_hash = self.get_tip_hash();

        while current_hash != "None" {
            // Genesis block has "None" as previous hash
            if current_hash == block_hash {
                return Ok(true);
            }

            if let Some(block) = self.get_block(&current_hash)? {
                current_hash = block.get_pre_block_hash();
            } else {
                break;
            }
        }

        Ok(false)
    }

    /// Remove a block from the blockchain (for reorganization)
    pub fn remove_block(&self, block_hash: &str) -> Result<()> {
        let block_tree = self
            .db
            .open_tree(BLOCKS_TREE)
            .map_err(|e| BlockchainError::Database(format!("Failed to open blocks tree: {e}")))?;

        // Get the block to find its parent
        let block = self.get_block(block_hash)?.ok_or_else(|| {
            BlockchainError::InvalidBlock(format!("Cannot remove non-existent block: {block_hash}"))
        })?;

        // Remove the block from storage
        block_tree
            .remove(block_hash)
            .map_err(|e| BlockchainError::Database(format!("Failed to remove block: {e}")))?;

        // Update tip if this was the tip block
        if self.get_tip_hash() == block_hash {
            let new_tip = block.get_pre_block_hash();
            block_tree
                .insert(TIP_BLOCK_HASH_KEY, new_tip.as_bytes())
                .map_err(|e| BlockchainError::Database(format!("Failed to update tip: {e}")))?;
            self.set_tip_hash(&new_tip);
        }

        Ok(())
    }

    /// Synchronize blockchain with another node's blockchain
    pub fn sync_with_peer(&self, peer_blocks: &[Block]) -> Result<bool> {
        let mut updated = false;

        // Sort blocks by height to process in order
        let mut sorted_blocks = peer_blocks.to_vec();
        sorted_blocks.sort_by_key(|b| b.get_height());

        for block in sorted_blocks {
            // Check if we already have this block
            if !self.block_exists(block.get_hash())? {
                // Validate the block before adding
                if self.validate_block_for_sync(&block)? {
                    // Check for fork resolution
                    if self.should_reorganize(&block)? {
                        self.reorganize_to_block(&block)?;
                        updated = true;
                    } else {
                        self.add_block(&block)?;
                        updated = true;
                    }
                    info!("Synchronized block: {}", block.get_hash());
                }
            }
        }

        Ok(updated)
    }

    /// Check if we should reorganize to a new block (simple longest chain rule)
    fn should_reorganize(&self, new_block: &Block) -> Result<bool> {
        let current_height = self.get_best_height()?;
        Ok(new_block.get_height() > current_height)
    }

    /// Reorganize blockchain to a new block (simple implementation)
    fn reorganize_to_block(&self, new_block: &Block) -> Result<()> {
        // For simplicity, we'll just add the block if it extends the chain
        // In a full implementation, this would handle complex reorganizations
        self.add_block(new_block)
    }

    /// Validate a block for synchronization
    fn validate_block_for_sync(&self, block: &Block) -> Result<bool> {
        // Check if previous block exists (unless it's genesis)
        if block.get_pre_block_hash() != "None"
            && !self.block_exists(&block.get_pre_block_hash())?
        {
            return Ok(false); // Previous block not found
        }

        // Validate proof of work
        if !crate::core::ProofOfWork::validate(block) {
            return Ok(false); // Invalid proof of work
        }

        // Validate merkle root
        if !block.verify_merkle_root()? {
            return Ok(false); // Invalid merkle root
        }

        // Validate all transactions in the block
        for transaction in block.get_transactions() {
            if !transaction.verify(self) {
                return Ok(false); // Invalid transaction
            }
        }

        Ok(true)
    }

    // This is critical - I need to prevent double-spending within a single block
    // Someone could try to spend the same UTXO multiple times in different transactions
    fn check_for_double_spending(&self, transactions: &[Transaction]) -> Result<()> {
        use std::collections::HashSet;
        let mut spent_outputs: HashSet<(Vec<u8>, usize)> = HashSet::new();

        for (tx_index, transaction) in transactions.iter().enumerate() {
            // I skip coinbase transactions since they don't spend existing outputs
            if transaction.is_coinbase() {
                continue;
            }

            // I check each input to see if it's already been spent in this block
            for input in transaction.get_vin() {
                let output_reference = (input.get_txid().to_vec(), input.get_vout());
                
                // If I've already seen this output being spent, that's a double-spend!
                if spent_outputs.contains(&output_reference) {
                    return Err(BlockchainError::Transaction(format!(
                        "Double-spending detected in transaction {}: output {}:{} already spent in this block",
                        tx_index,
                        HEXLOWER.encode(input.get_txid()),
                        input.get_vout()
                    )));
                }

                // I mark this output as spent
                spent_outputs.insert(output_reference);
            }
        }

        // If I get here, no double-spending was detected
        Ok(())
    }

    // I also need to check if an output has already been spent in the blockchain
    pub fn is_output_spent(&self, txid: &[u8], vout: usize) -> bool {
        // I iterate through all blocks to see if this output has been spent
        let mut iterator = self.iterator();
        while let Some(block) = iterator.next() {
            for transaction in block.get_transactions() {
                // I skip coinbase transactions
                if transaction.is_coinbase() {
                    continue;
                }

                // I check if any input spends the output I'm looking for
                for input in transaction.get_vin() {
                    if input.get_txid() == txid && input.get_vout() == vout {
                        return true; // This output has been spent
                    }
                }
            }
        }
        false // This output hasn't been spent yet
    }

    // I want to be able to validate that a transaction's inputs haven't been spent
    pub fn validate_transaction_inputs(&self, transaction: &Transaction) -> Result<bool> {
        if transaction.is_coinbase() {
            return Ok(true); // Coinbase transactions don't have real inputs to validate
        }

        for input in transaction.get_vin() {
            // I check if this input has already been spent
            if self.is_output_spent(input.get_txid(), input.get_vout()) {
                return Err(BlockchainError::Transaction(format!(
                    "Input already spent: {}:{}",
                    HEXLOWER.encode(input.get_txid()),
                    input.get_vout()
                )));
            }

            // I also verify that the referenced transaction exists
            if self.find_transaction(input.get_txid()).is_none() {
                return Err(BlockchainError::Transaction(format!(
                    "Referenced transaction not found: {}",
                    HEXLOWER.encode(input.get_txid())
                )));
            }
        }

        Ok(true)
    }
}

pub struct BlockchainIterator {
    db: Db,
    current_hash: String,
}

impl Iterator for BlockchainIterator {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        let block_tree = self.db.open_tree(BLOCKS_TREE).ok()?;
        let data = block_tree.get(self.current_hash.clone()).ok()??;
        let block = Block::deserialize(data.to_vec().as_slice()).ok()?;
        self.current_hash = block.get_pre_block_hash().clone();
        Some(block)
    }
}

impl BlockchainIterator {
    fn new(tip_hash: String, db: Db) -> BlockchainIterator {
        BlockchainIterator {
            current_hash: tip_hash,
            db,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Block> {
        let block_tree = self.db.open_tree(BLOCKS_TREE).ok()?;
        let data = block_tree.get(self.current_hash.clone()).ok()??;
        let block = Block::deserialize(data.to_vec().as_slice()).ok()?;
        self.current_hash = block.get_pre_block_hash().clone();
        Some(block)
    }
}
