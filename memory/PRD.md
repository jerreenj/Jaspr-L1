# JasprChain - Product Requirements Document

## Status: TESTNET with Mobile Responsive UI + Genesis Config

**Chain ID:** `jasprchain-testnet-1`

## What's Running

### Real-Time Activity
- **Blocks:** Every 2 seconds, all FINALIZED
- **Validators:** 20 active validators
- **Staking:** Random stakes/unstakes (1-10K JASPR)
- **P2P:** 5 simulated peers
- **Move VM:** Token transfers, account creations

### Random Sequence Logic
1. **Staking Simulation:**
   - Random amount: 1-10,000 JASPR
   - Random interval: 5-60 seconds
   - Actions: 70% stake, 30% unstake
   - Source: Community pool (up to 500M JASPR)

2. **Validators (20 total):**
   - Initial 4: Jaspr Labs (10K), Foundation (8K), Community (6K), Ecosystem (4K)
   - Additional 16: Alpha to Pi Node (3.5K decreasing to 950K)

## Mobile Responsive Design ✅
- **Hamburger menu** on mobile (< 768px)
- **2x2 grid** for stats (vs 4-column on desktop)
- **Card layout** for validators on mobile
- **Touch-friendly** tap targets (44px minimum)
- **Responsive text** sizing

## Genesis Configuration ✅
**Endpoint:** `GET /api/genesis`

```json
{
  "chain_id": "jasprchain-testnet-1",
  "chain_name": "JasprChain Testnet",
  "genesis_time": "2025-01-01T00:00:00.000000000Z",
  "consensus_params": {...},
  "app_state": {
    "bank": {"supply": "1000000000000000000 ujaspr"},
    "staking": {"max_validators": 100, "bond_denom": "ujaspr"},
    "slashing": {"slash_fraction_double_sign": "0.05"},
    "move": {"modules": ["JASPR", "Coin", "Account"]},
    "sentinel": {"guard_mode": "ENFORCED"}
  },
  "validators": [4 genesis validators]
}
```

## Test Results
- **Backend:** 22/22 passed (100%)
- **Frontend:** All passed (100%)
- **Report:** `/app/test_reports/iteration_7.json`

## API Endpoints

### Genesis
- `GET /api/genesis` - Full genesis config

### Core
- `GET /api/health` - testnet status
- `GET /api/network/stats` - 20 validators, TPS
- `GET /api/blocks` - Block list

### Staking
- `GET /api/staking/validators` - 20 validators with APY
- `POST /api/staking/stake` - Stake tokens
- `POST /api/staking/unstake` - 14-day unbonding

### P2P
- `GET /api/p2p/info` - is_running=true, 5 peers
- `GET /api/p2p/peers` - Peer list

### Move VM
- `GET /api/move/modules` - Stdlib modules
- `POST /api/move/execute` - Execute function

### WebSocket
- `ws://*/ws` - Real-time events (new_block, stats_update, staking_update, move_event)

## MOCKED Components
- P2P peers SIMULATED
- Move VM bytecode SIMULATED
- Staking activity AUTOMATED
- Slashing AUTOMATED
- Consensus SIMULATED
- BLS signatures SIMULATED

---
*Last Updated: December 2025*
*Test Coverage: 100% (22/22 backend, all frontend)*
*Mobile: Responsive 390x844 to 1920x1080*
