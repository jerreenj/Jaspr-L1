# JasprChain Rust Implementation

High-performance Layer 1 blockchain built in Rust.

## Project Structure

```
jasprchain/
├── Cargo.toml              # Workspace manifest
├── src/
│   ├── lib.rs              # Main library entry
│   ├── consensus/          # BLS signatures, VRF, finality
│   ├── execution/          # Parallel transaction execution
│   ├── state/              # Sparse Merkle Tree
│   ├── wallet/             # MPC + Account Abstraction
│   ├── sentinel/           # AI risk scoring (hooks)
│   ├── network/            # P2P, mempool
│   └── crypto/             # Ed25519, BLS12-381
└── move-runtime/           # Move VM integration
```

## Build

```bash
cargo build --release
```

## Run Node

```bash
cargo run --release -- --config config.toml
```

## Features

- BLS12-381 signatures for consensus
- Ed25519 for wallet keys
- Parallel execution with conflict detection
- Sparse Merkle Tree state
- MPC wallet support
- AI Sentinel hooks
- Move VM integration (via move-vm-runtime)

## License

MIT - Jaspr Labs 2025
