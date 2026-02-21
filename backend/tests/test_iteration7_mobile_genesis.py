"""
Test iteration 7 features:
- Mobile responsive layout (2x2 grid on small screens)
- Genesis config API /api/genesis returns valid config
- Genesis has chain_id, validators, tokenomics
- Dashboard loads on mobile viewport
- Validators page loads on mobile
- Block height updates in real-time
- 20 validators showing
"""
import pytest
import requests
import os
import time

BASE_URL = os.environ.get('REACT_APP_BACKEND_URL', '').rstrip('/')

class TestGenesisAPI:
    """Test /api/genesis endpoint"""
    
    def test_genesis_endpoint_returns_200(self):
        """Genesis endpoint should return 200"""
        response = requests.get(f"{BASE_URL}/api/genesis")
        assert response.status_code == 200, f"Expected 200, got {response.status_code}"
        print("✓ Genesis endpoint returns 200")
    
    def test_genesis_has_chain_id(self):
        """Genesis should have chain_id"""
        response = requests.get(f"{BASE_URL}/api/genesis")
        data = response.json()
        assert "chain_id" in data, "Genesis missing chain_id"
        assert data["chain_id"] == "jasprchain-testnet-1", f"Unexpected chain_id: {data['chain_id']}"
        print(f"✓ Genesis has chain_id: {data['chain_id']}")
    
    def test_genesis_has_chain_name(self):
        """Genesis should have chain_name"""
        response = requests.get(f"{BASE_URL}/api/genesis")
        data = response.json()
        assert "chain_name" in data, "Genesis missing chain_name"
        assert data["chain_name"] == "JasprChain Testnet", f"Unexpected chain_name: {data['chain_name']}"
        print(f"✓ Genesis has chain_name: {data['chain_name']}")
    
    def test_genesis_has_validators(self):
        """Genesis should have validators array"""
        response = requests.get(f"{BASE_URL}/api/genesis")
        data = response.json()
        assert "validators" in data, "Genesis missing validators"
        assert isinstance(data["validators"], list), "validators should be a list"
        assert len(data["validators"]) >= 4, f"Expected at least 4 validators, got {len(data['validators'])}"
        print(f"✓ Genesis has {len(data['validators'])} validators")
    
    def test_genesis_validators_have_required_fields(self):
        """Genesis validators should have address, pub_key, power, name"""
        response = requests.get(f"{BASE_URL}/api/genesis")
        data = response.json()
        validators = data.get("validators", [])
        
        for v in validators:
            assert "address" in v, "Validator missing address"
            assert "pub_key" in v, "Validator missing pub_key"
            assert "power" in v, "Validator missing power"
            assert "name" in v, "Validator missing name"
        
        print(f"✓ All {len(validators)} genesis validators have required fields")
    
    def test_genesis_has_tokenomics(self):
        """Genesis should have tokenomics in app_state.bank"""
        response = requests.get(f"{BASE_URL}/api/genesis")
        data = response.json()
        
        assert "app_state" in data, "Genesis missing app_state"
        assert "bank" in data["app_state"], "Genesis missing app_state.bank"
        
        bank = data["app_state"]["bank"]
        assert "supply" in bank, "Genesis missing supply"
        assert "denom_metadata" in bank, "Genesis missing denom_metadata"
        assert "balances" in bank, "Genesis missing balances"
        
        # Check JASPR token metadata
        denom_metadata = bank["denom_metadata"]
        assert len(denom_metadata) > 0, "No denom metadata"
        jaspr_meta = denom_metadata[0]
        assert jaspr_meta["symbol"] == "JASPR", f"Expected JASPR symbol, got {jaspr_meta.get('symbol')}"
        
        print(f"✓ Genesis has tokenomics with JASPR token")
    
    def test_genesis_has_staking_params(self):
        """Genesis should have staking parameters"""
        response = requests.get(f"{BASE_URL}/api/genesis")
        data = response.json()
        
        assert "app_state" in data, "Genesis missing app_state"
        assert "staking" in data["app_state"], "Genesis missing app_state.staking"
        
        staking = data["app_state"]["staking"]
        assert "params" in staking, "Staking missing params"
        assert "validators" in staking, "Staking missing validators"
        
        params = staking["params"]
        assert "bond_denom" in params, "Staking params missing bond_denom"
        assert params["bond_denom"] == "ujaspr", f"Expected ujaspr, got {params['bond_denom']}"
        
        print(f"✓ Genesis has staking params with bond_denom: {params['bond_denom']}")
    
    def test_genesis_has_consensus_params(self):
        """Genesis should have consensus parameters"""
        response = requests.get(f"{BASE_URL}/api/genesis")
        data = response.json()
        
        assert "consensus_params" in data, "Genesis missing consensus_params"
        consensus = data["consensus_params"]
        
        assert "block" in consensus, "Consensus missing block params"
        assert "validator" in consensus, "Consensus missing validator params"
        
        # Check supported key types
        validator_params = consensus["validator"]
        assert "pub_key_types" in validator_params, "Missing pub_key_types"
        assert "ed25519" in validator_params["pub_key_types"], "ed25519 not in pub_key_types"
        
        print(f"✓ Genesis has consensus params with pub_key_types: {validator_params['pub_key_types']}")
    
    def test_genesis_has_sentinel_params(self):
        """Genesis should have AI Sentinel parameters"""
        response = requests.get(f"{BASE_URL}/api/genesis")
        data = response.json()
        
        assert "app_state" in data, "Genesis missing app_state"
        assert "sentinel" in data["app_state"], "Genesis missing app_state.sentinel"
        
        sentinel = data["app_state"]["sentinel"]
        assert "params" in sentinel, "Sentinel missing params"
        
        params = sentinel["params"]
        assert "guard_mode" in params, "Sentinel params missing guard_mode"
        assert params["guard_mode"] == "ENFORCED", f"Expected ENFORCED, got {params['guard_mode']}"
        
        print(f"✓ Genesis has sentinel params with guard_mode: {params['guard_mode']}")
    
    def test_genesis_has_move_vm_params(self):
        """Genesis should have Move VM parameters"""
        response = requests.get(f"{BASE_URL}/api/genesis")
        data = response.json()
        
        assert "app_state" in data, "Genesis missing app_state"
        assert "move" in data["app_state"], "Genesis missing app_state.move"
        
        move = data["app_state"]["move"]
        assert "params" in move, "Move missing params"
        assert "modules" in move, "Move missing modules"
        
        # Check stdlib modules
        modules = move["modules"]
        module_names = [m["name"] for m in modules]
        assert "JASPR" in module_names, "JASPR module not in genesis"
        assert "Coin" in module_names, "Coin module not in genesis"
        assert "Account" in module_names, "Account module not in genesis"
        
        print(f"✓ Genesis has Move VM with stdlib modules: {module_names}")


