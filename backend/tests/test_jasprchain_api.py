"""
JasprChain L1 Blockchain API Tests
Tests for staking, wallet, validators, blocks, sentinel, and mempool APIs
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
    
    def test_tps_endpoint(self):
        """Test TPS endpoint"""
        response = requests.get(f"{BASE_URL}/api/network/tps")
        assert response.status_code == 200
        data = response.json()
        assert "current_tps" in data
        assert "target_tps" in data
        assert data["target_tps"] == 10000


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
    
    def test_stake_tokens(self, test_wallet):
        """Test staking tokens to a validator"""
        wallet_address = test_wallet["address"]
        
        # Stake tokens
        stake_response = requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator1",
                "amount": 100000000000  # 100 JSP
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
        assert info_data["stakes"]["jaspr1validator1"] == 100000000000
    
    def test_unstake_tokens(self, test_wallet):
        """Test unstaking tokens from a validator"""
        wallet_address = test_wallet["address"]
        
        # First stake some tokens
        requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator2",
                "amount": 50000000000  # 50 JSP
            }
        )
        
        # Now unstake
        unstake_response = requests.post(
            f"{BASE_URL}/api/staking/unstake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator2",
                "amount": 20000000000  # 20 JSP
            }
        )
        assert unstake_response.status_code == 200
        data = unstake_response.json()
        assert data["success"] == True
        assert "Unstaked" in data["message"]
        
        # Verify remaining stake
        staking_info = requests.get(f"{BASE_URL}/api/staking/{wallet_address}")
        info_data = staking_info.json()
        assert info_data["stakes"]["jaspr1validator2"] == 30000000000  # 50 - 20 = 30
    
    def test_stake_insufficient_balance(self, test_wallet):
        """Test staking with insufficient balance"""
        wallet_address = test_wallet["address"]
        
        # Try to stake more than balance (wallet has 1000 JSP)
        response = requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator1",
                "amount": 2000000000000  # 2000 JSP - more than balance
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
                "amount": 100000000000
            }
        )
        assert response.status_code == 400
        assert "Insufficient stake" in response.json()["detail"]


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
        assert data["balance"] == 1000000000000  # Initial balance 1000 JSP
        assert "JSP" in data["balance_formatted"]
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
        assert data["balance"] == 1000000000000
        assert "JSP" in data["balance_formatted"]


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
        
        # Create transfer
        response = requests.post(
            f"{BASE_URL}/api/transactions/transfer",
            json={
                "sender": sender["address"],
                "recipient": recipient["address"],
                "amount": 10000000000  # 10 JSP
            }
        )
        assert response.status_code == 200
        data = response.json()
        assert data["success"] == True
        assert "tx_hash" in data
        assert data["status"] == "pending"


class TestTokenSymbol:
    """Tests to verify $JSP token symbol is used throughout"""
    
    def test_wallet_balance_shows_jsp(self):
        """Verify wallet balance shows $JSP not JJ"""
        response = requests.post(f"{BASE_URL}/api/wallets/create")
        wallet = response.json()
        
        details = requests.get(f"{BASE_URL}/api/wallets/{wallet['address']}")
        data = details.json()
        
        # Should show JSP, not JJ
        assert "JSP" in data["balance_formatted"]
        assert "JJ" not in data["balance_formatted"]
    
    def test_staking_stats_shows_jsp(self):
        """Verify staking stats shows $JSP"""
        response = requests.get(f"{BASE_URL}/api/staking/stats/overview")
        data = response.json()
        
        assert "JSP" in data["total_staked_formatted"]
        assert "JJ" not in data["total_staked_formatted"]
    
    def test_validators_stake_shows_jsp(self):
        """Verify validators stake shows $JSP"""
        response = requests.get(f"{BASE_URL}/api/staking/validators")
        data = response.json()
        
        for validator in data["validators"]:
            assert "JSP" in validator["stake_formatted"]
            assert "JJ" not in validator["stake_formatted"]


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
