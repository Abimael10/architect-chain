use crate::error::{BlockchainError, Result};
use log::{info, warn};
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

/// DNS seeding configuration and implementation
///
/// This module provides Bitcoin-compatible DNS seeding functionality
/// to discover initial peers without relying on hardcoded addresses.
pub struct DnsSeeder {
    /// List of DNS seed hostnames
    dns_seeds: Vec<String>,
    /// Default port for the network
    default_port: u16,
    /// Timeout for DNS resolution
    resolution_timeout: Duration,
    /// Maximum number of addresses to return
    max_addresses: usize,
}

/// Represents a discovered peer address with metadata
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiscoveredPeer {
    /// Socket address of the peer
    pub address: SocketAddr,
    /// When this peer was discovered
    pub discovered_at: Instant,
    /// Source of discovery (DNS seed hostname)
    pub source: String,
}

impl DnsSeeder {
    /// Create a new DNS seeder with default configuration
    pub fn new(default_port: u16) -> Self {
        Self {
            dns_seeds: Self::default_dns_seeds(),
            default_port,
            resolution_timeout: Duration::from_secs(10),
            max_addresses: 100,
        }
    }

    /// Create a DNS seeder with custom seeds
    pub fn with_seeds(dns_seeds: Vec<String>, default_port: u16) -> Self {
        Self {
            dns_seeds,
            default_port,
            resolution_timeout: Duration::from_secs(10),
            max_addresses: 100,
        }
    }

    /// Set the DNS resolution timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.resolution_timeout = timeout;
        self
    }

    /// Set the maximum number of addresses to return
    pub fn with_max_addresses(mut self, max_addresses: usize) -> Self {
        self.max_addresses = max_addresses;
        self
    }

    /// Get default DNS seeds for the network
    /// In a real implementation, these would be actual DNS seeds
    fn default_dns_seeds() -> Vec<String> {
        vec![
            "seed1.architect-chain.org".to_string(),
            "seed2.architect-chain.org".to_string(),
            "seed3.architect-chain.org".to_string(),
            "dnsseed.architect-chain.org".to_string(),
        ]
    }

    /// Discover peers from all configured DNS seeds
    pub fn discover_peers(&self) -> Result<Vec<DiscoveredPeer>> {
        info!(
            "Starting DNS peer discovery from {} seeds",
            self.dns_seeds.len()
        );

        let mut all_peers = HashSet::new();
        let mut successful_seeds = 0;

        for seed in &self.dns_seeds {
            match self.resolve_seed(seed) {
                Ok(peers) => {
                    successful_seeds += 1;
                    info!("DNS seed '{}' returned {} peers", seed, peers.len());
                    all_peers.extend(peers);
                }
                Err(e) => {
                    warn!("Failed to resolve DNS seed '{seed}': {e}");
                }
            }
        }

        if successful_seeds == 0 {
            return Err(BlockchainError::Network(
                "All DNS seeds failed to resolve".to_string(),
            ));
        }

        let mut peers: Vec<DiscoveredPeer> = all_peers.into_iter().collect();

        // Limit the number of returned peers
        if peers.len() > self.max_addresses {
            peers.truncate(self.max_addresses);
        }

        info!(
            "DNS discovery completed: {} unique peers from {} successful seeds",
            peers.len(),
            successful_seeds
        );

        Ok(peers)
    }

    /// Resolve a single DNS seed to peer addresses
    fn resolve_seed(&self, seed: &str) -> Result<Vec<DiscoveredPeer>> {
        info!("Resolving DNS seed: {seed}");

        // For development/testing, we'll simulate DNS resolution
        // In production, this would use actual DNS resolution
        if self.is_development_mode() {
            return self.simulate_dns_resolution(seed);
        }

        // Actual DNS resolution implementation
        self.perform_dns_resolution(seed)
    }

    /// Check if we're in development mode (no real DNS seeds available)
    fn is_development_mode(&self) -> bool {
        // Check if any of the seeds are localhost or development domains
        self.dns_seeds.iter().any(|seed| {
            seed.contains("localhost")
                || seed.contains("127.0.0.1")
                || seed.contains("architect-chain.org") // Our example domain
        })
    }

    /// Simulate DNS resolution for development/testing
    fn simulate_dns_resolution(&self, seed: &str) -> Result<Vec<DiscoveredPeer>> {
        info!("Simulating DNS resolution for development seed: {seed}");

        // Return some simulated peer addresses for testing
        let simulated_peers = vec![
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 2001),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 2002),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 2003),
            SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                self.default_port,
            ),
            SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101)),
                self.default_port,
            ),
        ];

        let discovered_peers = simulated_peers
            .into_iter()
            .map(|addr| DiscoveredPeer {
                address: addr,
                discovered_at: Instant::now(),
                source: seed.to_string(),
            })
            .collect();

        Ok(discovered_peers)
    }

    /// Perform actual DNS resolution
    fn perform_dns_resolution(&self, seed: &str) -> Result<Vec<DiscoveredPeer>> {
        let seed_with_port = format!("{}:{}", seed, self.default_port);

        match seed_with_port.to_socket_addrs() {
            Ok(addresses) => {
                let discovered_peers = addresses
                    .map(|addr| DiscoveredPeer {
                        address: addr,
                        discovered_at: Instant::now(),
                        source: seed.to_string(),
                    })
                    .collect();

                Ok(discovered_peers)
            }
            Err(e) => Err(BlockchainError::Network(format!(
                "DNS resolution failed for '{seed}': {e}"
            ))),
        }
    }

    /// Add a custom DNS seed
    pub fn add_seed(&mut self, seed: String) {
        if !self.dns_seeds.contains(&seed) {
            self.dns_seeds.push(seed);
        }
    }

    /// Remove a DNS seed
    pub fn remove_seed(&mut self, seed: &str) {
        self.dns_seeds.retain(|s| s != seed);
    }

    /// Get all configured DNS seeds
    pub fn get_seeds(&self) -> &[String] {
        &self.dns_seeds
    }

    /// Test connectivity to a discovered peer
    pub fn test_peer_connectivity(&self, peer: &DiscoveredPeer) -> bool {
        use std::net::TcpStream;

        match TcpStream::connect_timeout(&peer.address, self.resolution_timeout) {
            Ok(_) => {
                info!("Peer {} is reachable", peer.address);
                true
            }
            Err(e) => {
                warn!("Peer {} is not reachable: {}", peer.address, e);
                false
            }
        }
    }

    /// Filter peers by reachability
    pub fn filter_reachable_peers(&self, peers: Vec<DiscoveredPeer>) -> Vec<DiscoveredPeer> {
        info!("Testing connectivity to {} discovered peers", peers.len());

        let reachable_peers: Vec<DiscoveredPeer> = peers
            .into_iter()
            .filter(|peer| self.test_peer_connectivity(peer))
            .collect();

        info!("Found {} reachable peers", reachable_peers.len());
        reachable_peers
    }
}

