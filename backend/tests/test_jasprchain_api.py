"""
JasprChain L1 Blockchain API Tests
Tests for tokenomics, staking, wallet, validators, blocks, sentinel, and mempool APIs

UPDATED: Token changed from JSP to JASPR, wallet allocation changed from 1000 to 10,000 JASPR
Tokenomics from litepaper: 1B total supply, 52% community, 15% treasury, etc.
"""
import pytest
import requests
import os

BASE_URL = os.environ.get('REACT_APP_BACKEND_URL', '').rstrip('/')


class TestHealthAndNetwork:
    """Health and network stats tests"""
    
    def test_health_endpoint(self):
        """Test API health check"""
        response = requests.get(f"{BASE_URL}/api/health")
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "healthy"
        assert data["chain_id"] == 1
        assert "height" in data
        assert data["network"] == "testnet"
    
    def test_network_stats(self):
        """Test network statistics endpoint"""
        response = requests.get(f"{BASE_URL}/api/network/stats")
        assert response.status_code == 200
        data = response.json()
        assert data["chain_id"] == 1
        assert "height" in data
        assert "validators" in data
        assert data["validators"]["total"] == 4
        assert data["validators"]["active"] == 4
        assert "mempool" in data
        assert "sentinel" in data
        assert "finality" in data
        assert data["network"] == "testnet"
    
    def test_tps_endpoint(self):
        """Test TPS endpoint"""
        response = requests.get(f"{BASE_URL}/api/network/tps")
        assert response.status_code == 200
        data = response.json()
        assert "current_tps" in data
        assert "target_tps" in data
        assert data["target_tps"] == 10000


class TestTokenomics:
    """Tokenomics API tests - verifying litepaper values"""
    
    def test_tokenomics_endpoint(self):
        """Test tokenomics endpoint returns correct litepaper values"""
        response = requests.get(f"{BASE_URL}/api/tokenomics")
        assert response.status_code == 200
        data = response.json()
        
        # Token info
        assert data["token"]["symbol"] == "JASPR"
        assert data["token"]["name"] == "Jaspr"
        assert data["token"]["decimals"] == 9
        
        # Total supply - 1 billion with 9 decimals
        assert data["supply"]["total"] == 1_000_000_000_000_000_000
        assert "1,000,000,000 JASPR" in data["supply"]["total_formatted"]
        
        # Inflation type
        assert data["inflation"] == "fixed_supply"
        assert data["network"] == "testnet"
    
    def test_tokenomics_distribution_percentages(self):
        """Test tokenomics distribution matches litepaper percentages"""
        response = requests.get(f"{BASE_URL}/api/tokenomics")
        data = response.json()
        
        distribution = data["distribution"]
        
        # Verify percentages from litepaper
        assert distribution["community_incentives"]["percentage"] == 52
        assert distribution["treasury_reserve"]["percentage"] == 15
        assert distribution["liquidity_market_making"]["percentage"] == 10
        assert distribution["team_advisors"]["percentage"] == 10
        assert distribution["investors"]["percentage"] == 8
        assert distribution["ecosystem_partnerships"]["percentage"] == 5
        
        # Verify total is 100%
        total_percentage = sum(d["percentage"] for d in distribution.values())
        assert total_percentage == 100
    
    def test_tokenomics_initial_distribution_amounts(self):
        """Test initial distribution amounts match litepaper"""
        response = requests.get(f"{BASE_URL}/api/tokenomics")
        data = response.json()
        
        distribution = data["distribution"]
        
        # Initial amounts (with 9 decimals)
        assert distribution["community_incentives"]["initial"] == 520_000_000_000_000_000  # 52%
        assert distribution["treasury_reserve"]["initial"] == 150_000_000_000_000_000     # 15%
        assert distribution["liquidity_market_making"]["initial"] == 100_000_000_000_000_000  # 10%
        assert distribution["team_advisors"]["initial"] == 100_000_000_000_000_000        # 10%
        assert distribution["investors"]["initial"] == 80_000_000_000_000_000             # 8%
        assert distribution["ecosystem_partnerships"]["initial"] == 50_000_000_000_000_000  # 5%
    
    def test_tokenomics_treasury_addresses(self):
        """Test treasury addresses are properly defined"""
        response = requests.get(f"{BASE_URL}/api/tokenomics")
        data = response.json()
        
        treasury = data["treasury_addresses"]
        assert treasury["community"] == "jaspr1treasury_community"
        assert treasury["reserve"] == "jaspr1treasury_reserve"
        assert treasury["liquidity"] == "jaspr1treasury_liquidity"
        assert treasury["team"] == "jaspr1treasury_team"
        assert treasury["investors"] == "jaspr1treasury_investors"
        assert treasury["ecosystem"] == "jaspr1treasury_ecosystem"
    
    def test_circulating_supply_increases_with_wallets(self):
        """Test that circulating supply increases when wallets are created"""
        # Get initial circulating supply
        initial_response = requests.get(f"{BASE_URL}/api/tokenomics")
        initial_circulating = initial_response.json()["supply"]["circulating"]
        
        # Create a new wallet (should allocate 10,000 JASPR from community pool)
        requests.post(f"{BASE_URL}/api/wallets/create")
        
        # Get updated circulating supply
        updated_response = requests.get(f"{BASE_URL}/api/tokenomics")
        updated_circulating = updated_response.json()["supply"]["circulating"]
        
        # Circulating supply should increase by 10,000 JASPR (10,000 * 10^9)
        expected_increase = 10_000_000_000_000
        assert updated_circulating >= initial_circulating + expected_increase


