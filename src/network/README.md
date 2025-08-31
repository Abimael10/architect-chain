# Network Layer

P2P networking layer for blockchain distributed operation and consensus.

## Components

### Server (`server.rs`)
TCP-based P2P server handling peer connections, message routing, and blockchain synchronization.

### Simple Peer Manager (`simple_peer_manager.rs`)
Basic peer connection management with connection limits and tracking.

### DNS Seeding (`dns_seeding.rs`)
Peer discovery through DNS seeding for network bootstrap.

### Node Management (`node.rs`)
Node identification, addressing, and state tracking.

## Configuration

```rust
pub const CENTRAL_NODE: &str = "127.0.0.1:2001";
pub const TRANSACTION_THRESHOLD: usize = 10;
const TCP_WRITE_TIMEOUT: u64 = 5000;
```

## Message Types

```rust
pub enum Package {
    Block { addr_from: String, block: Vec<u8> },
    GetBlocks { addr_from: String },
    GetData { addr_from: String, op_type: OpType, id: Vec<u8> },
    Inv { addr_from: String, op_type: OpType, items: Vec<Vec<u8>> },
    Tx { addr_from: String, transaction: Vec<u8> },
    Version { addr_from: String, version: usize, best_height: usize },
}
```

## Usage

### Start Server
```rust
let server = Server::new(blockchain);
server.run("127.0.0.1:2001")?;
```

### Send Transaction
```rust
send_tx(CENTRAL_NODE, &transaction);
```

## Network Operations

- **TCP P2P Communication**: Direct peer messaging
- **Peer Discovery**: DNS-based peer finding
- **Block Propagation**: Network-wide block distribution
- **Transaction Relay**: Transaction propagation
- **Blockchain Sync**: Automatic synchronization

## Security

- **Connection Management**: Basic peer limits
- **Message Validation**: Format validation
- **Timeout Protection**: Connection timeouts
- **Error Handling**: Graceful error recovery

## Testing

```bash
cargo test --lib network
```

The network layer provides complete P2P functionality for blockchain operation.