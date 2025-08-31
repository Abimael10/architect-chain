//! Simplified peer-to-peer networking functionality
//!
//! This module handles network communication between blockchain nodes,
//! including message passing, block synchronization, and transaction propagation.
//!
//! Simplified to focus on blockchain essentials without unnecessary complexity.

pub mod dns_seeding;
pub mod node;
pub mod server;
pub mod simple_peer_manager;

pub use crate::storage::BlockInTransit;
pub use dns_seeding::{DiscoveredPeer, DnsSeeder};
pub use node::{Node, Nodes};
pub use server::{send_tx, Server, CENTRAL_NODE};
pub use simple_peer_manager::SimplePeerManager;