class TestValidators:
    """Validator API tests"""
    
    def test_get_all_validators(self):
        """Test getting all validators"""
        response = requests.get(f"{BASE_URL}/api/validators")
        assert response.status_code == 200
        data = response.json()
        assert "validators" in data
        assert len(data["validators"]) == 4
        assert data["total_validators"] == 4
        assert data["active_validators"] == 4
        
        # Verify validator names
        names = [v["name"] for v in data["validators"]]
        assert "Jaspr Labs" in names
        assert "Foundation" in names
        assert "Community" in names
        assert "Ecosystem" in names
    
    def test_get_single_validator(self):
        """Test getting a single validator"""
        response = requests.get(f"{BASE_URL}/api/validators/jaspr1validator1")
        assert response.status_code == 200
        data = response.json()
        assert data["address"] == "jaspr1validator1"
        assert data["name"] == "Jaspr Labs"
        assert data["active"] == True
        assert data["jailed"] == False
        assert "stake" in data
        assert "voting_power" in data
    
    def test_get_nonexistent_validator(self):
        """Test getting a non-existent validator"""
        response = requests.get(f"{BASE_URL}/api/validators/jaspr1nonexistent")
        assert response.status_code == 404


class TestStaking:
    """Staking API tests"""
    
    @pytest.fixture
    def test_wallet(self):
        """Create a test wallet for staking tests"""
        response = requests.post(f"{BASE_URL}/api/wallets/create")
        assert response.status_code == 200
        return response.json()
    
    def test_staking_validators_endpoint(self):
        """Test staking validators endpoint with APY"""
        response = requests.get(f"{BASE_URL}/api/staking/validators")
        assert response.status_code == 200
        data = response.json()
        assert "validators" in data
        assert len(data["validators"]) == 4
        assert "total_stake" in data
        
        # Verify APY is present for each validator
        for validator in data["validators"]:
            assert "apy" in validator
            assert validator["apy"] > 0
            assert "commission_rate" in validator
            assert "stake_share" in validator
            # Verify JASPR in formatted stake
            assert "JASPR" in validator["stake_formatted"]
    
    def test_staking_stats_overview(self):
        """Test staking stats overview endpoint"""
        response = requests.get(f"{BASE_URL}/api/staking/stats/overview")
        assert response.status_code == 200
        data = response.json()
        assert "total_staked" in data
        assert "active_validators" in data
        assert data["active_validators"] == 4
        assert "average_apy" in data
        assert data["average_apy"] > 0
        assert "unbonding_period_days" in data
        assert data["unbonding_period_days"] == 14
        # Verify JASPR in formatted string
        assert "JASPR" in data["total_staked_formatted"]
    
    def test_stake_tokens(self, test_wallet):
        """Test staking tokens to a validator"""
        wallet_address = test_wallet["address"]
        
        # Stake tokens (1000 JASPR)
        stake_response = requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator1",
                "amount": 1000_000_000_000  # 1000 JASPR
            }
        )
        assert stake_response.status_code == 200
        data = stake_response.json()
        assert data["success"] == True
        assert "Staked" in data["message"]
        
        # Verify stake was recorded
        staking_info = requests.get(f"{BASE_URL}/api/staking/{wallet_address}")
        assert staking_info.status_code == 200
        info_data = staking_info.json()
        assert "jaspr1validator1" in info_data["stakes"]
        assert info_data["stakes"]["jaspr1validator1"] == 1000_000_000_000
    
    def test_unstake_tokens(self, test_wallet):
        """Test unstaking tokens from a validator"""
        wallet_address = test_wallet["address"]
        
        # First stake some tokens (500 JASPR)
        requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator2",
                "amount": 500_000_000_000  # 500 JASPR
            }
        )
        
        # Now unstake (200 JASPR)
        unstake_response = requests.post(
            f"{BASE_URL}/api/staking/unstake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator2",
                "amount": 200_000_000_000  # 200 JASPR
            }
        )
        assert unstake_response.status_code == 200
        data = unstake_response.json()
        assert data["success"] == True
        assert "Unstaked" in data["message"]
        
        # Verify remaining stake (300 JASPR)
        staking_info = requests.get(f"{BASE_URL}/api/staking/{wallet_address}")
        info_data = staking_info.json()
        assert info_data["stakes"]["jaspr1validator2"] == 300_000_000_000  # 500 - 200 = 300
    
    def test_stake_insufficient_balance(self, test_wallet):
        """Test staking with insufficient balance"""
        wallet_address = test_wallet["address"]
        
        # Try to stake more than balance (wallet has 10,000 JASPR)
        response = requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator1",
                "amount": 20000_000_000_000  # 20,000 JASPR - more than balance
            }
        )
        assert response.status_code == 400
        assert "Insufficient balance" in response.json()["detail"]
    
    def test_unstake_insufficient_stake(self, test_wallet):
        """Test unstaking more than staked"""
        wallet_address = test_wallet["address"]
        
        # Try to unstake without staking first
        response = requests.post(
            f"{BASE_URL}/api/staking/unstake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator3",
                "amount": 100_000_000_000
            }
        )
        assert response.status_code == 400
        assert "Insufficient stake" in response.json()["detail"]
    
    def test_dynamic_apy_calculation(self):
        """Test that APY varies by validator based on stake share"""
        response = requests.get(f"{BASE_URL}/api/staking/validators")
        data = response.json()
        
        # All validators should have APY > 0
        for validator in data["validators"]:
            assert validator["apy"] > 0
            # APY should be reasonable (between 5% and 15%)
            assert 5 <= validator["apy"] <= 15


