"""
JasprChain Slashing, P2P, and Move VM Tests
Tests for the three new major features:
1. Slashing - penalties for validator misbehavior
2. P2P - async networking with gossip protocol
3. Move VM - smart contract execution with stdlib

MOCKED: Move VM bytecode execution is simulated, P2P not started (simulation mode)
"""
import pytest
import requests
import os

BASE_URL = os.environ.get('REACT_APP_BACKEND_URL', '').rstrip('/')


class TestSlashingAPI:
    """Tests for Slashing mechanism API"""
    
    def test_slashing_stats_endpoint(self):
        """Test /api/slashing/stats returns slashing statistics"""
        response = requests.get(f"{BASE_URL}/api/slashing/stats")
        assert response.status_code == 200
        data = response.json()
        
        # Verify stats structure
        assert "total_slashed_amount" in data
        assert "double_sign_events" in data
        assert "downtime_events" in data
        assert "invalid_attestation_events" in data
        assert "validators_jailed" in data
        assert "validators_tombstoned" in data
        assert "pending_evidence" in data
        assert "total_records" in data
        
        # Initial state should have 0 events
        assert data["total_slashed_amount"] == 0
        assert data["double_sign_events"] == 0
        assert data["downtime_events"] == 0
    
    def test_slashing_history_endpoint(self):
        """Test /api/slashing/history returns slashing records"""
        response = requests.get(f"{BASE_URL}/api/slashing/history")
        assert response.status_code == 200
        data = response.json()
        
        # Verify structure
        assert "records" in data
        assert "total" in data
        assert isinstance(data["records"], list)
    
    def test_slashing_history_with_validator_filter(self):
        """Test /api/slashing/history with validator filter"""
        response = requests.get(f"{BASE_URL}/api/slashing/history?validator=jaspr1validator1")
        assert response.status_code == 200
        data = response.json()
        
        # Should return empty list for validator with no slashing events
        assert "records" in data
        assert isinstance(data["records"], list)
    
    def test_slashing_validator_signing_info_not_registered(self):
        """Test /api/slashing/validator/{address} returns 404 for unregistered validator"""
        # Validators need to be registered with slashing module first
        response = requests.get(f"{BASE_URL}/api/slashing/validator/jaspr1validator1")
        # Returns 404 because validator not registered with slashing module yet
        assert response.status_code == 404
        assert "Validator not found" in response.json()["detail"]


class TestP2PAPI:
    """Tests for P2P networking API"""
    
    def test_p2p_info_endpoint(self):
        """Test /api/p2p/info returns node information"""
        response = requests.get(f"{BASE_URL}/api/p2p/info")
        assert response.status_code == 200
        data = response.json()
        
        # Verify structure
        assert "node_id" in data
        assert "version" in data
        assert "is_running" in data
        assert "connected_peers" in data
        assert "chain_height" in data
        
        # Verify values
        assert data["version"] == "0.1.0"
        assert data["is_running"] == False  # Simulation mode - not started
        assert data["connected_peers"] == 0
        assert len(data["node_id"]) == 64  # SHA256 hash length
    
    def test_p2p_peers_endpoint(self):
        """Test /api/p2p/peers returns peer list"""
        response = requests.get(f"{BASE_URL}/api/p2p/peers")
        assert response.status_code == 200
        data = response.json()
        
        # Verify structure
        assert "peers" in data
        assert "count" in data
        assert isinstance(data["peers"], list)
        
        # No peers in simulation mode
        assert data["count"] == 0
    
    def test_p2p_stats_endpoint(self):
        """Test /api/p2p/stats returns P2P statistics"""
        response = requests.get(f"{BASE_URL}/api/p2p/stats")
        assert response.status_code == 200
        data = response.json()
        
        # Verify stats structure
        assert "messages_sent" in data
        assert "messages_received" in data
        assert "blocks_propagated" in data
        assert "transactions_propagated" in data
        assert "bytes_sent" in data
        assert "bytes_received" in data
        assert "peer_connections" in data
        assert "peer_disconnections" in data
        assert "connected_peers" in data
        assert "is_running" in data
        
        # Simulation mode - no activity
        assert data["is_running"] == False


