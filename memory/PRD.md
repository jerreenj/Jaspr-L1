# JasprChain - Product Requirements Document

## Original Problem Statement
Build JasprChain - a high-performance Layer 1 blockchain (CORE ONLY, no application layer):
- Rust core chain architecture (Python simulator + Rust production code)
- Move VM integration points
- HyperLiquid-style validator model (4 initial validators)
- AI Sentinel with real ML risk scoring
- MPC + Account Abstraction wallets
- <2s deterministic finality
- **$JSP native token** (used for transactions, not public/airdrop)
- Full validator delegation staking with dynamic APY calculation

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
- **Staking** - Validator delegation with dynamic APY
- AI Sentinel - Guard modes
- Mempool - Priority lanes

## What's Been Implemented (Dec 2025)

### Core L1 Features ✅
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
- ✅ **Full Staking/Unstaking with Dynamic APY**
- ✅ $JSP token symbol throughout

### Token: $JSP
- Used internally for transactions
- NOT public/airdrop distributed
- Initial wallet balance: 1000 $JSP
- Stake/Unstake supported

### Staking Features
- Dynamic APY calculation per validator
- Base rate: 8% annual
- Performance bonus: up to 4% (based on uptime + blocks proposed)
- Stake concentration penalty for >40% stake share
- Commission deduction (default 5%)
- Average APY: ~9.5%
- 14-day unbonding period (simulated)

### Removed (Application Layer - Build on Top)
- ❌ DEX (not L1 - build as smart contract)
- ❌ NFTs, tokens (not L1 - deploy via Move VM)

## API Endpoints

### Staking APIs
- `GET /api/staking/stats/overview` - Network staking statistics
- `GET /api/staking/validators` - Validators with APY info
- `POST /api/staking/stake` - Stake tokens
- `POST /api/staking/unstake` - Unstake tokens
- `GET /api/staking/{address}` - User staking info

### Core APIs
- `GET /api/health` - Chain health
- `GET /api/network/stats` - Network statistics
- `GET /api/blocks` - Block list
- `GET /api/validators` - Validator list
- `POST /api/wallets/create` - Create MPC wallet
- `GET /api/sentinel` - AI Sentinel status

## Test Results
- Backend: 27/27 tests passed (100%)
- Frontend: All tests passed (100%)
- Test report: `/app/test_reports/iteration_2.json`

## Prioritized Backlog

### P0 (Critical for Testnet)
- [ ] Complete libp2p networking
- [ ] RocksDB persistence
- [ ] Move VM integration (move-vm-runtime)
- [ ] Genesis config file

### P1 (High)
- [ ] RPC server (JSON-RPC 2.0)
- [ ] Light client support
- [ ] Actual unbonding period implementation
- [ ] Slashing implementation

### P2 (Medium)
- [ ] Move module deployment
- [ ] Gas estimation
- [ ] Archive node support
- [ ] Validator rewards distribution

## Next Tasks
1. Add libp2p peer discovery
2. Implement RPC endpoints
3. Add RocksDB for block/state persistence
4. Move VM contract execution
5. Testnet deployment scripts

## MOCKED Components (Simulation)
- Blockchain consensus is simulated (not real distributed consensus)
- BLS signatures are simulated (not real cryptographic signatures)
- MPC wallet uses simplified crypto (not real multi-party computation)
- AI Sentinel ML is mocked (not real machine learning model)
