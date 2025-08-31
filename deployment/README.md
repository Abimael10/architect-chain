# Multi-Node Blockchain Deployment

Complete multi-node blockchain network with node isolation and P2P networking.

## Quick Start

```bash
# Build
cargo build --release

# Deploy 3-node network
./deployment/multi-node-blockchain-deployment.sh

# Monitor
./deployment/monitor-network.sh

# Stop
./deployment/stop-network.sh
```

## Network Architecture

```
Bootstrap Node (2001) ←→ Mining Node 1 (2002)
       ↕                        ↕
Mining Node 2 (2003)    ←→    [Additional Nodes...]
```

## Node Types

1. **Bootstrap Node (Port 2001)**: Network entry point
2. **Mining Node 1 (Port 2002)**: Mining with unique address
3. **Mining Node 2 (Port 2003)**: Additional mining node

## Configuration

```bash
export NODE_ADDRESS="127.0.0.1:2001"
export MINING_ADDRESS="wallet_address"
export RUST_LOG="info"
```

## Features

- **Multi-Node Network**: Isolated databases per node
- **P2P TCP Networking**: Direct node communication
- **Transaction Propagation**: Network-wide transaction relay
- **Block Mining**: Proof-of-work consensus
- **Blockchain Sync**: Automatic synchronization
- **Peer Discovery**: DNS seeding

## Node Isolation

Each node maintains separate storage:
- `data/node_2001/` - Bootstrap node
- `data/node_2002/` - Mining node 1
- `data/node_2003/` - Mining node 2

## Scaling

```bash
# Add more nodes
export NODE_ADDRESS="127.0.0.1:2004"
./target/release/architect-chain startnode [miner_address]
```

## Security

- **ECDSA P-256**: Transaction signatures
- **SHA-256 PoW**: Consensus mechanism
- **UTXO Model**: Double-spend prevention
- **Connection Limits**: Basic peer management

## Performance

- **Block Time**: ~1-2 seconds
- **Memory Usage**: ~50MB per node
- **Network**: Local TCP communication
- **Storage**: Sled embedded database

## Production Considerations

**Current Status**: Development and educational use
**For Production**: Consider distributed storage, advanced networking, comprehensive monitoring

This deployment demonstrates complete blockchain functionality including decentralization, consensus, and P2P networking.