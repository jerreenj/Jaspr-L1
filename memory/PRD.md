# JasprChain - Layer 1 Blockchain

## Project Overview
JasprChain is a high-performance, mobile-first Layer 1 blockchain with Move VM smart contract support and PoS consensus.

## Current State: Rust Core Implementation COMPLETE (~60%)

### What's Been Built

#### 1. Python/JS Testnet Simulation (COMPLETE - DO NOT MODIFY)
- FastAPI backend + React frontend demo
- Real-time block production, 20 validators
- JASPR token tokenomics, staking/unstaking
- LMDB persistence, mobile-responsive UI
- Location: `/app/backend/`, `/app/frontend/`

#### 2. Rust Core Implementation (60% COMPLETE)
Location: `/app/rust-core/`

**Crates Built:**
| Crate | Status | Description |
|-------|--------|-------------|
| `jaspr-types` | ✅ Complete | Core types: Block, Transaction, Address, Hash, Account, Receipt |
| `jaspr-crypto` | ✅ Complete | Ed25519 signatures, SHA256/SHA3/Blake3 hashing |
| `jaspr-state` | ✅ Complete | RocksDB storage, AccountStore, BlockStore, StateTree |
| `jaspr-consensus` | ✅ Complete | PoS ValidatorSet, BlockProducer, Attestations, ConsensusEngine |
| `jaspr-executor` | ✅ Complete | Transaction execution, Gas metering, VM adapter trait |
| `jaspr-move-vm` | ✅ Complete | Move VM adapter, module cache, stdlib definitions |
| `jaspr-network` | ✅ Complete | P2P peer management, gossip protocol, network service |
| `jaspr-node` | ✅ Complete | Full node: config, mempool, RPC types, node orchestration |

**Binary: `jasprchain`**
```bash
jasprchain run      # Start node
jasprchain keygen   # Generate keypair
jasprchain init     # Initialize chain
jasprchain version  # Show version
```

### Build & Test
```bash
cd /app/rust-core
cargo build   # Compiles successfully
cargo test    # All 17 tests pass
./target/debug/jasprchain --help
```

## Architecture
```
/app/rust-core/
├── Cargo.toml              # Workspace config
└── crates/
    ├── jaspr-types/        # Core data structures
    ├── jaspr-crypto/       # Cryptographic primitives
    ├── jaspr-state/        # RocksDB storage layer
    ├── jaspr-consensus/    # PoS consensus engine
    ├── jaspr-executor/     # Transaction execution
    ├── jaspr-move-vm/      # Move VM integration
    ├── jaspr-network/      # P2P networking
    └── jaspr-node/         # Full node binary
```

## Key Features Implemented
- **Cryptography**: Ed25519 signing, multi-hash support (SHA256, SHA3, Blake3)
- **Storage**: RocksDB with column families (blocks, txs, accounts, receipts, state)
- **Consensus**: PoS with validator set, weighted proposer selection, attestations, finality
- **Execution**: Full tx execution pipeline with gas metering, VM adapter pattern
- **Move VM**: Module cache, stdlib definitions (Coin, Staking, JASPR, Account, Event)
- **Networking**: Peer management, gossip protocol, message types, network service
- **Node**: Full orchestration with mempool, config, genesis initialization

## Upcoming Tasks
1. **Faucet Implementation** - Web endpoint to distribute testnet JASPR
2. **Real P2P with libp2p** - Replace simulated networking
3. **Full Move VM Integration** - Connect move-vm-runtime crate
4. **RPC Server** - HTTP/WebSocket JSON-RPC endpoint
5. **Block Explorer API** - Query blocks, txs, accounts
6. **SDK/CLI Tools** - Transaction building, wallet management

## Technical Details
- **Language**: Rust 2021 Edition
- **Storage**: RocksDB
- **Consensus**: Proof-of-Stake (67% finality threshold)
- **Block Time**: 2 seconds
- **Token**: JASPR (1B fixed supply, 9 decimals)
- **Min Validator Stake**: 32,000 JASPR
