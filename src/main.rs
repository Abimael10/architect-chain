// This is my main entry point for the blockchain CLI application
// I'm importing all the core components I built for this blockchain
use architect_chain::cli::{FeeModeArg, FeePriorityArg};
use architect_chain::{
    convert_address, hash_pub_key, send_tx, utils, validate_address, Blockchain, Command,
    DynamicFeeConfig, FeeCalculator, FeeMode, FeePriority, Opt, Server, Transaction, UTXOSet,
    Wallets, ADDRESS_CHECK_SUM_LEN, CENTRAL_NODE, GLOBAL_CONFIG,
};
use clap::Parser;
use data_encoding::HEXLOWER;
use log::{error, LevelFilter};
use std::process;

// I use this constant to check if the user wants to mine immediately after sending a transaction
const MINE_TRUE: usize = 1;

fn main() {
    // I initialize logging so I can see what's happening in my blockchain
    // Setting it to Info level gives me enough detail without being too verbose
    env_logger::builder().filter_level(LevelFilter::Info).init();

    // I parse the command line arguments using clap - this gives me a nice CLI interface
    let opt = Opt::parse();

    // I run the actual command and handle any errors that might occur
    // If something goes wrong, I log the error and exit with code 1
    if let Err(e) = run_command(opt.command) {
        error!("Error: {e}");
        process::exit(1);
    }
}

