# Core Blockchain Components

This module implements the fundamental blockchain components in Rust.

## Components

### Monetary System (`monetary.rs`)
Bitcoin-compatible satoshi-based system (1 coin = 100,000,000 satoshis) with safe conversion utilities and validation functions.

### Transaction System (`transaction.rs`)
UTXO-based transactions with ECDSA P-256 signatures, fee handling, and multiple creation methods.

```rust
// Create UTXO transaction
let tx = Transaction::new_utxo_transaction(from, to, amount, &utxo_set)?;

// Create with priority
let tx = Transaction::new_utxo_transaction_with_priority(
    from, to, amount, FeePriority::High, &utxo_set
)?;
```

### Block System (`block.rs`)
Blockchain blocks with Merkle tree integration, proper hash calculation, and serialization support.

### Blockchain Management (`blockchain.rs`)
Core blockchain with node-specific database isolation, block mining, UTXO management, and synchronization.

### Merkle Tree (`merkle.rs`)
Bitcoin-compatible Merkle trees with double SHA-256 hashing and proof generation/verification.

### Proof of Work (`proof_of_work.rs`)
Mining algorithm with configurable difficulty and proper nonce iteration.

### Difficulty Adjustment (`difficulty.rs`)
Maintains 2-minute block times with adjustment every 10 blocks (bounds: 1-12).

### Fee System (`fees/`)
Fixed and dynamic fee calculation with priority-based pricing.

## Usage

### Create Blockchain
```rust
let blockchain = Blockchain::create_blockchain(genesis_address)?;
let utxo_set = UTXOSet::new(blockchain.clone());
```

### Mine Block
```rust
let coinbase_tx = Transaction::new_coinbase_tx(miner_address)?;
let new_block = blockchain.mine_block(&[coinbase_tx])?;
```

### Multi-Node
```rust
let blockchain = Blockchain::create_blockchain_with_path(address, "data/node_2001")?;
```

## Security

- **ECDSA P-256**: Transaction signatures
- **SHA-256**: Block hashing and proof-of-work
- **Merkle Trees**: Transaction verification
- **UTXO Model**: Double-spend prevention

## Testing

```bash
cargo test --lib core
```

All components are fully implemented and tested.