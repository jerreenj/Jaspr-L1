"""
JasprChain Persistence and Unbonding Tests
Tests for:
1. LMDB Persistence - blocks survive restart
2. 14-day unbonding period for unstaking
3. Unbonding entries with remaining_days and is_claimable
4. Claim unbonded tokens API
"""
import pytest
import requests
import os
import time

BASE_URL = os.environ.get('REACT_APP_BACKEND_URL', '').rstrip('/')


class TestPersistence:
    """Tests for LMDB persistence layer"""
    
    def test_persistence_api_returns_stats(self):
        """Test /api/network/persistence returns DB stats"""
        response = requests.get(f"{BASE_URL}/api/network/persistence")
        assert response.status_code == 200
        data = response.json()
        
        # Verify persistence type
        assert data["persistence"] == "lmdb"
        assert data["db_path"] == "/app/data/jasprchain"
        
        # Verify stats structure
        assert "stats" in data
        stats = data["stats"]
        assert "page_size" in stats
        assert "entries" in stats
        assert "map_size" in stats
        assert "last_txnid" in stats
        
        # Verify latest height is tracked
        assert "latest_height" in data
        assert data["latest_height"] >= 0
    
    def test_blocks_are_persisted(self):
        """Test that blocks are persisted and height increases"""
        # Get initial height
        response1 = requests.get(f"{BASE_URL}/api/network/persistence")
        initial_height = response1.json()["latest_height"]
        
        # Wait for a new block (2 second block time)
        time.sleep(3)
        
        # Get updated height
        response2 = requests.get(f"{BASE_URL}/api/network/persistence")
        updated_height = response2.json()["latest_height"]
        
        # Height should have increased
        assert updated_height > initial_height
    
    def test_blocks_endpoint_shows_persisted_flag(self):
        """Test that blocks endpoint shows persisted=True"""
        response = requests.get(f"{BASE_URL}/api/blocks?limit=5")
        assert response.status_code == 200
        data = response.json()
        
        assert data["persisted"] == True
        assert len(data["blocks"]) > 0
    
    def test_block_production_continues(self):
        """Test that block production continues in real-time"""
        # Get current height
        response1 = requests.get(f"{BASE_URL}/api/health")
        height1 = response1.json()["height"]
        
        # Wait for blocks
        time.sleep(5)
        
        # Get new height
        response2 = requests.get(f"{BASE_URL}/api/health")
        height2 = response2.json()["height"]
        
        # Should have produced at least 1 block in 5 seconds (2s block time)
        assert height2 > height1


