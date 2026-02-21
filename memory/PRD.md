# JasprChain - Product Requirements Document

## Original Problem Statement
Build JasprChain - a high-performance Layer 1 blockchain TESTNET:
- Core L1 foundation that applications can be built on top of
- **LMDB Persistence** - Blocks survive restart
- **14-day Unbonding Period** - Real staking economics
- $JASPR tokenomics from litepaper
- HyperLiquid-style validators (4 initial)
- AI Sentinel with ML risk scoring
- MPC + Account Abstraction wallets
- <2s deterministic finality

**This is TESTNET/DEVNET - NOT mainnet**

## Key Features Implemented

### 1. LMDB Persistence ✅
- **Database:** LMDB at `/app/data/jasprchain`
- **Persisted Data:** Blocks, state (balances, stakes), unbonding entries, wallets, metadata
- **Blocks survive restart** - Chain continues from last height
- **API:** `GET /api/network/persistence` for DB stats

### 2. 14-Day Unbonding Period ✅
- Unstaking tokens triggers 14-day lock period
- Tokens cannot be used during unbonding
- After 14 days, tokens become claimable
- **APIs:**
  - `GET /api/staking/{address}/unbonding` - View unbonding entries
  - `POST /api/staking/{address}/claim` - Claim completed unbonding
- **Entry Fields:** delegator, validator, amount, unlock_time, remaining_days, is_claimable

### 3. $JASPR Tokenomics (from Litepaper) ✅
- **Total Supply:** 1,000,000,000 (Fixed - no inflation)
- **Distribution:**
  - Community Incentives: 52% (520,000,000)
  - Treasury Reserve: 15% (150,000,000)
  - Liquidity & Market Making: 10% (100,000,000)
  - Core Team & Advisors: 10% (100,000,000)
  - Investors: 8% (80,000,000)
  - Ecosystem & Partnerships: 5% (50,000,000)
- **New Wallets:** 10,000 JASPR from community pool

### 4. Real-Time Block Production ✅
- Blocks produced every 2 seconds
- Blocks include transactions from mempool
- All blocks marked FINALIZED (1ms finality)
- Persisted to LMDB

### 5. Dynamic APY Calculation ✅
- Base rate: 8% annual
- Performance bonus: up to 4%
- Stake concentration penalty
- Commission deduction (5% default)
- **Average APY:** ~9.5%

## Test Results
- **Backend:** 53/53 tests passed (100%)
- **Frontend:** All tests passed (100%)
- **Test report:** `/app/test_reports/iteration_4.json`

## Architecture

```
/app/
├── backend/
│   ├── jasprchain/
│   │   ├── storage/        # NEW: LMDB persistence
│   │   │   └── persistence.py
│   │   ├── consensus/
│   │   ├── execution/
│   │   ├── state/
│   │   ├── wallet/
│   │   ├── sentinel/
│   │   ├── network/
│   │   └── engine.py       # Core orchestrator
│   └── server.py           # FastAPI endpoints
├── frontend/
│   └── src/pages/          # React dashboard
├── data/
│   └── jasprchain/         # LMDB database files
└── rust-core/              # Rust boilerplate for mainnet
```

## API Endpoints

### Persistence
- `GET /api/network/persistence` - DB stats

### Unbonding
- `GET /api/staking/{address}/unbonding` - Unbonding entries
- `POST /api/staking/{address}/claim` - Claim completed

### Tokenomics
- `GET /api/tokenomics` - Full tokenomics info

### Staking
- `GET /api/staking/stats/overview` - Network staking stats
- `GET /api/staking/validators` - Validators with APY
- `POST /api/staking/stake` - Stake tokens
- `POST /api/staking/unstake` - Unstake (starts unbonding)
- `GET /api/staking/{address}` - User staking info

### Core
- `GET /api/health` - Chain health
- `GET /api/blocks` - Block list (persisted=True)
- `GET /api/validators` - Validator list
- `POST /api/wallets/create` - Create MPC wallet

## What's NOT Implemented (Future)
- ❌ DEX (application layer - build on top)
- ❌ libp2p networking (use simulation)
- ❌ Move VM (use simulation)
- ❌ Slashing (placeholder)

## Prioritized Backlog

### P0 (For Production Mainnet)
- [ ] libp2p networking
- [ ] Move VM integration
- [ ] Genesis config file
- [ ] Slashing mechanism

### P1 (High)
- [ ] RPC server (JSON-RPC 2.0)
- [ ] Light client support
- [ ] Archive node support

### P2 (Medium)
- [ ] Move module deployment
- [ ] Gas estimation
- [ ] Validator rewards distribution

## MOCKED Components
- Blockchain consensus (simulated, not distributed)
- BLS signatures (simulated)
- MPC wallet (simplified crypto)
- AI Sentinel ML (mocked)
- **Unbonding period is REAL 14 days**

---
*Last Updated: December 2025*
*Status: TESTNET with Persistence - Ready for investor demonstration*
