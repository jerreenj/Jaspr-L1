"""Move VM Integration for JasprChain
Simulates Move smart contract execution

Features:
- Module deployment and storage
- Script execution
- Resource management
- Gas metering
"""
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, Set
from datetime import datetime, timezone
import hashlib
import json


@dataclass
class MoveModule:
    """Represents a deployed Move module"""
    address: str  # Deployer address
    name: str     # Module name
    bytecode: str # Compiled bytecode (hex string)
    abi: Dict     # Module ABI (functions, structs, etc.)
    deployed_at: int
    tx_hash: str
    
    @property
    def module_id(self) -> str:
        return f"{self.address}::{self.name}"
    
    def to_dict(self) -> dict:
        return {
            "address": self.address,
            "name": self.name,
            "module_id": self.module_id,
            "bytecode_size": len(self.bytecode) // 2,
            "abi": self.abi,
            "deployed_at": self.deployed_at,
            "tx_hash": self.tx_hash
        }


@dataclass
class MoveResource:
    """Represents a resource stored in an account"""
    owner: str
    resource_type: str  # e.g., "0x1::Coin::Coin<0x1::JASPR::JASPR>"
    data: Dict
    
    def to_dict(self) -> dict:
        return {
            "owner": self.owner,
            "resource_type": self.resource_type,
            "data": self.data
        }


@dataclass
class MoveFunction:
    """Represents a Move function"""
    module_id: str
    name: str
    visibility: str  # "public", "public(friend)", "private"
    is_entry: bool
    type_params: List[str]
    params: List[Dict]
    returns: List[Dict]
    
    def to_dict(self) -> dict:
        return {
            "module_id": self.module_id,
            "name": self.name,
            "visibility": self.visibility,
            "is_entry": self.is_entry,
            "type_params": self.type_params,
            "params": self.params,
            "returns": self.returns
        }


@dataclass
class ExecutionResult:
    """Result of Move script/function execution"""
    success: bool
    gas_used: int
    return_values: List[Any]
    events: List[Dict]
    error: Optional[str] = None
    changes: List[Dict] = field(default_factory=list)
    
    def to_dict(self) -> dict:
        return {
            "success": self.success,
            "gas_used": self.gas_used,
            "return_values": self.return_values,
            "events": self.events,
            "error": self.error,
            "changes": self.changes
        }


