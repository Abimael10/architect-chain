//! Blockchain integration tests
//!
//! Tests the core blockchain functionality that was implemented,
//! focusing on the critical features that make this a working blockchain.

use architect_chain::core::{Block, Blockchain, ProofOfWork, Transaction};
use architect_chain::storage::UTXOSet;
use architect_chain::wallet::Wallets;
use tempfile::tempdir;

#[test]
fn test_proof_of_work_validation() {
    let test_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
    let coinbase_tx = Transaction::new_coinbase_tx(test_address).unwrap();

    // Create a block with valid proof of work
    let block = Block::new_block(
        "prev_hash".to_string(),
        &[coinbase_tx],
        1,
        1, // Easy difficulty for test
    )
    .unwrap();

    // The mined block should pass proof of work validation
    assert!(ProofOfWork::validate(&block));

    // Block should have valid merkle root
    assert!(block.verify_merkle_root().unwrap());
}

#[test]
fn test_blockchain_creation_and_mining() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_blockchain");

    let test_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
    let blockchain =
        Blockchain::create_blockchain_with_path(test_address, db_path.to_str().unwrap()).unwrap();

    // Should start with genesis block
    assert_eq!(blockchain.get_best_height().unwrap(), 0);

    // Mine a new block
    let coinbase_tx = Transaction::new_coinbase_tx(test_address).unwrap();
    let block = blockchain
        .mine_block_with_fees(&[coinbase_tx], test_address)
        .unwrap();

    // Block should be valid and added to chain
    assert_eq!(block.get_height(), 1);
    assert_eq!(blockchain.get_best_height().unwrap(), 1);
    assert!(ProofOfWork::validate(&block));
}

#[test]
fn test_transaction_creation_and_validation() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_blockchain");

    let mut wallets = Wallets::new();
    let sender_address = wallets.create_wallet().unwrap();
    let recipient_address = wallets.create_wallet().unwrap();

    let blockchain =
        Blockchain::create_blockchain_with_path(&sender_address, db_path.to_str().unwrap())
            .unwrap();

    // Mine initial block to create UTXOs
    let coinbase_tx = Transaction::new_coinbase_tx(&sender_address).unwrap();
    blockchain
        .mine_block_with_fees(&[coinbase_tx], &sender_address)
        .unwrap();

    let utxo_set = UTXOSet::new(blockchain.clone());
    utxo_set.reindex();

    // Create a transaction
    let tx = Transaction::new_utxo_transaction(
        &sender_address,
        &recipient_address,
        1000000, // 0.01 coins
        &utxo_set,
    )
    .unwrap();

    // Transaction should be valid
    assert!(tx.verify(&blockchain));
    assert!(!tx.is_coinbase());
    assert!(tx.get_fee() > 0);

    // Mine the transaction
    let block = blockchain
        .mine_block_with_fees(&[tx], &sender_address)
        .unwrap();
    assert_eq!(block.get_height(), 2);
    assert_eq!(block.get_transactions().len(), 2); // coinbase + transaction
}

#[test]
fn test_blockchain_synchronization() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_blockchain");

    let test_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";

    // Create blockchain and mine blocks
    let blockchain =
        Blockchain::create_blockchain_with_path(test_address, db_path.to_str().unwrap()).unwrap();

    // Mine initial block
    let coinbase_tx = Transaction::new_coinbase_tx(test_address).unwrap();
    let block1 = blockchain
        .mine_block_with_fees(&[coinbase_tx], test_address)
        .unwrap();
    assert_eq!(blockchain.get_best_height().unwrap(), 1);

    // Create additional blocks to sync
    let mut additional_blocks = Vec::new();
    let mut prev_hash = block1.get_hash().to_string();

    for i in 2..=4 {
        let coinbase_tx = Transaction::new_coinbase_tx(test_address).unwrap();
        let block = Block::new_block(
            prev_hash,
            &[coinbase_tx],
            i,
            1, // Easy difficulty
        )
        .unwrap();
        prev_hash = block.get_hash().to_string();
        additional_blocks.push(block);
    }

    // Sync with additional blocks
    let sync_result = blockchain.sync_with_peer(&additional_blocks).unwrap();
    assert!(sync_result);
    assert!(blockchain.get_best_height().unwrap() >= 4);
}

#[test]
fn test_fork_resolution() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_blockchain");

    let test_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
    let blockchain =
        Blockchain::create_blockchain_with_path(test_address, db_path.to_str().unwrap()).unwrap();

    // Mine initial block
    let coinbase_tx = Transaction::new_coinbase_tx(test_address).unwrap();
    blockchain
        .mine_block_with_fees(&[coinbase_tx], test_address)
        .unwrap();
    assert_eq!(blockchain.get_best_height().unwrap(), 1);

    // Create a longer competing chain
    let mut competing_blocks = Vec::new();
    let mut prev_hash = blockchain.get_tip_hash();

    for i in 2..=4 {
        let coinbase_tx = Transaction::new_coinbase_tx(test_address).unwrap();
        let block = Block::new_block(
            prev_hash,
            &[coinbase_tx],
            i,
            1, // Easy difficulty
        )
        .unwrap();
        prev_hash = block.get_hash().to_string();
        competing_blocks.push(block);
    }

    // Sync with longer chain
    let sync_result = blockchain.sync_with_peer(&competing_blocks).unwrap();
    assert!(sync_result);
    assert!(blockchain.get_best_height().unwrap() >= 4);
}