class TestUnbonding:
    """Tests for 14-day unbonding period"""
    
    @pytest.fixture
    def test_wallet(self):
        """Create a test wallet for unbonding tests"""
        response = requests.post(f"{BASE_URL}/api/wallets/create")
        assert response.status_code == 200
        return response.json()
    
    def test_unstake_creates_unbonding_entry(self, test_wallet):
        """Test that unstaking creates an unbonding entry instead of returning tokens immediately"""
        wallet_address = test_wallet["address"]
        
        # First stake some tokens (2000 JASPR)
        stake_response = requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator1",
                "amount": 2000_000_000_000  # 2000 JASPR
            }
        )
        assert stake_response.status_code == 200
        
        # Get balance before unstake
        balance_before = requests.get(f"{BASE_URL}/api/wallets/{wallet_address}")
        balance_before_value = balance_before.json()["balance"]
        
        # Unstake 1000 JASPR
        unstake_response = requests.post(
            f"{BASE_URL}/api/staking/unstake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator1",
                "amount": 1000_000_000_000  # 1000 JASPR
            }
        )
        assert unstake_response.status_code == 200
        data = unstake_response.json()
        
        # Verify response includes unbonding period info
        assert data["success"] == True
        assert "unbonding_period_days" in data
        assert data["unbonding_period_days"] == 14
        assert "14 days" in data["message"]
        
        # Balance should NOT have increased (tokens are in unbonding)
        balance_after = requests.get(f"{BASE_URL}/api/wallets/{wallet_address}")
        balance_after_value = balance_after.json()["balance"]
        assert balance_after_value == balance_before_value
    
    def test_unbonding_entries_endpoint(self, test_wallet):
        """Test /api/staking/{address}/unbonding returns unbonding entries"""
        wallet_address = test_wallet["address"]
        
        # Stake and unstake to create unbonding entry
        requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator2",
                "amount": 1500_000_000_000
            }
        )
        
        requests.post(
            f"{BASE_URL}/api/staking/unstake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator2",
                "amount": 500_000_000_000
            }
        )
        
        # Get unbonding entries
        response = requests.get(f"{BASE_URL}/api/staking/{wallet_address}/unbonding")
        assert response.status_code == 200
        data = response.json()
        
        # Verify structure
        assert data["address"] == wallet_address
        assert "unbonding_entries" in data
        assert "total_unbonding" in data
        assert "claimable_amount" in data
        assert "unbonding_period_days" in data
        assert data["unbonding_period_days"] == 14
        
        # Should have at least one unbonding entry
        assert len(data["unbonding_entries"]) >= 1
        
        # Verify entry structure
        entry = data["unbonding_entries"][0]
        assert "delegator" in entry
        assert "validator" in entry
        assert "amount" in entry
        assert "unlock_time" in entry
        assert "remaining_days" in entry
        assert "is_claimable" in entry
    
    def test_unbonding_entry_shows_remaining_days(self, test_wallet):
        """Test that unbonding entries show remaining_days and is_claimable"""
        wallet_address = test_wallet["address"]
        
        # Stake and unstake
        requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator3",
                "amount": 1000_000_000_000
            }
        )
        
        requests.post(
            f"{BASE_URL}/api/staking/unstake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator3",
                "amount": 500_000_000_000
            }
        )
        
        # Get unbonding entries
        response = requests.get(f"{BASE_URL}/api/staking/{wallet_address}/unbonding")
        data = response.json()
        
        # Find the entry for validator3
        entry = None
        for e in data["unbonding_entries"]:
            if e["validator"] == "jaspr1validator3":
                entry = e
                break
        
        assert entry is not None
        
        # Remaining days should be close to 14 (just created)
        assert entry["remaining_days"] >= 13.9  # Allow small margin
        assert entry["remaining_days"] <= 14.0
        
        # Should not be claimable yet
        assert entry["is_claimable"] == False
    
    def test_claim_unbonded_no_claimable(self, test_wallet):
        """Test claim endpoint returns error when no tokens are claimable"""
        wallet_address = test_wallet["address"]
        
        # Try to claim without any unbonding entries
        response = requests.post(f"{BASE_URL}/api/staking/{wallet_address}/claim")
        assert response.status_code == 400
        assert "No tokens available to claim" in response.json()["detail"]
    
    def test_claim_unbonded_with_fresh_unbonding(self, test_wallet):
        """Test claim endpoint returns error when unbonding period not complete"""
        wallet_address = test_wallet["address"]
        
        # Stake and unstake to create unbonding entry
        requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator4",
                "amount": 1000_000_000_000
            }
        )
        
        requests.post(
            f"{BASE_URL}/api/staking/unstake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator4",
                "amount": 500_000_000_000
            }
        )
        
        # Try to claim immediately (should fail - 14 day period)
        response = requests.post(f"{BASE_URL}/api/staking/{wallet_address}/claim")
        assert response.status_code == 400
        assert "No tokens available to claim" in response.json()["detail"]
    
    def test_staking_info_includes_unbonding(self, test_wallet):
        """Test that staking info endpoint includes unbonding section"""
        wallet_address = test_wallet["address"]
        
        # Stake and unstake
        requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator1",
                "amount": 1000_000_000_000
            }
        )
        
        requests.post(
            f"{BASE_URL}/api/staking/unstake",
            json={
                "delegator": wallet_address,
                "validator": "jaspr1validator1",
                "amount": 300_000_000_000
            }
        )
        
        # Get staking info
        response = requests.get(f"{BASE_URL}/api/staking/{wallet_address}")
        assert response.status_code == 200
        data = response.json()
        
        # Verify unbonding section exists
        assert "unbonding" in data
        assert "entries" in data["unbonding"]
        assert "total_unbonding" in data["unbonding"]
        assert "claimable" in data["unbonding"]
        assert "unbonding_period_days" in data
        assert data["unbonding_period_days"] == 14
        
        # Should have unbonding entry
        assert data["unbonding"]["total_unbonding"] >= 300_000_000_000