class TestValidators20:
    """Test 20 validators are present"""
    
    def test_validators_count_is_20(self):
        """Should have 20 validators"""
        response = requests.get(f"{BASE_URL}/api/validators")
        data = response.json()
        
        assert data.get("total_validators") == 20, f"Expected 20 validators, got {data.get('total_validators')}"
        print(f"✓ Total validators: {data['total_validators']}")
    
    def test_all_validators_active(self):
        """All 20 validators should be active"""
        response = requests.get(f"{BASE_URL}/api/validators")
        data = response.json()
        
        assert data.get("active_validators") == 20, f"Expected 20 active, got {data.get('active_validators')}"
        print(f"✓ Active validators: {data['active_validators']}")
    
    def test_validators_have_stake(self):
        """All validators should have stake"""
        response = requests.get(f"{BASE_URL}/api/validators")
        data = response.json()
        
        validators = data.get("validators", [])
        for v in validators:
            assert v.get("stake", 0) > 0, f"Validator {v.get('name')} has no stake"
        
        print(f"✓ All {len(validators)} validators have stake")
    
    def test_validators_have_names(self):
        """All validators should have names"""
        response = requests.get(f"{BASE_URL}/api/validators")
        data = response.json()
        
        validators = data.get("validators", [])
        names = [v.get("name") for v in validators]
        
        # Check for expected names
        expected_names = ["Jaspr Labs", "Foundation", "Community", "Ecosystem", 
                         "Alpha Node", "Beta Node", "Gamma Node", "Delta Node"]
        
        for name in expected_names:
            assert name in names, f"Expected validator '{name}' not found"
        
        print(f"✓ Validators have expected names: {names[:5]}...")