class TestWallets:
    """Wallet API tests"""
    
    def test_create_wallet(self):
        """Test wallet creation"""
        response = requests.post(f"{BASE_URL}/api/wallets/create")
        assert response.status_code == 200
        data = response.json()
        assert "address" in data
        assert data["address"].startswith("jaspr1")
        assert "public_key" in data
        assert data["type"] == "mpc_wallet"
        assert data["threshold"] == "2-of-3"
    
    def test_wallet_receives_10000_jaspr(self):
        """Test new wallet receives 10,000 JASPR from community pool"""
        # Create wallet
        create_response = requests.post(f"{BASE_URL}/api/wallets/create")
        wallet = create_response.json()
        
        # Get wallet details
        response = requests.get(f"{BASE_URL}/api/wallets/{wallet['address']}")
        assert response.status_code == 200
        data = response.json()
        
        # Should have 10,000 JASPR (10,000 * 10^9)
        assert data["balance"] == 10_000_000_000_000
        assert "10000" in data["balance_formatted"]
        assert "JASPR" in data["balance_formatted"]
    
    def test_get_wallet_details(self):
        """Test getting wallet details"""
        # Create wallet first
        create_response = requests.post(f"{BASE_URL}/api/wallets/create")
        wallet = create_response.json()
        
        # Get wallet details
        response = requests.get(f"{BASE_URL}/api/wallets/{wallet['address']}")
        assert response.status_code == 200
        data = response.json()
        assert data["address"] == wallet["address"]
        assert data["balance"] == 10_000_000_000_000  # 10,000 JASPR
        assert "JASPR" in data["balance_formatted"]
        assert data["mpc_wallet"] is not None
        assert data["aa_wallet"] is not None
    
    def test_get_wallet_balance(self):
        """Test getting wallet balance"""
        # Create wallet first
        create_response = requests.post(f"{BASE_URL}/api/wallets/create")
        wallet = create_response.json()
        
        # Get balance
        response = requests.get(f"{BASE_URL}/api/wallets/{wallet['address']}/balance")
        assert response.status_code == 200
        data = response.json()
        assert data["address"] == wallet["address"]
        assert data["balance"] == 10_000_000_000_000  # 10,000 JASPR
        assert "JASPR" in data["balance_formatted"]