// This is where I handle all the different CLI commands
// Each command corresponds to a different blockchain operation I want to perform
fn run_command(command: Command) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        // When I want to create a new blockchain, this is the genesis block creation
        Command::Createblockchain { address } => {
            // First, I validate that the address format is correct (Bitcoin-compatible)
            if !validate_address(&address) {
                return Err(format!("Invalid address: {address}").into());
            }
            // I create the blockchain with this address receiving the genesis block reward
            let blockchain = Blockchain::create_blockchain(&address)?;
            // I need to build the UTXO set from the blockchain for efficient balance lookups
            let utxo_set = UTXOSet::new(blockchain);
            utxo_set.reindex();
            println!("Done!");
        }
        // When I want to create a new wallet for storing my cryptocurrency
        Command::Createwallet => {
            // I load the wallet collection (or create it if it doesn't exist)
            let mut wallet = Wallets::new();
            // I generate a new ECDSA key pair and derive a Bitcoin-compatible address
            let address = wallet.create_wallet()?;
            println!("Your new address: {address}")
        }
        // When I want to check how much cryptocurrency an address has
        Command::GetBalance { address } => {
            // First, I validate the address format
            if !validate_address(&address) {
                return Err(format!("Invalid address: {address}").into());
            }

            // I decode the Base58 address to get the public key hash
            let payload = utils::base58_decode(&address)?;
            if payload.len() < ADDRESS_CHECK_SUM_LEN + 1 {
                return Err("Address too short".into());
            }
            // I extract the public key hash from the address (removing version byte and checksum)
            let pub_key_hash = &payload[1..payload.len() - ADDRESS_CHECK_SUM_LEN];

            // I load the blockchain and build the UTXO set for efficient lookups
            let blockchain = Blockchain::new_blockchain()?;
            let utxo_set = UTXOSet::new(blockchain);
            // I find all unspent transaction outputs belonging to this address
            let utxos = utxo_set.find_utxo(pub_key_hash);
            // I sum up all the values to get the total balance
            let mut balance = 0;
            for utxo in utxos {
                balance += utxo.get_value();
            }
            println!("Balance of {address}: {balance}");
        }
        // When I want to see all the wallet addresses I have created
        Command::ListAddresses => {
            // I load my wallet collection
            let wallets = Wallets::new();
            // I iterate through all addresses and print them
            for address in wallets.get_addresses() {
                println!("{address}")
            }
        }
        // When I want to send cryptocurrency from one address to another
        Command::Send {
            from,
            to,
            amount,
            mine,
            priority,
        } => {
            // I validate both addresses to make sure they're properly formatted
            if !validate_address(&from) {
                return Err(format!("Invalid sender address: {from}").into());
            }
            if !validate_address(&to) {
                return Err(format!("Invalid recipient address: {to}").into());
            }
            if amount == 0 {
                return Err("Amount must be positive".into());
            }

            // I load the blockchain and create the UTXO set for transaction validation
            let blockchain = Blockchain::new_blockchain()?;
            let utxo_set = UTXOSet::new(blockchain.clone());

            // I convert the CLI priority argument to my internal priority enum
            let fee_priority = match priority {
                Some(FeePriorityArg::Low) => FeePriority::Low,
                Some(FeePriorityArg::Normal) => FeePriority::Normal,
                Some(FeePriorityArg::High) => FeePriority::High,
                Some(FeePriorityArg::Urgent) => FeePriority::Urgent,
                None => FeePriority::Normal, // Default to normal priority
            };

            // I create the transaction with the appropriate fee calculation method
            let transaction = if priority.is_some() {
                // If priority is specified, I use the priority-based fee calculation
                Transaction::new_utxo_transaction_with_priority(
                    &from,
                    &to,
                    amount,
                    fee_priority,
                    &utxo_set,
                )?
            } else {
                // Otherwise, I use the default fee calculation
                Transaction::new_utxo_transaction(&from, &to, amount, &utxo_set)?
            };

            // I decide whether to mine the transaction immediately or send it to the network
            if mine == MINE_TRUE {
                // If mining immediately, I create a new block with this transaction
                let block = blockchain.mine_block_with_fees(&[transaction], &from)?;
                // I update the UTXO set with the new block
                utxo_set.update(&block);
            } else {
                // Otherwise, I broadcast the transaction to the P2P network
                send_tx(CENTRAL_NODE, &transaction);
            }
            println!("Success!")
        }
        // When I want to see the entire blockchain history (useful for debugging)
        Command::Printchain => {
            // I create an iterator that walks through the blockchain from newest to oldest
            let mut block_iterator = Blockchain::new_blockchain()?.iterator();
            loop {
                let option = block_iterator.next();
                if let Some(block) = option {
                    // I print the block header information
                    println!("Pre block hash: {}", block.get_pre_block_hash());
                    println!("Cur block hash: {}", block.get_hash());
                    println!("Cur block Timestamp: {}", block.get_timestamp());

                    // I iterate through all transactions in this block
                    for tx in block.get_transactions() {
                        let cur_txid_hex = HEXLOWER.encode(tx.get_id());
                        println!("- Transaction txid_hex: {cur_txid_hex}");

                        // For regular transactions (not coinbase), I show the inputs
                        if !tx.is_coinbase() {
                            for input in tx.get_vin() {
                                let txid_hex = HEXLOWER.encode(input.get_txid());
                                // I convert the public key to an address for readability
                                let pub_key_hash = hash_pub_key(input.get_pub_key());
                                let address = convert_address(pub_key_hash.as_slice());
                                println!(
                                    "-- Input txid = {}, vout = {}, from = {}",
                                    txid_hex,
                                    input.get_vout(),
                                    address,
                                )
                            }
                        }
                        // I show all outputs (where the money is going)
                        for output in tx.get_vout() {
                            let pub_key_hash = output.get_pub_key_hash();
                            let address = convert_address(pub_key_hash);
                            println!("-- Output value = {}, to = {}", output.get_value(), address,)
                        }
                    }
                    println!()
                } else {
                    // No more blocks to iterate through
                    break;
                }
            }
        }
        // When I want to rebuild the UTXO index (useful if it gets corrupted)
        Command::Reindexutxo => {
            // I load the blockchain
            let blockchain = Blockchain::new_blockchain()?;
            // I create a new UTXO set and rebuild it from scratch
            let utxo_set = UTXOSet::new(blockchain);
            utxo_set.reindex();
            // I count how many transactions are in the UTXO set for verification
            let count = utxo_set.count_transactions();
            println!("Done! There are {count} transactions in the UTXO set.");
        }
        // When I want to start a blockchain node (either as a miner or validator)
        Command::StartNode { miner } => {
            // I configure the node based on the network address it should listen on
            let socket_addr = GLOBAL_CONFIG.get_node_addr();
            let node_id = GLOBAL_CONFIG.extract_node_id_from_addr();
            GLOBAL_CONFIG.set_node_id(node_id.clone());

            // If a miner address is provided, this node will participate in mining
            if let Some(addr) = miner {
                if !validate_address(&addr) {
                    return Err(format!("Invalid miner address: {addr}").into());
                }
                println!("Mining is on. Address to receive rewards: {addr}");
                GLOBAL_CONFIG.set_mining_addr(addr);
            }

            // I need to load the blockchain for this specific node
            // Each node has its own database to ensure proper isolation
            let blockchain = if let Some(existing_node_id) = GLOBAL_CONFIG.get_node_id() {
                match Blockchain::new_blockchain_with_node_id(&existing_node_id) {
                    Ok(bc) => bc,
                    Err(_) => {
                        // If no blockchain exists for this node, I need to either:
                        // 1. Create a new one (if this is the first node)
                        // 2. Sync from other nodes (if this is a joining node)
                        println!("No blockchain found for node {existing_node_id}. Use 'createblockchain' first or sync from network.");
                        return Err("No blockchain found for this node".into());
                    }
                }
            } else {
                // Fallback to the default blockchain if no node ID is set
                Blockchain::new_blockchain()?
            };

            // I create the P2P server and start listening for connections
            let server = Server::new(blockchain);
            server
                .run(&socket_addr)
                .map_err(|e| format!("Server error: {e}"))?
        }
        // When I want to estimate how much fee I should pay for a transaction
        Command::EstimateFee { priority } => {
            // I convert the CLI priority to my internal enum
            let fee_priority = match priority {
                FeePriorityArg::Low => FeePriority::Low,
                FeePriorityArg::Normal => FeePriority::Normal,
                FeePriorityArg::High => FeePriority::High,
                FeePriorityArg::Urgent => FeePriority::Urgent,
            };

            // I use my fee calculator to estimate the appropriate fee
            let estimated_fee = FeeCalculator::estimate_fee(fee_priority);
            println!("Estimated fee for {priority} priority: {estimated_fee} coins");
        }
        // When I want to check the current fee system configuration and statistics
        Command::FeeStatus => {
            // I get a summary of the current fee configuration
            let config_summary = FeeCalculator::get_config_summary();
            println!("Fee System Status:");
            println!("  {config_summary}");

            // I also show fee statistics if available
            if let Some(stats) = FeeCalculator::get_fee_statistics() {
                println!();
                print!("{stats}");
            }
        }
        // When I want to change how fees are calculated (fixed vs dynamic)
        Command::SetFeeMode { mode } => {
            // I convert the CLI argument to my internal fee mode enum
            let new_mode = match mode {
                FeeModeArg::Fixed(amount) => FeeMode::Fixed { amount },
                FeeModeArg::Dynamic => FeeMode::Dynamic {
                    config: DynamicFeeConfig::default(),
                },
            };

            // I switch the fee calculator to the new mode
            FeeCalculator::switch_fee_mode(new_mode)?;
            println!("Fee mode updated successfully");
            println!("New configuration: {}", FeeCalculator::get_config_summary());
        }
    }
    Ok(())
}