class TestUnbondingMultipleEntries:
    """Tests for multiple unbonding entries"""
    
    @pytest.fixture
    def test_wallet(self):
        """Create a test wallet"""
        response = requests.post(f"{BASE_URL}/api/wallets/create")
        return response.json()
    
    def test_multiple_unstakes_create_multiple_entries(self, test_wallet):
        """Test that multiple unstakes create multiple unbonding entries"""
        wallet_address = test_wallet["address"]
        
        # Stake to multiple validators
        for validator in ["jaspr1validator1", "jaspr1validator2"]:
            requests.post(
                f"{BASE_URL}/api/staking/stake",
                json={
                    "delegator": wallet_address,
                    "validator": validator,
                    "amount": 1000_000_000_000
                }
            )
        
        # Unstake from both
        for validator in ["jaspr1validator1", "jaspr1validator2"]:
            requests.post(
                f"{BASE_URL}/api/staking/unstake",
                json={
                    "delegator": wallet_address,
                    "validator": validator,
                    "amount": 500_000_000_000
                }
            )
        
        # Get unbonding entries
        response = requests.get(f"{BASE_URL}/api/staking/{wallet_address}/unbonding")
        data = response.json()
        
        # Should have 2 unbonding entries
        assert len(data["unbonding_entries"]) >= 2
        
        # Total unbonding should be 1000 JASPR (500 + 500)
        assert data["total_unbonding"] >= 1000_000_000_000


class TestStakingStatsWithUnbonding:
    """Tests for staking stats including unbonding period info"""
    
    def test_staking_stats_shows_unbonding_period(self):
        """Test that staking stats overview shows unbonding period"""
        response = requests.get(f"{BASE_URL}/api/staking/stats/overview")
        assert response.status_code == 200
        data = response.json()
        
        assert "unbonding_period_days" in data
        assert data["unbonding_period_days"] == 14


class TestPreviousFeaturesStillWork:
    """Regression tests to ensure previous features still work"""
    
    def test_health_endpoint(self):
        """Test health endpoint still works"""
        response = requests.get(f"{BASE_URL}/api/health")
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "healthy"
        assert data["network"] == "testnet"
    
    def test_validators_endpoint(self):
        """Test validators endpoint still works"""
        response = requests.get(f"{BASE_URL}/api/validators")
        assert response.status_code == 200
        data = response.json()
        assert len(data["validators"]) == 4
    
    def test_wallet_creation(self):
        """Test wallet creation still works"""
        response = requests.post(f"{BASE_URL}/api/wallets/create")
        assert response.status_code == 200
        data = response.json()
        assert data["address"].startswith("jaspr1")
        assert data["threshold"] == "2-of-3"
    
    def test_sentinel_endpoint(self):
        """Test sentinel endpoint still works"""
        response = requests.get(f"{BASE_URL}/api/sentinel")
        assert response.status_code == 200
        data = response.json()
        assert "guard_mode" in data
    
    def test_tokenomics_endpoint(self):
        """Test tokenomics endpoint still works"""
        response = requests.get(f"{BASE_URL}/api/tokenomics")
        assert response.status_code == 200
        data = response.json()
        assert data["token"]["symbol"] == "JASPR"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
