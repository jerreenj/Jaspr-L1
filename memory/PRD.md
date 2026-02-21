# JasprChain - Product Requirements Document

## Original Problem Statement
Build JasprChain - a high-performance Layer 1 blockchain with:
- Rust core chain architecture (mirrored in Python)
- Move-based VM integration points
- HyperLiquid-style validator model (4 initial validators)
- AI Sentinel with real ML risk scoring
- Hybrid DEX with off-chain matching, on-chain settlement
- MPC + Account Abstraction wallets
- <2s deterministic finality
- $JJ native token

## Architecture
```
JasprChain v0.1
├── /backend/jasprchain/          # Python L1 implementation (mirrors Rust)
│   ├── consensus/                # BLS signatures, VRF proposer, finality
│   │   ├── block.py              # Block/BlockHeader structures
│   │   ├── validator.py          # ValidatorSet (4 validators)
│   │   ├── proposer.py           # VRF-based proposer selection
│   │   └── finality.py           # Committee finality engine
│   ├── execution/                # Parallel execution engine
│   │   ├── transaction.py        # Transaction types
│   │   └── parallel.py           # Conflict detection, rollback
│   ├── state/                    # Sparse Merkle Tree
│   │   ├── smt.py                # State storage
│   │   └── account.py            # Account abstraction
│   ├── wallet/                   # MPC + AA
│   │   ├── mpc.py                # 2-of-3 threshold signing
│   │   └── aa.py                 # Session keys, spending limits
│   ├── sentinel/                 # AI risk scoring
│   │   ├── model.py              # sklearn ML model
│   │   └── risk_scorer.py        # Real-time scanning
│   ├── dex/                      # Hybrid DEX
│   │   ├── orderbook.py          # Price levels, matching
│   │   ├── settlement.py         # Atomic batch settlement
│   │   └── risk_engine.py        # Margin, liquidation
│   ├── network/                  # Mempool
│   │   └── mempool.py            # Priority lanes
│   └── engine.py                 # Main orchestrator
├── /frontend/src/                # React Dashboard
│   ├── pages/
│   │   ├── Dashboard.js          # Network overview
│   │   ├── BlockExplorer.js      # Block/TX details
│   │   ├── Validators.js         # Validator status
│   │   ├── WalletPage.js         # MPC wallet
│   │   ├── DEXPage.js            # Orderbook trading
│   │   ├── SentinelPage.js       # AI protection
│   │   └── MempoolPage.js        # Lane visualization
│   └── App.js                    # Main app with navigation
```

## User Personas
1. **Crypto Traders** - Use DEX, wallets, need fast finality
2. **Validators** - Monitor performance, stake management
3. **Developers** - Build on JasprChain, deploy contracts
4. **India's 200M+ users** - Mobile-first, safe onboarding

## Core Requirements (Static)
- [x] BLS12-381 consensus signatures
- [x] Ed25519 wallet keys
- [x] 4 validators (HyperLiquid model)
- [x] AI Sentinel with ML risk scoring
- [x] Hybrid DEX orderbook
- [x] MPC + AA wallets
- [x] <2s finality target
- [x] Mempool with priority lanes

## What's Been Implemented (Jan 2026)

### Backend (Python - Rust-mirrored)
- ✅ Complete consensus layer (BLS, VRF, committee finality)
- ✅ Parallel execution engine with conflict detection
- ✅ Sparse Merkle Tree state management
- ✅ MPC wallet with 2-of-3 threshold
- ✅ Account Abstraction (session keys, spending limits)
- ✅ AI Sentinel with sklearn RandomForest/IsolationForest
- ✅ Hybrid DEX orderbook with settlement engine
- ✅ DEX risk engine (margin, liquidation)
- ✅ Mempool with 6 priority lanes
- ✅ WebSocket support for real-time updates
- ✅ Auto block production every 2s

### Frontend (React)
- ✅ Dashboard with network stats
- ✅ Block Explorer with transaction details
- ✅ Validators page with stake/performance
- ✅ Wallet page with MPC creation
- ✅ DEX page with orderbook visualization
- ✅ AI Sentinel page with guard modes
- ✅ Mempool page with lane distribution
- ✅ Real-time updates via polling

## Prioritized Backlog

### P0 (Critical)
- [ ] Rust codebase generation for production
- [ ] Move VM integration (contract deployment)
- [ ] Production-grade BLS (blst crate)

### P1 (High)
- [ ] TradingView chart integration
- [ ] Mobile responsive design
- [ ] WebSocket full implementation
- [ ] Transaction history per wallet
- [ ] Multi-market DEX support

### P2 (Medium)
- [ ] Staking UI with delegation
- [ ] Guardian recovery for MPC wallets
- [ ] 2FA integration for AA wallets
- [ ] Advanced AI Sentinel patterns
- [ ] MEV protection lanes

## Next Tasks
1. Generate downloadable Rust codebase
2. Add TradingView charts to DEX
3. Implement staking/unstaking UI
4. Add more DEX markets (ETH/USDC, BTC/USDC)
5. Mobile-responsive layout
