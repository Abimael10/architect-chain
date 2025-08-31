```
    ╔═══════════════════════════════════════════════════════════════╗
    ║  █████╗ ██████╗  ██████╗██╗  ██╗██╗████████╗███████╗ ██████╗████████╗ ║
    ║ ██╔══██╗██╔══██╗██╔════╝██║  ██║██║╚══██╔══╝██╔════╝██╔════╝╚══██╔══╝ ║
    ║ ███████║██████╔╝██║     ███████║██║   ██║   █████╗  ██║        ██║    ║
    ║ ██╔══██║██╔══██╗██║     ██╔══██║██║   ██║   ██╔══╝  ██║        ██║    ║
    ║ ██║  ██║██║  ██║╚██████╗██║  ██║██║   ██║   ███████╗╚██████╗   ██║    ║
    ║ ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚═╝   ╚═╝   ╚══════╝ ╚═════╝   ╚═╝    ║
    ║                                                                       ║
    ║                          [ CUSTOM BLOCKCHAIN ]                        ║
    ║                                                                       ║
    ╚═══════════════════════════════════════════════════════════════════════╝
```

[![Rust](https://img.shields.io/badge/rust-1.70+-00ff00.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org)
[![Tests](https://img.shields.io/badge/tests-95%20PASSING-00ff00.svg?style=for-the-badge)](#testing)
[![Security](https://img.shields.io/badge/SECURITY-HARDENED-00ff00.svg?style=for-the-badge)](#security)

> *A blockchain implementation in Rust with UTXO model, proof-of-work consensus, and P2P networking*

## QUICK START

For a good start I compiled the essential commands in the Makefile I attached to the root of this repo so that the command handling is a little bit handy! (They are a ton...)

```bash
# Build the project
cargo build --release

# Create wallet and blockchain
./target/release/architect-chain createwallet
./target/release/architect-chain createblockchain <address>

# Check balance and send transactions
./target/release/architect-chain getbalance <address>
./target/release/architect-chain send <from> <to> <amount> <mine>
```

## CORE FEATURES

### **Blockchain Core**
- **SHA-256 Proof-of-Work** with dynamic difficulty adjustment (1-12 range)
- **UTXO Transaction Model** with Bitcoin-compatible structure
- **Merkle Trees** for transaction verification and integrity
- **Fork Resolution** using longest chain rule
- **Block Validation** with comprehensive PoW and transaction checks

### **Wallet System**
- **ECDSA P-256** key generation and transaction signing
- **Base58 Addresses** with Bitcoin-compatible format and checksums
- **Multi-Wallet Support** with optional AES-256-GCM encryption
- **UTXO Balance Tracking** with real-time calculation

### **Network Layer**
- **TCP P2P Communication** with message serialization
- **DNS Seeding** for peer discovery and bootstrap connections
- **Multi-Node Support** with isolated databases per node
- **Block Synchronization** with automatic validation

### **Fee System**
- **Dynamic Fees** with priority-based calculation (Low, Normal, High, Urgent)
- **Fixed Fee Mode** for legacy compatibility
- **Fee Estimation** based on transaction size and priority
- **Configurable Parameters** via TOML configuration

## SYSTEM ARCHITECTURE

```
┌─────────────────────────────────────────────────────────────────┐
│                        ARCHITECT CHAIN                         │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────┐ │
│  │ BLOCKCHAIN  │  │  NETWORK    │  │  STORAGE    │  │   CLI   │ │
│  │    CORE     │◄─┤   LAYER     │◄─┤   LAYER     │◄─┤         │ │
│  │             │  │             │  │             │  │         │ │
│  │ • Mining    │  │ • P2P TCP   │  │ • UTXO DB   │  │ • Clap  │ │
│  │ • PoW       │  │ • Sync      │  │ • Wallets   │  │ • Help  │ │
│  │ • Fees      │  │ • Discovery │  │ • Crypto    │  │ • Args  │ │
│  │ • Validate  │  │ • Peers     │  │ • Persist   │  │ • Logs  │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## COMMAND REFERENCE

### **Wallet Operations**
```bash
./target/release/architect-chain createwallet
./target/release/architect-chain listaddresses
./target/release/architect-chain getbalance <address>
```

### **Blockchain Operations**
```bash
./target/release/architect-chain createblockchain <address>
./target/release/architect-chain send <from> <to> <amount> <mine> [--priority <level>]
./target/release/architect-chain printchain
./target/release/architect-chain reindexutxo
```

### **Network Operations**
```bash
./target/release/architect-chain startnode [<miner_address>]
```

### **Fee Management**
```bash
./target/release/architect-chain feestatus
./target/release/architect-chain estimatefee <priority>
./target/release/architect-chain setfeemode <dynamic|fixed_amount>
```

## IMPLEMENTATION STATUS

| Component | Status | Tests |
|-----------|--------|-------|
| **Blockchain Core** | ✅ Implemented | 47 tests |
| **Transaction System** | ✅ Implemented | 24 tests |
| **Wallet Management** | ✅ Implemented | 15 tests |
| **P2P Network** | ✅ Implemented | 10 tests |
| **Storage Layer** | ✅ Considered | 17 tests |
| **CLI Interface** | ✅ Implemented | 11 commands |

## TESTING


```bash
cargo test                    # Run all tests
cargo clippy --all-targets    # Code quality check
```

## CONFIGURATION

**Basic Configuration** (`config/features.toml`):
```toml
[fee_system]
dynamic_enabled = true

[wallet]
encryption_enabled = false
backup_enabled = true

[network]
peer_discovery_mode = "Bootstrap"
```

## TECHNICAL SPECIFICATIONS

### **Monetary System**
- **Base Unit**: Satoshi (1 coin = 100,000,000 satoshis)
- **Block Reward**: 50 coins (5,000,000,000 satoshis)
- **Fee Range**: 1-10 satoshis (configurable)

### **Mining Parameters**
- **Algorithm**: SHA-256 Proof-of-Work
- **Difficulty**: 4 (adjusts every 10 blocks, range 1-12)
- **Block Time**: ~1-2 seconds (development setting)

### **Network Configuration**
- **Protocol**: TCP on port 2001 (default)
- **Message Format**: Binary serialization
- **Peer Discovery**: DNS seeding

## MULTI-NODE DEPLOYMENT

```bash
# Deploy 3-node network
./deployment/multi-node-blockchain-deployment.sh

# Monitor network status
./deployment/monitor-network.sh

# Stop network
./deployment/stop-network.sh
```

**Multi-Node Features**:
- Node isolation with separate databases, all the nodes keep the data consistency
- P2P block and transaction propagation
- Consensus across multiple miners
- Fork resolution and synchronization

## SECURITY

### **Cryptographic Security**
- **ECDSA P-256**: Transaction signatures
- **SHA-256**: Block hashing and proof-of-work
- **RIPEMD-160**: Address generation
- **AES-256-GCM**: Optional wallet encryption

### **Code Security**
- **Memory Safety**: Pure Rust implementation
- **Input Validation**: Comprehensive sanitization
- **Error Handling**: Result<T> usage
- **No Unsafe Code**: Zero unsafe blocks

## VERIFIED METRICS

```
┌─ PRODUCTION METRICS ────────────────────────────────────────────┐
│  Total Tests:          95/95 passing ✅                         │
│  Code Quality:         Clean clippy warnings ✅                 │
│  CLI Commands:         11 functional commands ✅                │
│  Multi-Node:           3-node deployment verified ✅            │
│  Dependencies:         24 widely knows dependencies ✅          │
└─────────────────────────────────────────────────────────────────┘
```

## USE CASES

### **Ready For**
- **Learning**: Learning blockchain concepts
- **Development**: Foundation for blockchain projects
- **Research**: Academic and experimental purposes
- **Prototyping**: Rapid blockchain development

### ** Production Considerations**
- **Database**: Sled embedded DB (consider distributed storage for scale if you plan to make it to production, which is some really big work!)
- **Networking**: Basic P2P (consider advanced features for production, as of now I do not have a use case where I want to rely on strongest networking implementations, time will tell)
- **Performance**: Optimized for putting concepts to work, not high-throughput
- **Monitoring**: Basic logging (comprehensive monitoring recommended)

## CONTRIBUTING

```bash
git clone https://github.com/Abimael10/architect-chain.git
cd architect-chain
cargo build && cargo test && cargo clippy --all-targets
```

## LEARNING PATH

```
Level 1: USER      → Create wallets, send transactions
Level 2: OPERATOR  → Deploy multi-node networks  
Level 3: DEVELOPER → Study code, run tests, modify
Level 4: ARCHITECT → Contribute, extend, innovate
```

## SUMMARY

**Architect Chain** is a **complete, functional blockchain implementation** that demonstrates all essential blockchain concepts with production-quality Rust code, comprehensive testing, and multi-node deployment capability.

**Built with ❤️ and ☕ by Juan Abimael Santos Castillo**
