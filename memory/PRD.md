# JasprChain - Product Requirements Document

## Original Problem Statement
Build JasprChain - a high-performance Layer 1 blockchain TESTNET with full core features:
- **Slashing** - Validator misbehavior penalties
- **P2P Networking** - Peer-to-peer communication layer
- **Move VM** - Smart contract execution
- LMDB Persistence - Blocks survive restart
- 14-day Unbonding Period - Real staking economics
- $JASPR Tokenomics - From litepaper (1B fixed supply)
- HyperLiquid-style validators (4 initial)
- AI Sentinel with ML risk scoring
- MPC + Account Abstraction wallets
- <2s deterministic finality

**This is TESTNET/DEVNET - Foundation for applications to be built on top**

## Key Features Implemented

### 1. Slashing Mechanism ✅
- **Double Signing:** 5% stake slashed, 7-day jail, permanent tombstone
- **Downtime:** 0.1% stake slashed per window, 1-day jail
- **Invalid Attestation:** 3% stake slashed, 3-day jail
- **APIs:**
  - `GET /api/slashing/stats` - Slashing statistics
  - `GET /api/slashing/history` - Slashing records
  - `GET /api/slashing/validator/{address}` - Validator signing info
  - `POST /api/slashing/unjail/{address}` - Unjail validator

### 2. P2P Networking ✅
- **Gossip Protocol:** Fanout=6, max_hops=4 for message propagation
- **Peer Management:** Connect, disconnect, ping/pong, reputation
- **Message Types:** Handshake, new_block, new_transaction, sync
- **APIs:**
  - `GET /api/p2p/info` - Node info (node_id, version, peers)
  - `GET /api/p2p/peers` - Connected peers list
  - `GET /api/p2p/stats` - Network statistics
  - `POST /api/p2p/connect` - Connect to peer

### 3. Move VM ✅
- **Standard Library:** JASPR, Coin, Account modules
- **Functions:** transfer, mint, burn, create_account, balance
- **Gas Metering:** GAS_UNIT_PRICE = 100 nanoJASPR
- **Module Deployment:** Custom modules can be deployed
- **APIs:**
  - `GET /api/move/modules` - List deployed modules
  - `GET /api/move/module/{id}` - Get module details
  - `POST /api/move/execute` - Execute function
  - `POST /api/move/deploy` - Deploy module
  - `GET /api/move/estimate-gas` - Gas estimation
  - `GET /api/move/stats` - VM statistics

### 4. LMDB Persistence ✅
- Blocks, state, unbonding entries persist to `/app/data/jasprchain`
- Chain continues from last height after restart

### 5. 14-Day Unbonding ✅
- Unstaking triggers 14-day lock period
- Tokens become claimable after period ends

### 6. $JASPR Tokenomics ✅
- **Total Supply:** 1,000,000,000 (Fixed - no inflation)
- **Distribution:**
  - Community Incentives: 52%
  - Treasury Reserve: 15%
  - Liquidity: 10%
  - Team: 10%
  - Investors: 8%
  - Ecosystem: 5%

## Test Results
- **Backend:** 81/81 tests passed (100%)
- **Frontend:** All tests passed (100%)
- **Test report:** `/app/test_reports/iteration_5.json`

## Architecture

```
/app/
├── backend/
│   ├── jasprchain/
│   │   ├── consensus/
│   │   │   ├── slashing.py      # NEW: Slashing module
│   │   │   ├── validator.py
│   │   │   └── finality.py
│   │   ├── execution/
│   │   │   ├── move_vm.py       # NEW: Move VM
│   │   │   └── parallel.py
│   │   ├── network/
│   │   │   ├── p2p.py           # NEW: P2P networking
│   │   │   └── mempool.py
│   │   ├── storage/
│   │   │   └── persistence.py   # LMDB
│   │   ├── wallet/
│   │   ├── sentinel/
│   │   └── engine.py            # Orchestrator
│   └── server.py                # FastAPI
├── frontend/
│   └── src/pages/               # React dashboard
├── data/
│   └── jasprchain/              # LMDB database
└── rust-core/                   # Rust boilerplate
```

## API Endpoints Summary

### Core
- Health, network stats, blocks, validators

### Tokenomics
- `GET /api/tokenomics`

### Staking
- stake, unstake, validators, unbonding, claim

### Slashing (NEW)
- stats, history, validator info, unjail

### P2P (NEW)
- info, peers, stats, connect

### Move VM (NEW)
- modules, execute, deploy, estimate-gas

## MOCKED Components (Simulation)
- Move VM bytecode execution SIMULATED
- P2P network NOT STARTED (is_running=false)
- Slashing works but no real consensus trigger
- Blockchain consensus simulated
- BLS signatures simulated

## What's Production-Ready
- All APIs functional
- State persistence working
- Tokenomics from litepaper
- Staking with APY calculation
- 14-day unbonding period

## Future (For Mainnet)
- Real Move bytecode interpreter
- Start P2P network with real peers
- Automatic slashing trigger from consensus
- RocksDB for higher performance
- Genesis config file

---
*Last Updated: December 2025*
*Status: TESTNET with Slashing, P2P, Move VM*
*Test Coverage: 81/81 backend tests passed (100%)*
