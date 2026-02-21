# JasprChain - Product Requirements Document

## Original Problem Statement
Build JasprChain - a high-performance Layer 1 blockchain TESTNET:
- Core L1 foundation that applications can be built on top of
- Rust core chain architecture (Python simulator + Rust production code)
- Move VM integration points (for future)
- HyperLiquid-style validator model (4 initial validators)
- AI Sentinel with ML risk scoring
- MPC + Account Abstraction wallets
- <2s deterministic finality
- **$JASPR native token** (from litepaper tokenomics)
- Full validator delegation staking with dynamic APY

**This is TESTNET/DEVNET for investor demonstration - NOT mainnet**

## $JASPR Tokenomics (from Litepaper)

### Token Details
- **Symbol:** JASPR
- **Name:** Jaspr
- **Total Supply:** 1,000,000,000 (1 billion) - FIXED
- **Decimals:** 9
- **Inflation:** None (fixed supply)

### Distribution
| Allocation | Percentage | Amount |
|------------|------------|--------|
| Community Incentives | 52% | 520,000,000 |
| Treasury Reserve | 15% | 150,000,000 |
| Liquidity & Market Making | 10% | 100,000,000 |
| Core Team & Advisors | 10% | 100,000,000 |
| Investors (Pre-Seed + Seed) | 8% | 80,000,000 |
| Ecosystem & Partnerships | 5% | 50,000,000 |

### Treasury Addresses (Testnet)
- `jaspr1treasury_community` - Community pool (funds new wallets)
- `jaspr1treasury_reserve` - Treasury reserve
- `jaspr1treasury_liquidity` - Liquidity/market making
- `jaspr1treasury_team` - Team/advisors
- `jaspr1treasury_investors` - Investor allocation
- `jaspr1treasury_ecosystem` - Ecosystem/partnerships

### Testnet Wallet Allocation
- New wallets receive **10,000 JASPR** from community pool
- Circulating supply increases as wallets are created

## Architecture

### Python Simulator (`/app/backend/jasprchain/`)
Working testnet simulation that mirrors future Rust implementation

### React Dashboard (`/app/frontend/`)
- Dashboard - Network overview with TESTNET badge
- Block Explorer - Block/TX details  
- Validators - Stake distribution
- Wallet - MPC wallet creation (10,000 JASPR initial balance)
- Staking - Validator delegation with dynamic APY
- AI Sentinel - Guard modes (PASSIVE/WARNING/ENFORCED)
- Mempool - Priority lanes

### Production Rust Codebase (`/app/rust-core/`)
Boilerplate for future mainnet development

## What's Been Implemented (Dec 2025) ✅

### Core L1 Features
- ✅ BLS12-381 consensus signatures
- ✅ Ed25519 wallet keys
- ✅ VRF-based proposer selection
- ✅ Committee finality with 2/3 threshold
- ✅ 4 validators (HyperLiquid model)
- ✅ Parallel execution with conflict detection
- ✅ Sparse Merkle Tree state
- ✅ MPC wallet (2-of-3 threshold)
- ✅ Account Abstraction (spending limits, blocked addresses)
- ✅ AI Sentinel with ML scoring
- ✅ Mempool with 6 priority lanes
- ✅ Full Staking/Unstaking with Dynamic APY
- ✅ $JASPR tokenomics from litepaper
- ✅ Treasury accounts with proper distribution
- ✅ Testnet branding throughout

### Staking APY Calculation
- Base rate: 8% annual
- Performance bonus: up to 4% (uptime + blocks proposed)
- Stake concentration penalty: for >40% stake share
- Commission deduction: default 5%
- **Average APY: ~9.5%**

## API Endpoints

### Tokenomics
- `GET /api/tokenomics` - Full tokenomics info

### Staking
- `GET /api/staking/stats/overview` - Network staking stats
- `GET /api/staking/validators` - Validators with APY
- `POST /api/staking/stake` - Stake tokens
- `POST /api/staking/unstake` - Unstake tokens
- `GET /api/staking/{address}` - User staking info

### Core
- `GET /api/health` - Chain health (includes network=testnet)
- `GET /api/network/stats` - Network statistics
- `GET /api/blocks` - Block list
- `GET /api/validators` - Validator list
- `POST /api/wallets/create` - Create MPC wallet

## Test Results
- **Backend:** 36/36 tests passed (100%)
- **Frontend:** All tests passed (100%)
- **Test report:** `/app/test_reports/iteration_3.json`

## Roadmap

### Phase 1: Testnet Foundation (CURRENT) ✅
- Core L1 simulation working
- Tokenomics implemented
- Staking functional
- Dashboard for investor demo

### Phase 2: DEX Integration (NEXT)
- Connect DEX application layer on top of L1
- Transaction flow through JasprChain testnet
- Economics validation

### Phase 3: Team Handoff
- Complete Rust codebase
- Documentation for engineering team
- Mainnet development begins

## Prioritized Backlog

### P0 (Critical for Production)
- [ ] Complete libp2p networking
- [ ] RocksDB persistence
- [ ] Move VM integration
- [ ] Genesis config file

### P1 (High)
- [ ] RPC server (JSON-RPC 2.0)
- [ ] Light client support
- [ ] Actual unbonding period (14 days)
- [ ] Slashing implementation

### P2 (Medium)
- [ ] Move module deployment
- [ ] Gas estimation
- [ ] Archive node support
- [ ] Validator rewards distribution

## MOCKED Components (Simulation)
- Blockchain consensus is simulated (not real distributed consensus)
- BLS signatures are simulated (not real cryptographic signatures)
- MPC wallet uses simplified crypto (not real multi-party computation)
- AI Sentinel ML is mocked (not real machine learning model)

---
*Last Updated: December 2025*
*Status: TESTNET - Ready for investor demonstration*