class TestBlocks:
    """Block API tests"""
    
    def test_get_blocks(self):
        """Test getting blocks list"""
        response = requests.get(f"{BASE_URL}/api/blocks?limit=10")
        assert response.status_code == 200
        data = response.json()
        assert "blocks" in data
        assert "total" in data
        assert "latest_height" in data
        
        # Genesis block should exist
        assert len(data["blocks"]) >= 1
    
    def test_get_genesis_block(self):
        """Test getting genesis block"""
        response = requests.get(f"{BASE_URL}/api/blocks/0")
        assert response.status_code == 200
        data = response.json()
        assert data["header"]["height"] == 0
        assert data["header"]["proposer"] == "jaspr1genesis"
        assert data["finalized"] == True
    
    def test_get_nonexistent_block(self):
        """Test getting non-existent block"""
        response = requests.get(f"{BASE_URL}/api/blocks/999999")
        assert response.status_code == 404


class TestSentinel:
    """AI Sentinel API tests"""
    
    def test_get_sentinel_status(self):
        """Test getting sentinel status"""
        response = requests.get(f"{BASE_URL}/api/sentinel")
        assert response.status_code == 200
        data = response.json()
        assert "guard_mode" in data
        assert data["guard_mode"] in ["passive", "warning", "enforced"]
        assert "stats" in data
        assert "quarantine_count" in data
    
    def test_get_sentinel_stats(self):
        """Test getting sentinel statistics"""
        response = requests.get(f"{BASE_URL}/api/sentinel/stats")
        assert response.status_code == 200
        data = response.json()
        assert "total_scanned" in data
        assert "threats_detected" in data
        assert "transactions_blocked" in data
    
    def test_get_quarantine(self):
        """Test getting quarantined transactions"""
        response = requests.get(f"{BASE_URL}/api/sentinel/quarantine")
        assert response.status_code == 200
        data = response.json()
        assert "quarantined" in data
        assert isinstance(data["quarantined"], list)
    
    def test_set_sentinel_mode(self):
        """Test setting sentinel mode"""
        # Set to warning mode
        response = requests.post(
            f"{BASE_URL}/api/sentinel/mode",
            json={"mode": "warning"}
        )
        assert response.status_code == 200
        assert response.json()["success"] == True
        
        # Verify mode changed
        status = requests.get(f"{BASE_URL}/api/sentinel")
        assert status.json()["guard_mode"] == "warning"
        
        # Reset to enforced
        requests.post(f"{BASE_URL}/api/sentinel/mode", json={"mode": "enforced"})


class TestMempool:
    """Mempool API tests"""
    
    def test_get_mempool_stats(self):
        """Test getting mempool statistics"""
        response = requests.get(f"{BASE_URL}/api/mempool")
        assert response.status_code == 200
        data = response.json()
        assert "stats" in data
        assert "quarantined" in data
        assert "total_pending" in data["stats"]
        assert "by_lane" in data["stats"]
        
        # Verify lane structure
        lanes = data["stats"]["by_lane"]
        assert "priority" in lanes
        assert "normal" in lanes
        assert "liquidation" in lanes
        assert "quarantine" in lanes


class TestTransactions:
    """Transaction API tests"""
    
    def test_transfer_transaction(self):
        """Test creating a transfer transaction"""
        # Create two wallets
        sender_response = requests.post(f"{BASE_URL}/api/wallets/create")
        sender = sender_response.json()
        
        recipient_response = requests.post(f"{BASE_URL}/api/wallets/create")
        recipient = recipient_response.json()
        
        # Create transfer (100 JASPR)
        response = requests.post(
            f"{BASE_URL}/api/transactions/transfer",
            json={
                "sender": sender["address"],
                "recipient": recipient["address"],
                "amount": 100_000_000_000  # 100 JASPR
            }
        )
        assert response.status_code == 200
        data = response.json()
        assert data["success"] == True
        assert "tx_hash" in data
        assert data["status"] == "pending"


