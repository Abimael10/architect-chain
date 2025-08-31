use crate::config::GLOBAL_CONFIG;
use crate::core::{Block, Blockchain, Transaction};
use crate::error::{BlockchainError, Result};
use crate::network::SimplePeerManager;
use crate::storage::{BlockInTransit, MemoryPool, UTXOSet};
use data_encoding::HEXLOWER;
use log::{error, info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use std::io::{BufReader, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const NODE_VERSION: usize = 1;
pub const CENTRAL_NODE: &str = "127.0.0.1:2001";
pub const TRANSACTION_THRESHOLD: usize = 10;
const TCP_WRITE_TIMEOUT: u64 = 5000;

/// Simplified server for blockchain P2P networking
pub struct Server {
    /// Core blockchain instance
    blockchain: Blockchain,
    /// Simple peer manager
    peer_manager: Arc<SimplePeerManager>,
}

/// Global memory pool
static GLOBAL_MEMORY_POOL: Lazy<MemoryPool> = Lazy::new(MemoryPool::new);

/// Global blocks in transit
static GLOBAL_BLOCKS_IN_TRANSIT: Lazy<BlockInTransit> = Lazy::new(BlockInTransit::new);

/// P2P message types
#[derive(Debug, Serialize, Deserialize)]
pub enum OpType {
    Tx,
    Block,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Package {
    Block {
        addr_from: String,
        block: Vec<u8>,
    },
    GetBlocks {
        addr_from: String,
    },
    GetData {
        addr_from: String,
        op_type: OpType,
        id: Vec<u8>,
    },
    Inv {
        addr_from: String,
        op_type: OpType,
        items: Vec<Vec<u8>>,
    },
    Tx {
        addr_from: String,
        transaction: Vec<u8>,
    },
    Version {
        addr_from: String,
        version: usize,
        best_height: usize,
    },
}

impl Server {
    /// Create a new simplified server
    pub fn new(blockchain: Blockchain) -> Self {
        let peer_manager = Arc::new(SimplePeerManager::new(8, 2001));

        Self {
            blockchain,
            peer_manager,
        }
    }

    /// Run the server
    pub fn run(&self, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr)
            .map_err(|e| BlockchainError::Network(format!("Failed to bind to {addr}: {e}")))?;

        info!("Server listening on {addr}");

        // If not central node, connect to network
        if addr != CENTRAL_NODE {
            self.connect_to_network()?;
        }

        // Start peer discovery in background
        self.start_peer_discovery();

        // Accept incoming connections
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let peer_addr = match stream.peer_addr() {
                        Ok(addr) => addr,
                        Err(e) => {
                            error!("Failed to get peer address: {e}");
                            continue;
                        }
                    };

                    // Check if we should accept this connection
                    if !self
                        .peer_manager
                        .should_accept_connection()
                        .unwrap_or(false)
                    {
                        warn!("Rejecting connection from {peer_addr}: connection limit reached");
                        continue;
                    }

                    // Record the connection
                    if let Err(e) = self.peer_manager.record_connection(peer_addr) {
                        warn!("Failed to record connection: {e}");
                    }

                    // Spawn handler thread
                    let blockchain = self.blockchain.clone();
                    let peer_manager = Arc::clone(&self.peer_manager);

                    thread::spawn(move || {
                        let result = Self::handle_connection(blockchain, stream, peer_addr);

                        // Remove connection when done
                        if let Err(e) = peer_manager.record_disconnection(peer_addr) {
                            warn!("Failed to record disconnection: {e}");
                        }

                        if let Err(e) = result {
                            error!("Error handling connection from {peer_addr}: {e}");
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting connection: {e}");
                }
            }
        }

        Ok(())
    }

    /// Connect to the network on startup
    fn connect_to_network(&self) -> Result<()> {
        if let Ok(best_height) = self.blockchain.get_best_height() {
            Self::send_version(CENTRAL_NODE, best_height)?;
        }
        Ok(())
    }

    /// Start peer discovery in background
    fn start_peer_discovery(&self) {
        let peer_manager = Arc::clone(&self.peer_manager);

        thread::spawn(move || {
            loop {
                // Perform peer discovery every 5 minutes
                thread::sleep(Duration::from_secs(300));

                if let Ok(peers) = peer_manager.get_peers_to_connect() {
                    for peer_addr in peers {
                        // Try to connect to discovered peers
                        if let Err(e) = Self::send_version(&peer_addr.to_string(), 0) {
                            error!("Failed to connect to peer {peer_addr}: {e}");
                        }
                    }
                }
            }
        });
    }

    /// Handle an individual connection
    fn handle_connection(
        blockchain: Blockchain,
        stream: TcpStream,
        peer_addr: SocketAddr,
    ) -> Result<()> {
        // Set connection timeout
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .map_err(|e| BlockchainError::Network(format!("Failed to set read timeout: {e}")))?;

        let reader = BufReader::new(&stream);
        let pkg_reader = Deserializer::from_reader(reader).into_iter::<Package>();

        for pkg in pkg_reader {
            let pkg = pkg.map_err(|e| {
                BlockchainError::Network(format!("Failed to deserialize package: {e}"))
            })?;

            info!("Received request from {peer_addr}: {pkg:?}");

            // Process the message
            if let Err(e) = Self::process_message(&blockchain, pkg) {
                error!("Error processing message from {peer_addr}: {e}");
            }
        }

        let _ = stream.shutdown(Shutdown::Both);
        Ok(())
    }

    /// Process an incoming message
    fn process_message(blockchain: &Blockchain, pkg: Package) -> Result<()> {
        match pkg {
            Package::Block { addr_from, block } => {
                Self::handle_block_message(blockchain, addr_from, block)
            }
            Package::GetBlocks { addr_from } => {
                Self::handle_get_blocks_message(blockchain, addr_from)
            }
            Package::GetData {
                addr_from,
                op_type,
                id,
            } => Self::handle_get_data_message(blockchain, addr_from, op_type, id),
            Package::Inv {
                addr_from,
                op_type,
                items,
            } => Self::handle_inv_message(addr_from, op_type, items),
            Package::Tx {
                addr_from: _,
                transaction,
            } => Self::handle_tx_message(blockchain, transaction),
            Package::Version {
                addr_from,
                version: _,
                best_height,
            } => Self::handle_version_message(blockchain, addr_from, best_height),
        }
    }

    /// Handle incoming block message
    fn handle_block_message(
        blockchain: &Blockchain,
        addr_from: String,
        block_data: Vec<u8>,
    ) -> Result<()> {
        let block = Block::deserialize(&block_data)
            .map_err(|e| BlockchainError::Network(format!("Failed to deserialize block: {e}")))?;

        // Add block to blockchain
        blockchain
            .add_block(&block)
            .map_err(|e| BlockchainError::Network(format!("Failed to add block: {e}")))?;

        info!("Added block {} from {}", block.get_hash(), addr_from);

        // Handle blocks in transit
        if !GLOBAL_BLOCKS_IN_TRANSIT.is_empty() {
            if let Some(block_hash) = GLOBAL_BLOCKS_IN_TRANSIT.first() {
                Self::send_get_data(&addr_from, OpType::Block, &block_hash)?;
                GLOBAL_BLOCKS_IN_TRANSIT.remove(&block_hash);
            }
        } else {
            let utxo_set = UTXOSet::new(blockchain.clone());
            utxo_set.reindex();
        }

        Ok(())
    }

    /// Handle get blocks message
    fn handle_get_blocks_message(blockchain: &Blockchain, addr_from: String) -> Result<()> {
        let blocks = blockchain.get_block_hashes();
        Self::send_inv(&addr_from, OpType::Block, &blocks)
    }

    /// Handle get data message
    fn handle_get_data_message(
        blockchain: &Blockchain,
        addr_from: String,
        op_type: OpType,
        id: Vec<u8>,
    ) -> Result<()> {
        match op_type {
            OpType::Block => match blockchain.get_block_by_bytes(&id) {
                Ok(Some(block)) => {
                    Self::send_block(&addr_from, &block)?;
                }
                Ok(None) => {
                    info!("Block not found for requested hash");
                }
                Err(e) => {
                    error!("Failed to get block: {e}");
                }
            },
            OpType::Tx => {
                let txid_hex = HEXLOWER.encode(&id);
                if let Some(tx) = GLOBAL_MEMORY_POOL.get(&txid_hex) {
                    Self::send_tx(&addr_from, &tx)?;
                }
            }
        }
        Ok(())
    }

    /// Handle inventory message
    fn handle_inv_message(addr_from: String, op_type: OpType, items: Vec<Vec<u8>>) -> Result<()> {
        match op_type {
            OpType::Block => {
                GLOBAL_BLOCKS_IN_TRANSIT.add_blocks(&items);
                if let Some(block_hash) = items.first() {
                    Self::send_get_data(&addr_from, OpType::Block, block_hash)?;
                    GLOBAL_BLOCKS_IN_TRANSIT.remove(block_hash);
                }
            }
            OpType::Tx => {
                if let Some(txid) = items.first() {
                    let txid_hex = HEXLOWER.encode(txid);
                    if !GLOBAL_MEMORY_POOL.contains(&txid_hex) {
                        Self::send_get_data(&addr_from, OpType::Tx, txid)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle transaction message
    fn handle_tx_message(blockchain: &Blockchain, transaction_data: Vec<u8>) -> Result<()> {
        let tx = Transaction::deserialize(&transaction_data).map_err(|e| {
            BlockchainError::Network(format!("Failed to deserialize transaction: {e}"))
        })?;

        GLOBAL_MEMORY_POOL.add(tx);

        // Check if we should mine a block
        if GLOBAL_MEMORY_POOL.len() >= TRANSACTION_THRESHOLD && GLOBAL_CONFIG.is_miner() {
            Self::try_mine_block(blockchain)?;
        }

        Ok(())
    }

    /// Handle version message
    fn handle_version_message(
        blockchain: &Blockchain,
        addr_from: String,
        best_height: usize,
    ) -> Result<()> {
        info!("Version message from {addr_from}, best_height={best_height}");

        // Handle blockchain synchronization
        match blockchain.get_best_height() {
            Ok(local_best_height) => {
                if local_best_height < best_height {
                    Self::send_get_blocks(&addr_from)?;
                }
                if local_best_height > best_height {
                    Self::send_version(&addr_from, local_best_height)?;
                }
            }
            Err(e) => {
                error!("Failed to get local best height: {e}");
            }
        }

        Ok(())
    }

    /// Try to mine a block with current transactions
    fn try_mine_block(blockchain: &Blockchain) -> Result<()> {
        let mining_address = GLOBAL_CONFIG
            .get_mining_addr()
            .ok_or_else(|| BlockchainError::Network("Mining address not configured".to_string()))?;

        let coinbase_tx = Transaction::new_coinbase_tx(&mining_address).map_err(|e| {
            BlockchainError::Network(format!("Failed to create coinbase transaction: {e}"))
        })?;

        let mut txs = GLOBAL_MEMORY_POOL.get_all();
        txs.push(coinbase_tx);

        let new_block = blockchain
            .mine_block(&txs)
            .map_err(|e| BlockchainError::Network(format!("Failed to mine block: {e}")))?;

        let utxo_set = UTXOSet::new(blockchain.clone());
        utxo_set.reindex();
        info!("New block {} is mined!", new_block.get_hash());

        // Clear mined transactions from memory pool
        for tx in &txs {
            let txid_hex = HEXLOWER.encode(tx.get_id());
            GLOBAL_MEMORY_POOL.remove(&txid_hex);
        }

        Ok(())
    }

    /// Send version message
    fn send_version(addr: &str, height: usize) -> Result<()> {
        let socket_addr = addr
            .parse::<SocketAddr>()
            .map_err(|e| BlockchainError::Network(format!("Invalid address {addr}: {e}")))?;

        let node_addr = GLOBAL_CONFIG.get_node_addr();

        let pkg = Package::Version {
            addr_from: node_addr,
            version: NODE_VERSION,
            best_height: height,
        };

        Self::send_data(socket_addr, pkg)
    }

    /// Send get blocks message
    fn send_get_blocks(addr: &str) -> Result<()> {
        let socket_addr = addr
            .parse::<SocketAddr>()
            .map_err(|e| BlockchainError::Network(format!("Invalid address {addr}: {e}")))?;

        let node_addr = GLOBAL_CONFIG.get_node_addr();

        let pkg = Package::GetBlocks {
            addr_from: node_addr,
        };

        Self::send_data(socket_addr, pkg)
    }

    /// Send get data message
    fn send_get_data(addr: &str, op_type: OpType, id: &[u8]) -> Result<()> {
        let socket_addr = addr
            .parse::<SocketAddr>()
            .map_err(|e| BlockchainError::Network(format!("Invalid address {addr}: {e}")))?;

        let node_addr = GLOBAL_CONFIG.get_node_addr();

        let pkg = Package::GetData {
            addr_from: node_addr,
            op_type,
            id: id.to_vec(),
        };

        Self::send_data(socket_addr, pkg)
    }

    /// Send inventory message
    fn send_inv(addr: &str, op_type: OpType, items: &[Vec<u8>]) -> Result<()> {
        let socket_addr = addr
            .parse::<SocketAddr>()
            .map_err(|e| BlockchainError::Network(format!("Invalid address {addr}: {e}")))?;

        let node_addr = GLOBAL_CONFIG.get_node_addr();

        let pkg = Package::Inv {
            addr_from: node_addr,
            op_type,
            items: items.to_vec(),
        };

        Self::send_data(socket_addr, pkg)
    }

    /// Send block message
    fn send_block(addr: &str, block: &Block) -> Result<()> {
        let socket_addr = addr
            .parse::<SocketAddr>()
            .map_err(|e| BlockchainError::Network(format!("Invalid address {addr}: {e}")))?;

        let node_addr = GLOBAL_CONFIG.get_node_addr();
        let block_data = block
            .serialize()
            .map_err(|e| BlockchainError::Network(format!("Failed to serialize block: {e}")))?;

        let pkg = Package::Block {
            addr_from: node_addr,
            block: block_data,
        };

        Self::send_data(socket_addr, pkg)
    }

    /// Send transaction message
    fn send_tx(addr: &str, tx: &Transaction) -> Result<()> {
        let socket_addr = addr
            .parse::<SocketAddr>()
            .map_err(|e| BlockchainError::Network(format!("Invalid address {addr}: {e}")))?;

        let node_addr = GLOBAL_CONFIG.get_node_addr();
        let tx_data = tx.serialize().map_err(|e| {
            BlockchainError::Network(format!("Failed to serialize transaction: {e}"))
        })?;

        let pkg = Package::Tx {
            addr_from: node_addr,
            transaction: tx_data,
        };

        Self::send_data(socket_addr, pkg)
    }

    /// Send data to a peer
    fn send_data(addr: SocketAddr, pkg: Package) -> Result<()> {
        info!("Sending package to {addr}: {pkg:?}");

        let stream = TcpStream::connect_timeout(&addr, Duration::from_millis(TCP_WRITE_TIMEOUT))
            .map_err(|e| BlockchainError::Network(format!("Failed to connect to {addr}: {e}")))?;

        stream
            .set_write_timeout(Some(Duration::from_millis(TCP_WRITE_TIMEOUT)))
            .map_err(|e| BlockchainError::Network(format!("Failed to set write timeout: {e}")))?;

        serde_json::to_writer(&stream, &pkg)
            .map_err(|e| BlockchainError::Network(format!("Failed to send data: {e}")))?;

        Ok(())
    }
}

/// Standalone function to send a transaction to a specific address
pub fn send_tx(addr: &str, tx: &Transaction) {
    let socket_addr = match addr.parse::<SocketAddr>() {
        Ok(addr) => addr,
        Err(e) => {
            error!("Failed to parse address {addr}: {e}");
            return;
        }
    };

    let node_addr = GLOBAL_CONFIG.get_node_addr();
    let tx_data = match tx.serialize() {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to serialize transaction: {e}");
            return;
        }
    };

    let pkg = Package::Tx {
        addr_from: node_addr,
        transaction: tx_data,
    };

    if let Err(e) = send_data_simple(socket_addr, pkg) {
        error!("Failed to send transaction: {e}");
    }
}

/// Simple data sending function for standalone usage
fn send_data_simple(addr: SocketAddr, pkg: Package) -> Result<()> {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(TCP_WRITE_TIMEOUT))
        .map_err(|e| BlockchainError::Network(format!("Failed to connect to {addr}: {e}")))?;

    stream
        .set_write_timeout(Some(Duration::from_millis(TCP_WRITE_TIMEOUT)))
        .map_err(|e| BlockchainError::Network(format!("Failed to set write timeout: {e}")))?;

    serde_json::to_writer(&stream, &pkg)
        .map_err(|e| BlockchainError::Network(format!("Failed to send data: {e}")))?;

    let _ = stream.flush();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_blockchain() -> Result<Blockchain> {
        let temp_dir = tempdir().map_err(|e| BlockchainError::Io(e.to_string()))?;
        let db_path = temp_dir.path().join("test_blockchain");

        let test_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
        Blockchain::create_blockchain_with_path(
            test_address,
            db_path
                .to_str()
                .ok_or_else(|| BlockchainError::InvalidBlock("Invalid path".to_string()))?,
        )
    }

    #[test]
    fn test_server_creation() -> Result<()> {
        let blockchain = create_test_blockchain()?;
        let _server = Server::new(blockchain);
        Ok(())
    }

    #[test]
    fn test_package_serialization() {
        let pkg = Package::Version {
            addr_from: "127.0.0.1:2001".to_string(),
            version: 1,
            best_height: 0,
        };

        let serialized = serde_json::to_string(&pkg).unwrap();
        let _deserialized: Package = serde_json::from_str(&serialized).unwrap();
    }
}
