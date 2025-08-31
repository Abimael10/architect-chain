use std::net::SocketAddr;
use std::sync::RwLock;

#[derive(Clone)]
pub struct Node {
    addr: String,
}

impl Node {
    fn new(addr: String) -> Node {
        Node { addr }
    }

    pub fn get_addr(&self) -> String {
        self.addr.clone()
    }

    pub fn parse_socket_addr(&self) -> SocketAddr {
        self.addr
            .parse()
            .expect("Failed to parse node address - address should be valid")
    }
}

pub struct Nodes {
    inner: RwLock<Vec<Node>>,
}

impl Default for Nodes {
    fn default() -> Self {
        Self::new()
    }
}

impl Nodes {
    pub fn new() -> Nodes {
        Nodes {
            inner: RwLock::new(vec![]),
        }
    }

    pub fn add_node(&self, addr: String) {
        let mut inner = self
            .inner
            .write()
            .expect("Failed to acquire write lock on nodes - this should never happen");
        if !inner.iter().any(|x| x.get_addr().eq(addr.as_str())) {
            inner.push(Node::new(addr));
        }
    }

    pub fn evict_node(&self, addr: &str) {
        let mut inner = self
            .inner
            .write()
            .expect("Failed to acquire write lock on nodes - this should never happen");
        if let Some(idx) = inner.iter().position(|x| x.get_addr().eq(addr)) {
            inner.remove(idx);
        }
    }

    pub fn first(&self) -> Option<Node> {
        let inner = self
            .inner
            .read()
            .expect("Failed to acquire read lock on nodes - this should never happen");
        if let Some(node) = inner.first() {
            return Some(node.clone());
        }
        None
    }

    pub fn get_nodes(&self) -> Vec<Node> {
        self.inner
            .read()
            .expect("Failed to acquire read lock on nodes - this should never happen")
            .to_vec()
    }

    pub fn len(&self) -> usize {
        self.inner
            .read()
            .expect("Failed to acquire read lock on nodes - this should never happen")
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner
            .read()
            .expect("Failed to acquire read lock on nodes - this should never happen")
            .is_empty()
    }

    pub fn node_is_known(&self, addr: &str) -> bool {
        let inner = self
            .inner
            .read()
            .expect("Failed to acquire read lock on nodes - this should never happen");
        if inner.iter().any(|x| x.get_addr().eq(addr)) {
            return true;
        }
        false
    }
}