class TestTokenSymbol:
    """Tests to verify JASPR token symbol is used throughout (not JSP or JJ)"""
    
    def test_wallet_balance_shows_jaspr(self):
        """Verify wallet balance shows JASPR not JSP or JJ"""
        response = requests.post(f"{BASE_URL}/api/wallets/create")
        wallet = response.json()
        
        details = requests.get(f"{BASE_URL}/api/wallets/{wallet['address']}")
        data = details.json()
        
        # Should show JASPR, not JSP or JJ
        assert "JASPR" in data["balance_formatted"]
        assert "JSP" not in data["balance_formatted"] or "JASPR" in data["balance_formatted"]
        assert "JJ" not in data["balance_formatted"]
    
    def test_staking_stats_shows_jaspr(self):
        """Verify staking stats shows JASPR"""
        response = requests.get(f"{BASE_URL}/api/staking/stats/overview")
        data = response.json()
        
        assert "JASPR" in data["total_staked_formatted"]
        assert "JJ" not in data["total_staked_formatted"]
    
    def test_validators_stake_shows_jaspr(self):
        """Verify validators stake shows JASPR"""
        response = requests.get(f"{BASE_URL}/api/staking/validators")
        data = response.json()
        
        for validator in data["validators"]:
            assert "JASPR" in validator["stake_formatted"]
            assert "JJ" not in validator["stake_formatted"]
    
    def test_tokenomics_shows_jaspr(self):
        """Verify tokenomics endpoint shows JASPR"""
        response = requests.get(f"{BASE_URL}/api/tokenomics")
        data = response.json()
        
        assert data["token"]["symbol"] == "JASPR"
        assert "JASPR" in data["supply"]["total_formatted"]
        assert "JASPR" in data["supply"]["circulating_formatted"]


class TestFullStakingFlow:
    """End-to-end staking flow tests"""
    
    def test_full_staking_flow(self):
        """Test complete staking flow: create wallet -> stake -> unstake"""
        # 1. Create wallet
        wallet_response = requests.post(f"{BASE_URL}/api/wallets/create")
        assert wallet_response.status_code == 200
        wallet = wallet_response.json()
        wallet_address = wallet["address"]
        
        # 2. Verify wallet has 10,000 JASPR
        balance_response = requests.get(f"{BASE_URL}/api/wallets/{wallet_address}")
        assert balance_response.json()["balance"] == 10_000_000_000_000
        
        # 3. Stake 1000 JASPR to validator
        stake_response = requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator1",
                "amount": 1000_000_000_000  # 1000 JASPR
            }
        )
        assert stake_response.status_code == 200
        assert stake_response.json()["success"] == True
        
        # 4. Verify stake recorded
        staking_info = requests.get(f"{BASE_URL}/api/staking/{wallet_address}")
        assert staking_info.json()["stakes"]["jaspr1validator1"] == 1000_000_000_000
        
        # 5. Verify balance reduced
        balance_after_stake = requests.get(f"{BASE_URL}/api/wallets/{wallet_address}")
        assert balance_after_stake.json()["balance"] == 9000_000_000_000  # 10000 - 1000
        
        # 6. Unstake 500 JASPR
        unstake_response = requests.post(
            f"{BASE_URL}/api/staking/unstake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator1",
                "amount": 500_000_000_000  # 500 JASPR
            }
        )
        assert unstake_response.status_code == 200
        assert unstake_response.json()["success"] == True
        
        # 7. Verify stake reduced
        staking_info_after = requests.get(f"{BASE_URL}/api/staking/{wallet_address}")
        assert staking_info_after.json()["stakes"]["jaspr1validator1"] == 500_000_000_000
        
        # 8. Verify balance increased
        balance_after_unstake = requests.get(f"{BASE_URL}/api/wallets/{wallet_address}")
        assert balance_after_unstake.json()["balance"] == 9500_000_000_000  # 9000 + 500


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