class TestMoveVMAPI:
    """Tests for Move VM smart contract API"""
    
    def test_move_modules_list_stdlib(self):
        """Test /api/move/modules lists stdlib modules (JASPR, Coin, Account)"""
        response = requests.get(f"{BASE_URL}/api/move/modules")
        assert response.status_code == 200
        data = response.json()
        
        # Verify structure
        assert "modules" in data
        assert "total" in data
        assert data["total"] >= 3  # At least stdlib modules
        
        # Verify stdlib modules exist
        module_ids = [m["module_id"] for m in data["modules"]]
        assert "0x1::JASPR" in module_ids
        assert "0x1::Coin" in module_ids
        assert "0x1::Account" in module_ids
    
    def test_move_module_jaspr_details(self):
        """Test /api/move/module/{module_id} returns JASPR module details"""
        response = requests.get(f"{BASE_URL}/api/move/module/0x1::JASPR")
        assert response.status_code == 200
        data = response.json()
        
        # Verify structure
        assert data["address"] == "0x1"
        assert data["name"] == "JASPR"
        assert data["module_id"] == "0x1::JASPR"
        assert "abi" in data
        
        # Verify JASPR module has transfer function
        function_names = [f["name"] for f in data["abi"]["functions"]]
        assert "transfer" in function_names
        assert "mint" in function_names
        assert "burn" in function_names
    
    def test_move_module_coin_details(self):
        """Test /api/move/module/{module_id} returns Coin module details"""
        response = requests.get(f"{BASE_URL}/api/move/module/0x1::Coin")
        assert response.status_code == 200
        data = response.json()
        
        assert data["module_id"] == "0x1::Coin"
        assert "abi" in data
        
        # Verify Coin module has expected structs
        struct_names = [s["name"] for s in data["abi"]["structs"]]
        assert "Coin" in struct_names
        assert "CoinStore" in struct_names
    
    def test_move_module_account_details(self):
        """Test /api/move/module/{module_id} returns Account module details"""
        response = requests.get(f"{BASE_URL}/api/move/module/0x1::Account")
        assert response.status_code == 200
        data = response.json()
        
        assert data["module_id"] == "0x1::Account"
        
        # Verify Account module has expected functions
        function_names = [f["name"] for f in data["abi"]["functions"]]
        assert "create_account" in function_names
    
    def test_move_module_not_found(self):
        """Test /api/move/module/{module_id} returns 404 for non-existent module"""
        response = requests.get(f"{BASE_URL}/api/move/module/0x1::NonExistent")
        assert response.status_code == 404
        assert "Module not found" in response.json()["detail"]
    
    def test_move_execute_jaspr_transfer(self):
        """Test /api/move/execute executes JASPR transfer function"""
        response = requests.post(
            f"{BASE_URL}/api/move/execute",
            json={
                "sender": "jaspr1sender_test",
                "module_id": "0x1::JASPR",
                "function_name": "transfer",
                "type_args": [],
                "args": ["jaspr1recipient_test", 1000000000]  # 1 JASPR
            }
        )
        assert response.status_code == 200
        data = response.json()
        
        # Verify execution result
        assert data["success"] == True
        assert "gas_used" in data
        assert data["gas_used"] > 0
        assert "events" in data
        assert len(data["events"]) > 0
        
        # Verify transfer event
        transfer_event = data["events"][0]
        assert transfer_event["type"] == "JASPRTransfer"
        assert transfer_event["from"] == "jaspr1sender_test"
        assert transfer_event["to"] == "jaspr1recipient_test"
        assert transfer_event["amount"] == 1000000000
    
    def test_move_execute_coin_transfer(self):
        """Test /api/move/execute executes Coin transfer function"""
        response = requests.post(
            f"{BASE_URL}/api/move/execute",
            json={
                "sender": "jaspr1coin_sender",
                "module_id": "0x1::Coin",
                "function_name": "transfer",
                "type_args": ["JASPR"],
                "args": ["jaspr1coin_recipient", 500000000]
            }
        )
        assert response.status_code == 200
        data = response.json()
        
        assert data["success"] == True
        assert "events" in data
        
        # Verify coin transfer event
        transfer_event = data["events"][0]
        assert transfer_event["type"] == "CoinTransfer"
    
    def test_move_execute_account_create(self):
        """Test /api/move/execute executes Account create_account function"""
        response = requests.post(
            f"{BASE_URL}/api/move/execute",
            json={
                "sender": "jaspr1account_creator",
                "module_id": "0x1::Account",
                "function_name": "create_account",
                "type_args": [],
                "args": ["jaspr1new_account"]
            }
        )
        assert response.status_code == 200
        data = response.json()
        
        assert data["success"] == True
        
        # Verify account created event
        event = data["events"][0]
        assert event["type"] == "AccountCreated"
    
    def test_move_execute_function_not_found(self):
        """Test /api/move/execute returns error for non-existent function"""
        response = requests.post(
            f"{BASE_URL}/api/move/execute",
            json={
                "sender": "jaspr1test",
                "module_id": "0x1::JASPR",
                "function_name": "non_existent_function",
                "type_args": [],
                "args": []
            }
        )
        assert response.status_code == 400
        assert "not found" in response.json()["detail"]
    
    def test_move_execute_module_not_found(self):
        """Test /api/move/execute returns error for non-existent module"""
        response = requests.post(
            f"{BASE_URL}/api/move/execute",
            json={
                "sender": "jaspr1test",
                "module_id": "0x1::NonExistent",
                "function_name": "test",
                "type_args": [],
                "args": []
            }
        )
        assert response.status_code == 400
        assert "not found" in response.json()["detail"]
    
    def test_move_deploy_custom_module(self):
        """Test /api/move/deploy can deploy custom modules"""
        import time
        unique_name = f"CustomModule_{int(time.time())}"
        
        response = requests.post(
            f"{BASE_URL}/api/move/deploy",
            json={
                "sender": "jaspr1custom_deployer",
                "name": unique_name,
                "bytecode": "0xabcdef1234567890",
                "abi": {
                    "structs": [
                        {"name": "CustomStruct", "abilities": ["store"], "fields": []}
                    ],
                    "functions": [
                        {"name": "custom_function", "visibility": "public", "is_entry": True}
                    ]
                }
            }
        )
        assert response.status_code == 200
        data = response.json()
        
        # Verify deployment result
        assert data["success"] == True
        assert "gas_used" in data
        assert data["gas_used"] > 0
        assert "events" in data
        
        # Verify module deployed event
        deploy_event = data["events"][0]
        assert deploy_event["type"] == "ModuleDeployed"
        assert deploy_event["deployer"] == "jaspr1custom_deployer"
        
        # Verify module_id in return values
        assert f"jaspr1custom_deployer::{unique_name}" in data["return_values"]
        
        # Verify module is now listed
        modules_response = requests.get(f"{BASE_URL}/api/move/modules")
        module_ids = [m["module_id"] for m in modules_response.json()["modules"]]
        assert f"jaspr1custom_deployer::{unique_name}" in module_ids
    
    def test_move_deploy_duplicate_module_fails(self):
        """Test /api/move/deploy fails for duplicate module"""
        # First deployment
        response1 = requests.post(
            f"{BASE_URL}/api/move/deploy",
            json={
                "sender": "jaspr1dup_deployer",
                "name": "DuplicateModule",
                "bytecode": "0x1234",
                "abi": {"structs": [], "functions": []}
            }
        )
        # May succeed or fail if already exists from previous test run
        
        # Second deployment should fail
        response2 = requests.post(
            f"{BASE_URL}/api/move/deploy",
            json={
                "sender": "jaspr1dup_deployer",
                "name": "DuplicateModule",
                "bytecode": "0x5678",
                "abi": {"structs": [], "functions": []}
            }
        )
        assert response2.status_code == 400
        assert "already exists" in response2.json()["detail"]
    
    def test_move_estimate_gas(self):
        """Test /api/move/estimate-gas returns gas estimate"""
        response = requests.get(
            f"{BASE_URL}/api/move/estimate-gas",
            params={
                "module_id": "0x1::JASPR",
                "function_name": "transfer",
                "type_args": "[]",
                "args": '["jaspr1recipient", 1000]'
            }
        )
        assert response.status_code == 200
        data = response.json()
        
        assert "estimated_gas" in data
        assert "gas_unit_price" in data
        assert data["estimated_gas"] > 0
        assert data["gas_unit_price"] == 100  # nanoJASPR per gas unit
    
    def test_move_stats_endpoint(self):
        """Test /api/move/stats returns VM statistics"""
        response = requests.get(f"{BASE_URL}/api/move/stats")
        assert response.status_code == 200
        data = response.json()
        
        # Verify stats structure
        assert "modules_deployed" in data
        assert "functions_called" in data
        assert "total_gas_used" in data
        assert "events_emitted" in data
        assert "total_modules" in data
        assert "total_resources" in data
        assert "stdlib_modules" in data
        
        # Verify stdlib modules listed
        assert "0x1::JASPR" in data["stdlib_modules"]
        assert "0x1::Coin" in data["stdlib_modules"]
        assert "0x1::Account" in data["stdlib_modules"]


