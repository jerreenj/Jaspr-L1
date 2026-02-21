#!/usr/bin/env python3
"""
JasprChain Backend API Testing Suite
Tests all blockchain APIs for functionality and integration
"""
import requests
import json
import sys
from datetime import datetime
from typing import Dict, Any, Optional

# Use external URL for testing
BASE_URL = "https://blockchain-proto.preview.emergentagent.com/api"

class JasprChainTester:
    def __init__(self):
        self.base_url = BASE_URL
        self.session = requests.Session()
        self.session.headers.update({'Content-Type': 'application/json'})
        
        # Test data storage
        self.created_wallet = None
        self.test_results = {
            'passed': [],
            'failed': [],
            'total': 0
        }
    
    def log_test(self, name: str, success: bool, details: str = ""):
        """Log test result"""
        self.test_results['total'] += 1
        if success:
            self.test_results['passed'].append(name)
            print(f"✅ {name}: PASSED {details}")
        else:
            self.test_results['failed'].append(name)
            print(f"❌ {name}: FAILED {details}")
        return success
    
    def make_request(self, method: str, endpoint: str, data: Dict = None) -> tuple:
        """Make HTTP request and return (success, response_data, status_code)"""
        url = f"{self.base_url}/{endpoint.lstrip('/')}"
        
        try:
            if method.upper() == 'GET':
                response = self.session.get(url)
            elif method.upper() == 'POST':
                response = self.session.post(url, json=data)
            else:
                return False, {}, 0
            
            try:
                response_data = response.json()
            except:
                response_data = {"text": response.text}
            
            return response.status_code < 400, response_data, response.status_code
        
        except Exception as e:
            return False, {"error": str(e)}, 0
    
    def test_health_endpoint(self):
        """Test /api/health endpoint"""
        success, data, status = self.make_request('GET', '/health')
        
        if success and data.get('status') == 'healthy':
            return self.log_test("Health Check", True, f"Chain ID: {data.get('chain_id')}, Height: {data.get('height')}")
        else:
            return self.log_test("Health Check", False, f"Status: {status}, Data: {data}")
    
    def test_network_stats(self):
        """Test /api/network/stats endpoint"""
        success, data, status = self.make_request('GET', '/network/stats')
        
        required_fields = ['chain_id', 'height', 'validators', 'mempool', 'sentinel']
        if success and all(field in data for field in required_fields):
            validators = data.get('validators', {})
            if validators.get('total') == 4:  # Should have 4 validators
                return self.log_test("Network Stats", True, f"Height: {data.get('height')}, Validators: {validators.get('total')}")
            else:
                return self.log_test("Network Stats", False, f"Expected 4 validators, got {validators.get('total')}")
        else:
            return self.log_test("Network Stats", False, f"Status: {status}, Missing fields or error")
    
    def test_blocks_endpoint(self):
        """Test /api/blocks endpoint"""
        success, data, status = self.make_request('GET', '/blocks')
        
        if success and 'blocks' in data:
            blocks = data.get('blocks', [])
            if len(blocks) > 0:
                # Check if genesis block exists
                genesis_found = any(block.get('height') == 0 for block in blocks)
                return self.log_test("Blocks Endpoint", True, f"Found {len(blocks)} blocks, Genesis: {genesis_found}")
            else:
                return self.log_test("Blocks Endpoint", False, "No blocks found")
        else:
            return self.log_test("Blocks Endpoint", False, f"Status: {status}, Data: {data}")
    
    def test_validators_endpoint(self):
        """Test /api/validators endpoint"""
        success, data, status = self.make_request('GET', '/validators')
        
        if success and 'validators' in data:
            validators = data.get('validators', [])
            expected_names = ['Jaspr Labs', 'Foundation', 'Community', 'Ecosystem']
            
            if len(validators) == 4:
                validator_names = [v.get('name', '') for v in validators]
                names_match = all(name in validator_names for name in expected_names)
                return self.log_test("Validators Endpoint", names_match, 
                                   f"Found {len(validators)} validators: {validator_names}")
            else:
                return self.log_test("Validators Endpoint", False, f"Expected 4 validators, got {len(validators)}")
        else:
            return self.log_test("Validators Endpoint", False, f"Status: {status}, Data: {data}")
    
    def test_wallet_creation(self):
        """Test POST /api/wallets/create endpoint"""
        success, data, status = self.make_request('POST', '/wallets/create')
        
        if success and 'address' in data and 'public_key' in data:
            self.created_wallet = data
            return self.log_test("Wallet Creation", True, f"Address: {data.get('address')[:12]}...")
        else:
            return self.log_test("Wallet Creation", False, f"Status: {status}, Data: {data}")
    
    def test_wallet_details(self):
        """Test GET /api/wallets/{address} endpoint"""
        if not self.created_wallet:
            return self.log_test("Wallet Details", False, "No wallet created to test")
        
        address = self.created_wallet.get('address')
        success, data, status = self.make_request('GET', f'/wallets/{address}')
        
        if success and 'balance' in data and 'address' in data:
            balance = data.get('balance', 0)
            return self.log_test("Wallet Details", True, f"Balance: {balance} ({data.get('balance_formatted', 'N/A')})")
        else:
            return self.log_test("Wallet Details", False, f"Status: {status}, Data: {data}")
    
    def test_transaction_transfer(self):
        """Test POST /api/transactions/transfer endpoint"""
        if not self.created_wallet:
            return self.log_test("Transaction Transfer", False, "No wallet created to test")
        
        # Create a second wallet for transfer
        success, recipient_data, _ = self.make_request('POST', '/wallets/create')
        if not success:
            return self.log_test("Transaction Transfer", False, "Failed to create recipient wallet")
        
        transfer_data = {
            "sender": self.created_wallet.get('address'),
            "recipient": recipient_data.get('address'),
            "amount": 1000000000  # 1 JJ
        }
        
        success, data, status = self.make_request('POST', '/transactions/transfer', transfer_data)
        
        if success and data.get('success') and 'tx_hash' in data:
            return self.log_test("Transaction Transfer", True, f"TX Hash: {data.get('tx_hash')[:12]}...")
        else:
            return self.log_test("Transaction Transfer", False, f"Status: {status}, Data: {data}")
    
    def test_sentinel_endpoint(self):
        """Test /api/sentinel endpoint"""
        success, data, status = self.make_request('GET', '/sentinel')
        
        if success and 'guard_mode' in data and 'stats' in data:
            guard_mode = data.get('guard_mode')
            stats = data.get('stats', {})
            return self.log_test("AI Sentinel", True, f"Mode: {guard_mode}, Scanned: {stats.get('total_scanned', 0)}")
        else:
            return self.log_test("AI Sentinel", False, f"Status: {status}, Data: {data}")
    
    def test_dex_markets(self):
        """Test /api/dex/markets endpoint"""
        success, data, status = self.make_request('GET', '/dex/markets')
        
        if success and 'markets' in data:
            markets = data.get('markets', [])
            jj_usdc_found = any('JJ/USDC' in str(market) for market in markets)
            return self.log_test("DEX Markets", jj_usdc_found, f"Found {len(markets)} markets")
        else:
            return self.log_test("DEX Markets", False, f"Status: {status}, Data: {data}")
    
    def test_dex_orderbook(self):
        """Test /api/dex/orderbook/JJ%2FUSDC endpoint"""
        success, data, status = self.make_request('GET', '/dex/orderbook/JJ%2FUSDC')
        
        if success and ('bids' in data or 'asks' in data or 'market' in data):
            return self.log_test("DEX Orderbook", True, "Orderbook data retrieved")
        else:
            return self.log_test("DEX Orderbook", False, f"Status: {status}, Data: {data}")
    
    def test_dex_order_placement(self):
        """Test POST /api/dex/order endpoint"""
        if not self.created_wallet:
            return self.log_test("DEX Order Placement", False, "No wallet created to test")
        
        order_data = {
            "trader": self.created_wallet.get('address'),
            "market": "JJ/USDC",
            "side": "buy",
            "order_type": "limit",
            "price": 1.0,
            "quantity": 100.0
        }
        
        success, data, status = self.make_request('POST', '/dex/order', order_data)
        
        if success and ('order' in data or 'message' in data):
            return self.log_test("DEX Order Placement", True, f"Order response received")
        else:
            return self.log_test("DEX Order Placement", False, f"Status: {status}, Data: {data}")
    
    def test_mempool_endpoint(self):
        """Test /api/mempool endpoint"""
        success, data, status = self.make_request('GET', '/mempool')
        
        if success and 'stats' in data:
            stats = data.get('stats', {})
            return self.log_test("Mempool Stats", True, f"Pending: {stats.get('total_pending', 0)}")
        else:
            return self.log_test("Mempool Stats", False, f"Status: {status}, Data: {data}")
    
    def run_all_tests(self):
        """Run all API tests"""
        print(f"🚀 Starting JasprChain API Tests")
        print(f"📡 Testing against: {self.base_url}")
        print("=" * 60)
        
        # Core endpoints
        self.test_health_endpoint()
        self.test_network_stats()
        self.test_blocks_endpoint()
        self.test_validators_endpoint()
        
        # Wallet operations
        self.test_wallet_creation()
        self.test_wallet_details()
        self.test_transaction_transfer()
        
        # AI Sentinel
        self.test_sentinel_endpoint()
        
        # DEX operations
        self.test_dex_markets()
        self.test_dex_orderbook()
        self.test_dex_order_placement()
        
        # Mempool
        self.test_mempool_endpoint()
        
        # Summary
        print("=" * 60)
        print(f"📊 Test Results:")
        print(f"✅ Passed: {len(self.test_results['passed'])}/{self.test_results['total']}")
        print(f"❌ Failed: {len(self.test_results['failed'])}/{self.test_results['total']}")
        
        if self.test_results['failed']:
            print(f"\n❌ Failed Tests:")
            for test in self.test_results['failed']:
                print(f"  - {test}")
        
        success_rate = len(self.test_results['passed']) / max(1, self.test_results['total']) * 100
        print(f"\n📈 Success Rate: {success_rate:.1f}%")
        
        return success_rate >= 80  # Consider 80%+ as passing


if __name__ == "__main__":
    tester = JasprChainTester()
    success = tester.run_all_tests()
    sys.exit(0 if success else 1)