#[test]
fn test_block_validation_during_sync() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_blockchain");

    let test_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
    let blockchain =
        Blockchain::create_blockchain_with_path(test_address, db_path.to_str().unwrap()).unwrap();

    // Create a valid block
    let coinbase_tx = Transaction::new_coinbase_tx(test_address).unwrap();
    let valid_block =
        Block::new_block(blockchain.get_tip_hash(), &[coinbase_tx.clone()], 1, 1).unwrap();

    // Create an invalid block (wrong previous hash)
    let invalid_block =
        Block::new_block("wrong_previous_hash".to_string(), &[coinbase_tx], 1, 1).unwrap();

    // Valid block should sync successfully
    let valid_sync = blockchain.sync_with_peer(&[valid_block]).unwrap();
    assert!(valid_sync);
    assert_eq!(blockchain.get_best_height().unwrap(), 1);

    // Invalid block should be rejected (blockchain height shouldn't change)
    blockchain.sync_with_peer(&[invalid_block]).unwrap();
    assert_eq!(blockchain.get_best_height().unwrap(), 1);
}

#[test]
fn test_difficulty_adjustment() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_blockchain");

    let test_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
    let blockchain =
        Blockchain::create_blockchain_with_path(test_address, db_path.to_str().unwrap()).unwrap();

    // Mine blocks to trigger difficulty adjustment
    for i in 1..=12 {
        let coinbase_tx = Transaction::new_coinbase_tx(test_address).unwrap();
        let block = blockchain
            .mine_block_with_fees(&[coinbase_tx], test_address)
            .unwrap();

        assert_eq!(block.get_height(), i);
        assert!(ProofOfWork::validate(&block));

        // Difficulty should be within valid bounds
        let difficulty = block.get_difficulty();
        assert!((1..=12).contains(&difficulty));
    }

    assert_eq!(blockchain.get_best_height().unwrap(), 12);
}

#[test]
fn test_fee_calculation() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_blockchain");

    let mut wallets = Wallets::new();
    let sender_address = wallets.create_wallet().unwrap();
    let recipient_address = wallets.create_wallet().unwrap();

    let blockchain =
        Blockchain::create_blockchain_with_path(&sender_address, db_path.to_str().unwrap())
            .unwrap();

    // Mine initial block
    let coinbase_tx = Transaction::new_coinbase_tx(&sender_address).unwrap();
    blockchain
        .mine_block_with_fees(&[coinbase_tx], &sender_address)
        .unwrap();

    let utxo_set = UTXOSet::new(blockchain.clone());
    utxo_set.reindex();

    // Create transaction with high priority
    let tx = Transaction::new_utxo_transaction_with_priority(
        &sender_address,
        &recipient_address,
        500000,
        architect_chain::core::FeePriority::High,
        &utxo_set,
    )
    .unwrap();

    let fee = tx.get_fee();
    assert!(fee > 0);

    // Mine the transaction
    blockchain
        .mine_block_with_fees(&[tx], &sender_address)
        .unwrap();
    utxo_set.reindex();

    // Verify recipient received the amount
    let recipient_balance = get_balance(&utxo_set, &recipient_address);
    assert_eq!(recipient_balance, 500000);
}

#[test]
fn test_blockchain_persistence() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("persistent_blockchain");
    let db_path_str = db_path.to_str().unwrap();

    let test_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";

    // Create blockchain and mine blocks
    {
        let blockchain =
            Blockchain::create_blockchain_with_path(test_address, db_path_str).unwrap();

        for _ in 1..=3 {
            let coinbase_tx = Transaction::new_coinbase_tx(test_address).unwrap();
            blockchain
                .mine_block_with_fees(&[coinbase_tx], test_address)
                .unwrap();
        }

        assert_eq!(blockchain.get_best_height().unwrap(), 3);
    }

    // Reopen blockchain from same path
    {
        let blockchain = Blockchain::new_blockchain_with_path(db_path_str).unwrap();
        assert_eq!(blockchain.get_best_height().unwrap(), 3);

        // Continue mining
        let coinbase_tx = Transaction::new_coinbase_tx(test_address).unwrap();
        blockchain
            .mine_block_with_fees(&[coinbase_tx], test_address)
            .unwrap();
        assert_eq!(blockchain.get_best_height().unwrap(), 4);
    }
}

// Helper function
fn get_balance(utxo_set: &UTXOSet, address: &str) -> u64 {
    use architect_chain::utils;
    use architect_chain::ADDRESS_CHECK_SUM_LEN;

    let payload = utils::base58_decode(address).unwrap();
    let pub_key_hash = &payload[1..payload.len() - ADDRESS_CHECK_SUM_LEN];
    let utxos = utxo_set.find_utxo(pub_key_hash);

    utxos.iter().map(|utxo| utxo.get_value()).sum()
}
