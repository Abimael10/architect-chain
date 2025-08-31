use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::env;
use std::sync::RwLock;

pub static GLOBAL_CONFIG: Lazy<Config> = Lazy::new(Config::new);

static DEFAULT_NODE_ADDR: &str = "127.0.0.1:2001";

const NODE_ADDRESS_KEY: &str = "NODE_ADDRESS";
const MINING_ADDRESS_KEY: &str = "MINING_ADDRESS";
const NODE_ID_KEY: &str = "NODE_ID";

pub struct Config {
    inner: RwLock<HashMap<String, String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn new() -> Config {
        let mut node_addr = String::from(DEFAULT_NODE_ADDR);
        if let Ok(addr) = env::var("NODE_ADDRESS") {
            node_addr = addr;
        }

        let mut map = HashMap::new();
        map.insert(String::from(NODE_ADDRESS_KEY), node_addr);

        // Set default node ID if not provided
        if let Ok(node_id) = env::var("NODE_ID") {
            map.insert(String::from(NODE_ID_KEY), node_id);
        }

        Config {
            inner: RwLock::new(map),
        }
    }

    pub fn get_node_addr(&self) -> String {
        let inner = self
            .inner
            .read()
            .expect("Failed to acquire read lock on config - this should never happen");
        inner
            .get(NODE_ADDRESS_KEY)
            .expect("Node address should always be present in config")
            .clone()
    }

    pub fn set_node_addr(&self, addr: String) {
        let mut inner = self
            .inner
            .write()
            .expect("Failed to acquire write lock on config - this should never happen");
        inner.insert(String::from(NODE_ADDRESS_KEY), addr);
    }

    pub fn set_mining_addr(&self, addr: String) {
        let mut inner = self
            .inner
            .write()
            .expect("Failed to acquire write lock on config - this should never happen");
        let _ = inner.insert(String::from(MINING_ADDRESS_KEY), addr);
    }

    pub fn get_mining_addr(&self) -> Option<String> {
        let inner = self
            .inner
            .read()
            .expect("Failed to acquire read lock on config - this should never happen");
        if let Some(addr) = inner.get(MINING_ADDRESS_KEY) {
            return Some(addr.clone());
        }
        None
    }

    pub fn is_miner(&self) -> bool {
        let inner = self
            .inner
            .read()
            .expect("Failed to acquire read lock on config - this should never happen");
        inner.contains_key(MINING_ADDRESS_KEY)
    }

    pub fn set_node_id(&self, node_id: String) {
        let mut inner = self
            .inner
            .write()
            .expect("Failed to acquire write lock on config - this should never happen");
        inner.insert(String::from(NODE_ID_KEY), node_id);
    }

    pub fn get_node_id(&self) -> Option<String> {
        let inner = self
            .inner
            .read()
            .expect("Failed to acquire read lock on config - this should never happen");
        inner.get(NODE_ID_KEY).cloned()
    }

    /// Extract node ID from address (e.g., "127.0.0.1:2001" -> "2001")
    pub fn extract_node_id_from_addr(&self) -> String {
        let addr = self.get_node_addr();
        if let Some(port) = addr.split(':').next_back() {
            port.to_string()
        } else {
            "default".to_string()
        }
    }
}