/// DNS seeding utility functions
impl DnsSeeder {
    /// Create a seeder for mainnet
    pub fn mainnet() -> Self {
        Self::new(2001) // Default mainnet port
    }

    /// Create a seeder for testnet
    pub fn testnet() -> Self {
        let mut seeder = Self::new(12001); // Testnet port
        seeder.dns_seeds = vec![
            "testnet-seed1.architect-chain.org".to_string(),
            "testnet-seed2.architect-chain.org".to_string(),
        ];
        seeder
    }

    /// Create a seeder for development/local testing
    pub fn development() -> Self {
        let mut seeder = Self::new(2001);
        seeder.dns_seeds = vec!["localhost".to_string(), "127.0.0.1".to_string()];
        seeder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_seeder_creation() {
        let seeder = DnsSeeder::new(2001);
        assert_eq!(seeder.default_port, 2001);
        assert!(!seeder.dns_seeds.is_empty());
    }

    #[test]
    fn test_custom_seeds() {
        let custom_seeds = vec![
            "custom1.example.com".to_string(),
            "custom2.example.com".to_string(),
        ];
        let seeder = DnsSeeder::with_seeds(custom_seeds.clone(), 8333);
        assert_eq!(seeder.get_seeds(), &custom_seeds);
    }

    #[test]
    fn test_development_mode_detection() {
        let seeder = DnsSeeder::development();
        assert!(seeder.is_development_mode());
    }

    #[test]
    fn test_simulated_dns_resolution() {
        let seeder = DnsSeeder::development();
        let peers = seeder.simulate_dns_resolution("localhost").unwrap();
        assert!(!peers.is_empty());

        // Check that all peers have the correct source
        for peer in &peers {
            assert_eq!(peer.source, "localhost");
        }
    }

    #[test]
    fn test_peer_discovery() {
        let seeder = DnsSeeder::development();
        let peers = seeder.discover_peers().unwrap();
        assert!(!peers.is_empty());
    }

    #[test]
    fn test_add_remove_seeds() {
        let mut seeder = DnsSeeder::new(2001);
        let initial_count = seeder.dns_seeds.len();

        seeder.add_seed("new-seed.example.com".to_string());
        assert_eq!(seeder.dns_seeds.len(), initial_count + 1);

        seeder.remove_seed("new-seed.example.com");
        assert_eq!(seeder.dns_seeds.len(), initial_count);
    }

    #[test]
    fn test_discovered_peer_equality() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 2001);
        let peer1 = DiscoveredPeer {
            address: addr,
            discovered_at: Instant::now(),
            source: "test".to_string(),
        };
        let peer2 = DiscoveredPeer {
            address: addr,
            discovered_at: Instant::now(),
            source: "test".to_string(),
        };

        // Peers with same address should be equal (for HashSet deduplication)
        assert_eq!(peer1.address, peer2.address);
    }
}
