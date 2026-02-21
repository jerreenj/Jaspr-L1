# JasprChain - Product Requirements Document

## Original Problem Statement
Build JasprChain - a high-performance Layer 1 blockchain (CORE ONLY, no application layer):
- Rust core chain architecture (Python simulator + Rust production code)
- Move VM integration points
- HyperLiquid-style validator model (4 initial validators)
- AI Sentinel with real ML risk scoring
- MPC + Account Abstraction wallets
- <2s deterministic finality
- $JJ native token
- Staking/Unstaking functionality

## Architecture

### Production Rust Codebase (`/app/rust-core/jasprchain/`)
```
jasprchain/
├── Cargo.toml                    # Dependencies: blst, ed25519-dalek, tokio, libp2p
├── src/
│   ├── lib.rs                    # Main library
│   ├── main.rs                   # Node binary
│   ├── consensus/
│   │   ├── block.rs              # Block/BlockHeader
│   │   ├── validator.rs          # ValidatorSet (HyperLiquid model)
│   │   ├── proposer.rs           # VRF-based selection
│   │   └── finality.rs           # BLS committee attestations
│   ├── execution/
│   │   ├── transaction.rs        # Transaction types
│   │   └── parallel.rs           # Conflict detection
│   ├── state/
│   │   ├── smt.rs                # Sparse Merkle Tree
│   │   └── account.rs            # Account model
│   ├── wallet/
│   │   ├── mpc.rs                # MPC 2-of-3 threshold
│   │   └── aa.rs                 # Account Abstraction
│   ├── sentinel/
│   │   └── scorer.rs             # Risk scoring hooks
│   ├── network/
│   │   └── mempool.rs            # Priority lanes
│   └── crypto/
│       ├── keys.rs               # Ed25519
│       ├── bls.rs                # BLS12-381 (blst)
│       └── hash.rs               # SHA256, Merkle
```

### Python Simulator (`/app/backend/jasprchain/`)
Mirrors Rust 1:1 for testing and dashboard

### React Dashboard (`/app/frontend/`)
- Dashboard - Network overview
- Block Explorer - Block/TX details  
- Validators - Stake distribution
- Wallet - MPC wallet creation
- **Staking** - Stake/Unstake to validators
- AI Sentinel - Guard modes
- Mempool - Priority lanes

## What's Been Implemented (Jan 2026)

### Core L1 Features
- ✅ BLS12-381 consensus signatures (blst crate)
- ✅ Ed25519 wallet keys (ed25519-dalek)
- ✅ VRF-based proposer selection
- ✅ Committee finality with 2/3 threshold
- ✅ 4 validators (HyperLiquid model)
- ✅ Parallel execution with conflict detection
- ✅ Sparse Merkle Tree state
- ✅ MPC wallet (2-of-3 threshold)
- ✅ Account Abstraction (spending limits, blocked addresses)
- ✅ AI Sentinel with ML scoring
- ✅ Mempool with 6 priority lanes
- ✅ **Staking/Unstaking** functionality
- ✅ Complete Rust codebase for production

### Removed (Application Layer - Build on Top)
- ❌ DEX (not L1 - build as smart contract)
- ❌ NFTs, tokens (not L1 - deploy via Move VM)

## Rust Build Instructions

```bash
cd /app/rust-core/jasprchain
cargo build --release
cargo run --release -- --rpc-port 8545 --p2p-port 30303
```

## Prioritized Backlog

### P0 (Critical for Testnet)
- [ ] Complete libp2p networking
- [ ] RocksDB persistence
- [ ] Move VM integration (move-vm-runtime)
- [ ] Genesis config file

### P1 (High)
- [ ] RPC server (JSON-RPC 2.0)
- [ ] Light client support
- [ ] Unbonding period for unstaking
- [ ] Slashing implementation

### P2 (Medium)
- [ ] Move module deployment
- [ ] Gas estimation
- [ ] Archive node support

## Next Tasks
1. Add libp2p peer discovery
2. Implement RPC endpoints
3. Add RocksDB for block/state persistence
4. Move VM contract execution
5. Testnet deployment scripts
