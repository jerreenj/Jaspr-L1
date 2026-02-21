"""AI Risk Scoring Model for JasprChain Sentinel
Actual ML-based risk detection

Features:
- Transaction pattern analysis
- Address reputation scoring
- Anomaly detection
- Scam likelihood prediction
"""
import numpy as np
from sklearn.ensemble import IsolationForest, RandomForestClassifier
from sklearn.preprocessing import StandardScaler
import joblib
import hashlib
from typing import Dict, List, Optional, Any, Tuple
from dataclasses import dataclass
import os


@dataclass
class TransactionFeatures:
    """Features extracted from a transaction for ML scoring"""
    amount: float  # Normalized amount
    gas_price: float  # Normalized gas price
    sender_age_days: float  # Account age
    sender_tx_count: int  # Historical tx count
    recipient_age_days: float
    recipient_tx_count: int
    time_since_last_tx: float  # Hours
    is_contract_interaction: int  # 0 or 1
    is_new_recipient: int  # First time sending to this address
    amount_vs_avg: float  # Ratio to sender's average tx
    gas_vs_avg: float  # Ratio to network average
    hour_of_day: int  # 0-23
    
    def to_array(self) -> np.ndarray:
        return np.array([
            self.amount,
            self.gas_price,
            self.sender_age_days,
            self.sender_tx_count,
            self.recipient_age_days,
            self.recipient_tx_count,
            self.time_since_last_tx,
            self.is_contract_interaction,
            self.is_new_recipient,
            self.amount_vs_avg,
            self.gas_vs_avg,
            self.hour_of_day
        ]).reshape(1, -1)