class MoveVM:
    """Move Virtual Machine simulation for JasprChain
    
    This is a simulation layer that mimics Move VM behavior.
    In production, this would be replaced with actual move-vm-runtime.
    """
    
    # Gas costs
    GAS_UNIT_PRICE = 100  # nanoJASPR per gas unit
    MIN_GAS = 1000
    MAX_GAS = 10_000_000
    
    # Operation gas costs
    GAS_COSTS = {
        "deploy_module": 50000,
        "call_function": 1000,
        "read_resource": 100,
        "write_resource": 500,
        "emit_event": 200,
        "type_check": 50,
        "bytecode_per_byte": 10
    }
    
    def __init__(self, persistence=None):
        self.persistence = persistence
        
        # Module storage: module_id -> MoveModule
        self.modules: Dict[str, MoveModule] = {}
        
        # Resource storage: owner::resource_type -> MoveResource
        self.resources: Dict[str, MoveResource] = {}
        
        # Execution context
        self._current_sender: Optional[str] = None
        self._gas_remaining: int = 0
        self._events: List[Dict] = []
        self._changes: List[Dict] = []
        
        # Statistics
        self.stats = {
            "modules_deployed": 0,
            "functions_called": 0,
            "total_gas_used": 0,
            "events_emitted": 0
        }
        
        # Initialize standard library modules
        self._init_stdlib()
    
    def _init_stdlib(self):
        """Initialize standard library modules"""
        # JASPR Token module
        jaspr_module = MoveModule(
            address="0x1",
            name="JASPR",
            bytecode="",  # Built-in
            abi={
                "structs": [
                    {
                        "name": "JASPR",
                        "abilities": ["store"],
                        "fields": []
                    }
                ],
                "functions": [
                    {"name": "initialize", "visibility": "public", "is_entry": True},
                    {"name": "mint", "visibility": "public", "is_entry": True},
                    {"name": "burn", "visibility": "public", "is_entry": True},
                    {"name": "transfer", "visibility": "public", "is_entry": True}
                ]
            },
            deployed_at=0,
            tx_hash="genesis"
        )
        self.modules[jaspr_module.module_id] = jaspr_module
        
        # Coin module (generic)
        coin_module = MoveModule(
            address="0x1",
            name="Coin",
            bytecode="",
            abi={
                "structs": [
                    {
                        "name": "Coin",
                        "type_params": ["CoinType"],
                        "abilities": ["store"],
                        "fields": [{"name": "value", "type": "u64"}]
                    },
                    {
                        "name": "CoinStore",
                        "type_params": ["CoinType"],
                        "abilities": ["key"],
                        "fields": [
                            {"name": "coin", "type": "Coin<CoinType>"},
                            {"name": "frozen", "type": "bool"}
                        ]
                    }
                ],
                "functions": [
                    {"name": "balance", "visibility": "public", "is_entry": False},
                    {"name": "withdraw", "visibility": "public", "is_entry": False},
                    {"name": "deposit", "visibility": "public", "is_entry": False},
                    {"name": "transfer", "visibility": "public", "is_entry": True}
                ]
            },
            deployed_at=0,
            tx_hash="genesis"
        )
        self.modules[coin_module.module_id] = coin_module
        
        # Account module
        account_module = MoveModule(
            address="0x1",
            name="Account",
            bytecode="",
            abi={
                "structs": [
                    {
                        "name": "Account",
                        "abilities": ["key"],
                        "fields": [
                            {"name": "authentication_key", "type": "vector<u8>"},
                            {"name": "sequence_number", "type": "u64"}
                        ]
                    }
                ],
                "functions": [
                    {"name": "create_account", "visibility": "public", "is_entry": True},
                    {"name": "get_sequence_number", "visibility": "public", "is_entry": False},
                    {"name": "exists_at", "visibility": "public", "is_entry": False}
                ]
            },
            deployed_at=0,
            tx_hash="genesis"
        )
        self.modules[account_module.module_id] = account_module
    
    def deploy_module(self, sender: str, name: str, bytecode: str, abi: Dict) -> ExecutionResult:
        """Deploy a new Move module"""
        self._current_sender = sender
        self._gas_remaining = self.MAX_GAS
        self._events = []
        self._changes = []
        
        try:
            # Calculate gas cost
            gas_cost = self.GAS_COSTS["deploy_module"]
            gas_cost += len(bytecode) * self.GAS_COSTS["bytecode_per_byte"] // 2
            
            self._consume_gas(gas_cost)
            
            # Create module
            module_id = f"{sender}::{name}"
            
            # Check if module already exists
            if module_id in self.modules:
                return ExecutionResult(
                    success=False,
                    gas_used=self.MAX_GAS - self._gas_remaining,
                    return_values=[],
                    events=[],
                    error=f"Module {module_id} already exists"
                )
            
            tx_hash = hashlib.sha256(f"{sender}:{name}:{bytecode}".encode()).hexdigest()
            
            module = MoveModule(
                address=sender,
                name=name,
                bytecode=bytecode,
                abi=abi,
                deployed_at=int(datetime.now(timezone.utc).timestamp() * 1000),
                tx_hash=tx_hash
            )
            
            self.modules[module_id] = module
            self.stats["modules_deployed"] += 1
            
            # Emit deployment event
            self._emit_event({
                "type": "ModuleDeployed",
                "module_id": module_id,
                "deployer": sender
            })
            
            # Record change
            self._changes.append({
                "type": "module_published",
                "module_id": module_id
            })
            
            # Persist if available
            if self.persistence:
                self.persistence.save_state(f"move_module:{module_id}", module.to_dict())
            
            return ExecutionResult(
                success=True,
                gas_used=self.MAX_GAS - self._gas_remaining,
                return_values=[module_id],
                events=self._events,
                changes=self._changes
            )
            
        except GasExhaustedException as e:
            return ExecutionResult(
                success=False,
                gas_used=self.MAX_GAS,
                return_values=[],
                events=[],
                error="Out of gas"
            )
        except Exception as e:
            return ExecutionResult(
                success=False,
                gas_used=self.MAX_GAS - self._gas_remaining,
                return_values=[],
                events=[],
                error=str(e)
            )
    
    def execute_function(self, sender: str, module_id: str, function_name: str,
                         type_args: List[str], args: List[Any],
                         gas_limit: int = None) -> ExecutionResult:
        """Execute a Move function"""
        self._current_sender = sender
        self._gas_remaining = gas_limit or self.MAX_GAS
        self._events = []
        self._changes = []
        
        try:
            # Find module
            if module_id not in self.modules:
                return ExecutionResult(
                    success=False,
                    gas_used=0,
                    return_values=[],
                    events=[],
                    error=f"Module {module_id} not found"
                )
            
            module = self.modules[module_id]
            
            # Find function in ABI
            func_info = None
            for f in module.abi.get("functions", []):
                if f["name"] == function_name:
                    func_info = f
                    break
            
            if not func_info:
                return ExecutionResult(
                    success=False,
                    gas_used=0,
                    return_values=[],
                    events=[],
                    error=f"Function {function_name} not found in {module_id}"
                )
            
            # Check if function is callable (entry or public)
            if not func_info.get("is_entry", False) and func_info.get("visibility") != "public":
                return ExecutionResult(
                    success=False,
                    gas_used=0,
                    return_values=[],
                    events=[],
                    error=f"Function {function_name} is not callable"
                )
            
            self._consume_gas(self.GAS_COSTS["call_function"])
            
            # Simulate function execution based on known functions
            result = self._simulate_function(module_id, function_name, type_args, args)
            
            self.stats["functions_called"] += 1
            gas_used = self.MAX_GAS - self._gas_remaining if gas_limit is None else gas_limit - self._gas_remaining
            self.stats["total_gas_used"] += gas_used
            
            return ExecutionResult(
                success=True,
                gas_used=gas_used,
                return_values=result,
                events=self._events,
                changes=self._changes
            )
            
        except GasExhaustedException:
            return ExecutionResult(
                success=False,
                gas_used=gas_limit or self.MAX_GAS,
                return_values=[],
                events=[],
                error="Out of gas"
            )
        except MoveExecutionError as e:
            return ExecutionResult(
                success=False,
                gas_used=(gas_limit or self.MAX_GAS) - self._gas_remaining,
                return_values=[],
                events=[],
                error=str(e)
            )
        except Exception as e:
            return ExecutionResult(
                success=False,
                gas_used=(gas_limit or self.MAX_GAS) - self._gas_remaining,
                return_values=[],
                events=[],
                error=str(e)
            )
    
    def _simulate_function(self, module_id: str, function_name: str,
                           type_args: List[str], args: List[Any]) -> List[Any]:
        """Simulate function execution (placeholder for real VM)"""
        
        # Handle standard library functions
        if module_id == "0x1::Coin":
            if function_name == "transfer":
                # Simulate coin transfer
                if len(args) >= 2:
                    to_address = args[0]
                    amount = args[1]
                    self._emit_event({
                        "type": "CoinTransfer",
                        "from": self._current_sender,
                        "to": to_address,
                        "amount": amount,
                        "coin_type": type_args[0] if type_args else "JASPR"
                    })
                    return [True]
            
            elif function_name == "balance":
                # Return simulated balance
                self._consume_gas(self.GAS_COSTS["read_resource"])
                return [1000000000]  # 1 JASPR
        
        elif module_id == "0x1::Account":
            if function_name == "create_account":
                address = args[0] if args else self._current_sender
                self._emit_event({
                    "type": "AccountCreated",
                    "address": address
                })
                return [True]
        
        elif module_id == "0x1::JASPR":
            if function_name == "transfer":
                if len(args) >= 2:
                    to_address = args[0]
                    amount = args[1]
                    self._emit_event({
                        "type": "JASPRTransfer",
                        "from": self._current_sender,
                        "to": to_address,
                        "amount": amount
                    })
                    return [True]
        
        # For user-deployed modules, return success by default
        self._emit_event({
            "type": "FunctionExecuted",
            "module_id": module_id,
            "function": function_name,
            "sender": self._current_sender
        })
        
        return [True]
    
    def get_resource(self, owner: str, resource_type: str) -> Optional[MoveResource]:
        """Get a resource from an account"""
        key = f"{owner}::{resource_type}"
        return self.resources.get(key)
    
    def set_resource(self, owner: str, resource_type: str, data: Dict):
        """Set a resource in an account"""
        key = f"{owner}::{resource_type}"
        resource = MoveResource(
            owner=owner,
            resource_type=resource_type,
            data=data
        )
        self.resources[key] = resource
        
        if self.persistence:
            self.persistence.save_state(f"move_resource:{key}", resource.to_dict())
    
    def _consume_gas(self, amount: int):
        """Consume gas, raise exception if exhausted"""
        if amount > self._gas_remaining:
            raise GasExhaustedException(f"Required {amount} gas, only {self._gas_remaining} remaining")
        self._gas_remaining -= amount
    
    def _emit_event(self, event: Dict):
        """Emit an event"""
        event["timestamp"] = int(datetime.now(timezone.utc).timestamp() * 1000)
        self._events.append(event)
        self.stats["events_emitted"] += 1
    
    def get_module(self, module_id: str) -> Optional[MoveModule]:
        """Get a deployed module"""
        return self.modules.get(module_id)
    
    def list_modules(self, address: Optional[str] = None) -> List[dict]:
        """List deployed modules"""
        modules = list(self.modules.values())
        if address:
            modules = [m for m in modules if m.address == address]
        return [m.to_dict() for m in modules]
    
    def estimate_gas(self, module_id: str, function_name: str,
                     type_args: List[str], args: List[Any]) -> int:
        """Estimate gas for a function call"""
        base_gas = self.GAS_COSTS["call_function"]
        
        # Add gas for type checking
        base_gas += len(type_args) * self.GAS_COSTS["type_check"]
        
        # Add gas for argument processing
        base_gas += len(args) * 50
        
        # Add buffer for execution
        return int(base_gas * 1.5)
    
    def get_stats(self) -> dict:
        """Get VM statistics"""
        return {
            **self.stats,
            "total_modules": len(self.modules),
            "total_resources": len(self.resources),
            "stdlib_modules": ["0x1::JASPR", "0x1::Coin", "0x1::Account"]
        }


class GasExhaustedException(Exception):
    """Raised when transaction runs out of gas"""
    pass


class MoveExecutionError(Exception):
    """Raised when Move execution fails"""
    pass
