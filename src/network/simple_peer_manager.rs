use crate::error::{BlockchainError, Result};
use crate::network::dns_seeding::DnsSeeder;
use log::info;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

/// Simple peer manager for blockchain networking
///
/// This provides basic peer management without unnecessary complexity:
/// - Simple peer discovery via DNS seeding
/// - Basic connection tracking
/// - No peer reputation, banning, or complex retry logic
pub struct SimplePeerManager {
    /// DNS seeder for discovering peers
    dns_seeder: DnsSeeder,
    /// Currently connected peers
    connected_peers: Arc<RwLock<HashSet<SocketAddr>>>,
    /// Maximum number of connections
    max_connections: usize,
}

impl SimplePeerManager {
    /// Create a new simple peer manager
    pub fn new(max_connections: usize, default_port: u16) -> Self {
        Self {
            dns_seeder: DnsSeeder::new(default_port),
            connected_peers: Arc::new(RwLock::new(HashSet::new())),
            max_connections,
        }
    }

    /// Create a peer manager for development
    pub fn for_development() -> Self {
        Self {
            dns_seeder: DnsSeeder::development(),
            connected_peers: Arc::new(RwLock::new(HashSet::new())),
            max_connections: 8,
        }
    }

    /// Get peers to connect to
    pub fn get_peers_to_connect(&self) -> Result<Vec<SocketAddr>> {
        let connected_count = self.get_connected_count()?;

        if connected_count >= self.max_connections {
            return Ok(vec![]); // Already have enough connections
        }

        let needed = self.max_connections - connected_count;

        // Discover peers via DNS seeding
        let discovered_peers = self.dns_seeder.discover_peers()?;

        // Filter out already connected peers
        let connected_addrs = self.get_connected_addresses()?;

        let available_peers: Vec<SocketAddr> = discovered_peers
            .into_iter()
            .map(|peer| peer.address)
            .filter(|addr| !connected_addrs.contains(addr))
            .take(needed)
            .collect();

        info!("Found {} peers to connect to", available_peers.len());
        Ok(available_peers)
    }

    /// Record a successful connection
    pub fn record_connection(&self, address: SocketAddr) -> Result<()> {
        let mut connected = self
            .connected_peers
            .write()
            .map_err(|e| BlockchainError::Network(format!("Failed to acquire peer lock: {e}")))?;

        connected.insert(address);
        info!("Connected to peer: {address}");
        Ok(())
    }

    /// Record a disconnection
    pub fn record_disconnection(&self, address: SocketAddr) -> Result<()> {
        let mut connected = self
            .connected_peers
            .write()
            .map_err(|e| BlockchainError::Network(format!("Failed to acquire peer lock: {e}")))?;

        connected.remove(&address);
        info!("Disconnected from peer: {address}");
        Ok(())
    }

    /// Get connected peer addresses
    pub fn get_connected_addresses(&self) -> Result<HashSet<SocketAddr>> {
        let connected = self
            .connected_peers
            .read()
            .map_err(|e| BlockchainError::Network(format!("Failed to acquire peer lock: {e}")))?;
        Ok(connected.clone())
    }

    /// Get number of connected peers
    pub fn get_connected_count(&self) -> Result<usize> {
        let connected = self
            .connected_peers
            .read()
            .map_err(|e| BlockchainError::Network(format!("Failed to acquire peer lock: {e}")))?;
        Ok(connected.len())
    }

    /// Check if we should accept more connections
    pub fn should_accept_connection(&self) -> Result<bool> {
        let connected_count = self.get_connected_count()?;
        Ok(connected_count < self.max_connections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_peer_manager_creation() {
        let manager = SimplePeerManager::new(8, 2001);
        assert_eq!(manager.get_connected_count().unwrap(), 0);
    }

    #[test]
    fn test_connection_tracking() {
        let manager = SimplePeerManager::new(8, 2001);
        let addr = "127.0.0.1:2001".parse().unwrap();

        // Record connection
        manager.record_connection(addr).unwrap();
        assert_eq!(manager.get_connected_count().unwrap(), 1);

        // Record disconnection
        manager.record_disconnection(addr).unwrap();
        assert_eq!(manager.get_connected_count().unwrap(), 0);
    }

    #[test]
    fn test_connection_limits() {
        let manager = SimplePeerManager::new(2, 2001);
        let addr1 = "127.0.0.1:2001".parse().unwrap();
        let addr2 = "127.0.0.1:2002".parse().unwrap();

        // Should accept connections up to limit
        assert!(manager.should_accept_connection().unwrap());
        manager.record_connection(addr1).unwrap();

        assert!(manager.should_accept_connection().unwrap());
        manager.record_connection(addr2).unwrap();

        // Should not accept more connections
        assert!(!manager.should_accept_connection().unwrap());
    }
}