class RiskModel:
    """ML model for transaction risk scoring
    
    Uses:
    - Isolation Forest for anomaly detection
    - Random Forest for threat classification
    - Heuristic rules for known patterns
    """
    
    def __init__(self):
        self.anomaly_detector = IsolationForest(
            n_estimators=100,
            contamination=0.1,
            random_state=42
        )
        self.threat_classifier: Optional[RandomForestClassifier] = None
        self.scaler = StandardScaler()
        self.is_trained = False
        
        # Known malicious patterns (heuristic layer)
        self.known_scam_addresses: set = set()
        self.known_scam_contracts: set = set()
        self.suspicious_patterns: List[Dict[str, Any]] = []
        
        # Initialize with synthetic training data
        self._initialize_model()
    
    def _initialize_model(self):
        """Initialize model with synthetic normal transaction data"""
        # Generate synthetic normal transaction features
        np.random.seed(42)
        n_samples = 1000
        
        # Normal transaction patterns
        normal_data = np.column_stack([
            np.random.lognormal(10, 2, n_samples),  # amount
            np.random.normal(20, 5, n_samples),     # gas_price
            np.random.exponential(180, n_samples),  # sender_age_days
            np.random.poisson(50, n_samples),       # sender_tx_count
            np.random.exponential(180, n_samples),  # recipient_age_days
            np.random.poisson(50, n_samples),       # recipient_tx_count
            np.random.exponential(24, n_samples),   # time_since_last_tx
            np.random.binomial(1, 0.3, n_samples),  # is_contract_interaction
            np.random.binomial(1, 0.2, n_samples),  # is_new_recipient
            np.random.normal(1, 0.3, n_samples),    # amount_vs_avg
            np.random.normal(1, 0.2, n_samples),    # gas_vs_avg
            np.random.randint(0, 24, n_samples)     # hour_of_day
        ])
        
        # Fit scaler and anomaly detector
        self.scaler.fit(normal_data)
        scaled_data = self.scaler.transform(normal_data)
        self.anomaly_detector.fit(scaled_data)
        
        # Create threat classifier with synthetic labeled data
        n_threat_samples = 200
        threat_data = np.column_stack([
            np.random.lognormal(15, 3, n_threat_samples),  # Higher amounts
            np.random.normal(50, 20, n_threat_samples),    # Unusual gas
            np.random.exponential(10, n_threat_samples),   # New accounts
            np.random.poisson(5, n_threat_samples),        # Low tx count
            np.random.exponential(30, n_threat_samples),   # New recipients
            np.random.poisson(10, n_threat_samples),
            np.random.exponential(1, n_threat_samples),    # Rapid succession
            np.random.binomial(1, 0.8, n_threat_samples),  # Often contracts
            np.random.binomial(1, 0.9, n_threat_samples),  # New recipients
            np.random.lognormal(1, 1, n_threat_samples),   # Unusual amounts
            np.random.lognormal(1, 0.5, n_threat_samples), # Unusual gas
            np.random.randint(0, 24, n_threat_samples)
        ])
        
        # Combine and label
        all_data = np.vstack([normal_data, threat_data])
        labels = np.array([0] * n_samples + [1] * n_threat_samples)
        
        # Train classifier
        self.threat_classifier = RandomForestClassifier(
            n_estimators=50,
            max_depth=10,
            random_state=42
        )
        scaled_all = self.scaler.transform(all_data)
        self.threat_classifier.fit(scaled_all, labels)
        
        self.is_trained = True
    
    def score_transaction(self, features: TransactionFeatures) -> Tuple[float, Dict[str, Any]]:
        """Score a transaction for risk
        
        Returns (risk_score 0-100, detailed_analysis)
        """
        feature_array = features.to_array()
        
        # Scale features
        try:
            scaled = self.scaler.transform(feature_array)
        except:
            scaled = feature_array
        
        # Anomaly score (-1 = anomaly, 1 = normal)
        anomaly_score = self.anomaly_detector.decision_function(scaled)[0]
        anomaly_risk = max(0, min(100, (1 - anomaly_score) * 50))
        
        # Threat classification
        threat_prob = 0.0
        if self.threat_classifier:
            threat_prob = self.threat_classifier.predict_proba(scaled)[0][1]
        threat_risk = threat_prob * 100
        
        # Heuristic rules
        heuristic_risk = self._apply_heuristics(features)
        
        # Combine scores (weighted average)
        final_score = (
            anomaly_risk * 0.3 +
            threat_risk * 0.4 +
            heuristic_risk * 0.3
        )
        
        analysis = {
            'anomaly_score': round(anomaly_risk, 2),
            'threat_probability': round(threat_prob, 4),
            'threat_risk': round(threat_risk, 2),
            'heuristic_risk': round(heuristic_risk, 2),
            'final_score': round(final_score, 2),
            'factors': self._get_risk_factors(features, anomaly_risk, threat_risk, heuristic_risk)
        }
        
        return final_score, analysis
    
    def _apply_heuristics(self, features: TransactionFeatures) -> float:
        """Apply rule-based heuristics"""
        risk = 0.0
        
        # New account sending large amount
        if features.sender_age_days < 1 and features.amount > 1000000:
            risk += 30
        
        # Rapid transactions
        if features.time_since_last_tx < 0.1:  # < 6 minutes
            risk += 15
        
        # Unusual amount ratio
        if features.amount_vs_avg > 10:
            risk += 20
        
        # New recipient with high value
        if features.is_new_recipient and features.amount > 100000:
            risk += 15
        
        # Contract interaction from new account
        if features.is_contract_interaction and features.sender_tx_count < 5:
            risk += 10
        
        # Unusual hour (potential automated attack)
        if features.hour_of_day in [2, 3, 4, 5] and features.amount_vs_avg > 5:
            risk += 10
        
        return min(100, risk)
    
    def _get_risk_factors(self, features: TransactionFeatures, anomaly: float, threat: float, heuristic: float) -> List[str]:
        """Generate human-readable risk factors"""
        factors = []
        
        if anomaly > 50:
            factors.append("Transaction pattern deviates from normal behavior")
        
        if threat > 50:
            factors.append("Transaction matches known threat patterns")
        
        if features.sender_age_days < 1:
            factors.append("Sender account is very new (<24h)")
        
        if features.amount_vs_avg > 5:
            factors.append(f"Amount is {features.amount_vs_avg:.1f}x sender's average")
        
        if features.is_new_recipient and features.amount > 100000:
            factors.append("Large transfer to new recipient")
        
        if features.time_since_last_tx < 0.1:
            factors.append("Rapid transaction sequence detected")
        
        return factors
    
    def add_known_scam_address(self, address: str):
        """Add address to known scam list"""
        self.known_scam_addresses.add(address.lower())
    
    def add_known_scam_contract(self, address: str):
        """Add contract to known scam list"""
        self.known_scam_contracts.add(address.lower())
    
    def is_known_scam(self, address: str) -> bool:
        """Check if address is known scam"""
        addr_lower = address.lower()
        return addr_lower in self.known_scam_addresses or addr_lower in self.known_scam_contracts