class TestBlockHeightRealTime:
    """Test block height updates in real-time"""
    
    def test_block_height_increases(self):
        """Block height should increase over time"""
        response1 = requests.get(f"{BASE_URL}/api/health")
        height1 = response1.json().get("height", 0)
        
        # Wait 3 seconds (blocks produced every 2 seconds)
        time.sleep(3)
        
        response2 = requests.get(f"{BASE_URL}/api/health")
        height2 = response2.json().get("height", 0)
        
        assert height2 > height1, f"Height did not increase: {height1} -> {height2}"
        print(f"✓ Block height increased: {height1} -> {height2}")
    
    def test_blocks_endpoint_shows_recent_blocks(self):
        """Blocks endpoint should show recent blocks"""
        response = requests.get(f"{BASE_URL}/api/blocks?limit=5")
        data = response.json()
        
        assert "blocks" in data, "Missing blocks array"
        blocks = data["blocks"]
        assert len(blocks) >= 1, "No blocks returned"
        
        # Check blocks are in descending order
        heights = [b["height"] for b in blocks]
        assert heights == sorted(heights, reverse=True), "Blocks not in descending order"
        
        print(f"✓ Recent blocks: heights {heights}")
    
    def test_network_stats_has_height(self):
        """Network stats should have current height"""
        response = requests.get(f"{BASE_URL}/api/network/stats")
        data = response.json()
        
        assert "height" in data, "Network stats missing height"
        assert data["height"] > 0, f"Height should be > 0, got {data['height']}"
        
        print(f"✓ Network stats height: {data['height']}")


class TestRegressionAPIs:
    """Regression tests for existing APIs"""
    
    def test_health_endpoint(self):
        """Health endpoint should work"""
        response = requests.get(f"{BASE_URL}/api/health")
        assert response.status_code == 200
        data = response.json()
        assert data.get("status") == "healthy"
        assert data.get("network") == "testnet"
        print("✓ Health endpoint working")
    
    def test_tokenomics_endpoint(self):
        """Tokenomics endpoint should work"""
        response = requests.get(f"{BASE_URL}/api/tokenomics")
        assert response.status_code == 200
        data = response.json()
        assert "token" in data
        print("✓ Tokenomics endpoint working")
    
    def test_sentinel_endpoint(self):
        """Sentinel endpoint should work"""
        response = requests.get(f"{BASE_URL}/api/sentinel")
        assert response.status_code == 200
        data = response.json()
        assert "guard_mode" in data
        print("✓ Sentinel endpoint working")
    
    def test_mempool_endpoint(self):
        """Mempool endpoint should work"""
        response = requests.get(f"{BASE_URL}/api/mempool")
        assert response.status_code == 200
        data = response.json()
        assert "stats" in data
        print("✓ Mempool endpoint working")
    
    def test_staking_stats_endpoint(self):
        """Staking stats endpoint should work"""
        response = requests.get(f"{BASE_URL}/api/staking/stats/overview")
        assert response.status_code == 200
        data = response.json()
        assert "total_staked" in data
        assert "active_validators" in data
        print("✓ Staking stats endpoint working")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
