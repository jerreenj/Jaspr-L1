# JasprChain

A high-performance Layer 1 blockchain with Move VM smart contract support.

## Architecture

```
├── rust-core/          # Core blockchain (Rust)
│   ├── jaspr-types     # Core types
│   ├── jaspr-crypto    # Cryptography
│   ├── jaspr-state     # State storage (RocksDB)
│   ├── jaspr-consensus # PoS consensus
│   ├── jaspr-executor  # Transaction execution
│   ├── jaspr-move-vm   # Move VM integration
│   ├── jaspr-network   # P2P networking
│   └── jaspr-node      # Full node
├── backend/            # Testnet simulation (Python)
└── frontend/           # Dashboard (React)
```

## Quick Start

### Rust Core
```bash
cd rust-core
cargo build --release
```

### Testnet Demo
```bash
# Backend
cd backend
pip install -r requirements.txt
python server.py

# Frontend
cd frontend
yarn install
yarn start
```

## Features

- **Proof of Stake** consensus with BFT finality
- **Move VM** smart contracts
- **Sub-second** block times
- **High throughput** transaction processing
- **RocksDB** for persistent state

## License

MIT
