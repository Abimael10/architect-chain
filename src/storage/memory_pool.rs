use crate::core::Transaction;
use data_encoding::HEXLOWER;
use std::collections::HashMap;
use std::sync::RwLock;

/// ( K -> txid_hex, V => Transaction )
pub struct MemoryPool {
    inner: RwLock<HashMap<String, Transaction>>,
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryPool {
    pub fn new() -> MemoryPool {
        MemoryPool {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, txid: &str) -> Option<Transaction> {
        match self.inner.read() {
            Ok(pool) => pool.get(txid).cloned(),
            Err(_) => {
                log::error!("Failed to acquire read lock on memory pool");
                None
            }
        }
    }

    pub fn add(&self, tx: Transaction) {
        match self.inner.write() {
            Ok(mut pool) => {
                pool.insert(HEXLOWER.encode(tx.get_id()), tx);
            }
            Err(_) => {
                log::error!("Failed to acquire write lock on memory pool");
            }
        }
    }

    pub fn contains(&self, txid: &str) -> bool {
        match self.inner.read() {
            Ok(pool) => pool.contains_key(txid),
            Err(_) => {
                log::error!("Failed to acquire read lock on memory pool");
                false
            }
        }
    }

    pub fn remove(&self, txid: &str) {
        match self.inner.write() {
            Ok(mut pool) => {
                pool.remove(txid);
            }
            Err(_) => {
                log::error!("Failed to acquire write lock on memory pool");
            }
        }
    }

    pub fn len(&self) -> usize {
        match self.inner.read() {
            Ok(pool) => pool.len(),
            Err(_) => {
                log::error!("Failed to acquire read lock on memory pool");
                0
            }
        }
    }

    pub fn get_all(&self) -> Vec<Transaction> {
        match self.inner.read() {
            Ok(pool) => pool.values().cloned().collect(),
            Err(_) => {
                log::error!("Failed to acquire read lock on memory pool");
                Vec::new()
            }
        }
    }

    pub fn clear(&self) {
        match self.inner.write() {
            Ok(mut pool) => {
                pool.clear();
            }
            Err(_) => {
                log::error!("Failed to acquire write lock on memory pool");
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match self.inner.read() {
            Ok(pool) => pool.is_empty(),
            Err(_) => {
                log::error!("Failed to acquire read lock on memory pool");
                true // Conservative default
            }
        }
    }
}

pub struct BlockInTransit {
    inner: RwLock<Vec<Vec<u8>>>,
}

impl Default for BlockInTransit {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockInTransit {
    pub fn new() -> BlockInTransit {
        BlockInTransit {
            inner: RwLock::new(vec![]),
        }
    }

    pub fn add_blocks(&self, blocks: &[Vec<u8>]) {
        match self.inner.write() {
            Ok(mut inner) => {
                for hash in blocks {
                    inner.push(hash.to_vec());
                }
            }
            Err(_) => {
                log::error!("Failed to acquire write lock on block transit");
            }
        }
    }

    pub fn first(&self) -> Option<Vec<u8>> {
        match self.inner.read() {
            Ok(inner) => inner.first().map(|h| h.to_vec()),
            Err(_) => {
                log::error!("Failed to acquire read lock on block transit");
                None
            }
        }
    }

    pub fn remove(&self, block_hash: &[u8]) {
        match self.inner.write() {
            Ok(mut inner) => {
                if let Some(idx) = inner.iter().position(|x| x.eq(block_hash)) {
                    inner.remove(idx);
                }
            }
            Err(_) => {
                log::error!("Failed to acquire write lock on block transit");
            }
        }
    }

    pub fn clear(&self) {
        match self.inner.write() {
            Ok(mut inner) => {
                inner.clear();
            }
            Err(_) => {
                log::error!("Failed to acquire write lock on block transit");
            }
        }
    }

    pub fn len(&self) -> usize {
        match self.inner.read() {
            Ok(inner) => inner.len(),
            Err(_) => {
                log::error!("Failed to acquire read lock on block transit");
                0
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match self.inner.read() {
            Ok(inner) => inner.is_empty(),
            Err(_) => {
                log::error!("Failed to acquire read lock on block transit");
                true // Conservative default
            }
        }
    }
}