class TestPreviousFeaturesRegression:
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
    
    def test_staking_flow(self):
        """Test staking flow still works"""
        # Create wallet
        wallet_response = requests.post(f"{BASE_URL}/api/wallets/create")
        wallet = wallet_response.json()
        
        # Stake
        stake_response = requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet["address"],
                "validator": "jaspr1validator1",
                "amount": 1000_000_000_000
            }
        )
        assert stake_response.status_code == 200
        assert stake_response.json()["success"] == True
    
    def test_persistence_endpoint(self):
        """Test persistence endpoint still works"""
        response = requests.get(f"{BASE_URL}/api/network/persistence")
        assert response.status_code == 200
        data = response.json()
        assert data["persistence"] == "lmdb"
    
    def test_unbonding_flow(self):
        """Test unbonding flow still works"""
        # Create wallet
        wallet_response = requests.post(f"{BASE_URL}/api/wallets/create")
        wallet = wallet_response.json()
        
        # Stake
        requests.post(
            f"{BASE_URL}/api/staking/stake",
            json={
                "delegator": wallet["address"],
                "validator": "jaspr1validator2",
                "amount": 2000_000_000_000
            }
        )
        
        # Unstake (creates unbonding entry)
        unstake_response = requests.post(
            f"{BASE_URL}/api/staking/unstake",
            json={
                "delegator": wallet["address"],
                "validator": "jaspr1validator2",
                "amount": 1000_000_000_000
            }
        )
        assert unstake_response.status_code == 200
        assert unstake_response.json()["unbonding_period_days"] == 14
        
        # Check unbonding entries
        unbonding_response = requests.get(f"{BASE_URL}/api/staking/{wallet['address']}/unbonding")
        assert unbonding_response.status_code == 200
        assert unbonding_response.json()["total_unbonding"] >= 1000_000_000_000
    
    def test_block_production(self):
        """Test block production continues"""
        import time
        
        # Get current height
        response1 = requests.get(f"{BASE_URL}/api/health")
        height1 = response1.json()["height"]
        
        # Wait for blocks
        time.sleep(3)
        
        # Get new height
        response2 = requests.get(f"{BASE_URL}/api/health")
        height2 = response2.json()["height"]
        
        # Should have produced at least 1 block
        assert height2 > height1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
