"""Audit logging for security compliance and accountability.

This module provides tamper-evident audit logging for security-relevant events
including authentication, data access, and system events.
"""
from __future__ import annotations
import json
import hashlib
import time
from pathlib import Path
from typing import Optional, Dict, Any, List
from dataclasses import dataclass, asdict
from datetime import datetime, timezone
from enum import Enum
from .db import connect


class AuditEventType(Enum):
    """Types of audit events."""
    # Authentication events
    AUTH_LOGIN_SUCCESS = "auth.login.success"
    AUTH_LOGIN_FAILURE = "auth.login.failure"
    AUTH_LOGOUT = "auth.logout"
    AUTH_SESSION_TIMEOUT = "auth.session.timeout"
    AUTH_PASSWORD_CHANGED = "auth.password.changed"
    
    # Data access events
    DATA_VIEW = "data.view"
    DATA_CREATE = "data.create"
    DATA_UPDATE = "data.update"
    DATA_DELETE = "data.delete"
    DATA_EXPORT = "data.export"
    
    # Security events
    SECURITY_PERMISSION_DENIED = "security.permission.denied"
    SECURITY_INVALIDATION = "security.invalidation"
    SECURITY_CONFIG_CHANGED = "security.config.changed"
    
    # System events
    SYSTEM_STARTUP = "system.startup"
    SYSTEM_SHUTDOWN = "system.shutdown"
    SYSTEM_BACKUP = "system.backup"
    SYSTEM_RESTORE = "system.restore"


@dataclass
class AuditEvent:
    """Audit event record."""
    event_id: int
    event_type: str
    timestamp: float
    user_id: Optional[str]
    action: str
    result: str  # success, failure, denied
    ip_address: Optional[str]
    user_agent: Optional[str]
    resource_type: Optional[str]
    resource_id: Optional[str]
    details: Optional[Dict[str, Any]]
    previous_hash: str  # Hash of previous event for tamper-evidence
    event_hash: str  # Hash of this event
    
    def to_dict(self) -> dict:
        """Convert to dictionary."""
        return asdict(self)
    
    @classmethod
    def from_row(cls, row) -> 'AuditEvent':
        """Create from database row."""
        return cls(
            event_id=row[0],
            event_type=row[1],
            timestamp=row[2],
            user_id=row[3],
            action=row[4],
            result=row[5],
            ip_address=row[6],
            user_agent=row[7],
            resource_type=row[8],
            resource_id=row[9],
            details=json.loads(row[10]) if row[10] else None,
            previous_hash=row[11],
            event_hash=row[12]
        )


class AuditLogger:
    """Tamper-evident audit logging system.
    
    Features:
    - Append-only log storage
    - Cryptographic hash chaining
    - Event categorization
    - Configurable retention
    - Export capabilities
    - Integrity verification
    """
    
    def __init__(self, db_path: str | Path = 'fyi_system.db'):
        """Initialize audit logger.
        
        Args:
            db_path: Path to SQLite database
        """
        self.db_path = db_path
        self._ensure_audit_table()
    
    def _ensure_audit_table(self) -> None:
        """Create audit_log table if it doesn't exist."""
        conn = connect(self.db_path)
        try:
            conn.execute("""
                CREATE TABLE IF NOT EXISTS audit_log (
                    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    event_type TEXT NOT NULL,
                    timestamp REAL NOT NULL,
                    user_id TEXT,
                    action TEXT NOT NULL,
                    result TEXT NOT NULL,
                    ip_address TEXT,
                    user_agent TEXT,
                    resource_type TEXT,
                    resource_id TEXT,
                    details TEXT,
                    previous_hash TEXT NOT NULL,
                    event_hash TEXT NOT NULL
                )
            """)
            
            # Indexes for common queries
            conn.execute("""
                CREATE INDEX IF NOT EXISTS idx_audit_timestamp 
                ON audit_log(timestamp)
            """)
            conn.execute("""
                CREATE INDEX IF NOT EXISTS idx_audit_user 
                ON audit_log(user_id)
            """)
            conn.execute("""
                CREATE INDEX IF NOT EXISTS idx_audit_type 
                ON audit_log(event_type)
            """)
            
            conn.commit()
        finally:
            conn.close()
    
    def _get_last_hash(self) -> str:
        """Get hash of last event for chaining.
        
        Returns:
            Hash of last event or genesis hash if empty
        """
        conn = connect(self.db_path)
        try:
            cursor = conn.execute("""
                SELECT event_hash FROM audit_log 
                ORDER BY event_id DESC LIMIT 1
            """)
            row = cursor.fetchone()
            return row[0] if row else self._genesis_hash()
        finally:
            conn.close()
    
    def _genesis_hash(self) -> str:
        """Generate genesis hash for chain start.
        
        Returns:
            Genesis hash string (constant)
        """
        # Use constant genesis block data for consistent verification
        genesis_data = "FYI-Audit-Log-Genesis-Block-v1"
        return hashlib.sha256(genesis_data.encode('utf-8')).hexdigest()
    
    def _compute_event_hash(
        self,
        event_type: str,
        timestamp: float,
        user_id: Optional[str],
        action: str,
        result: str,
        previous_hash: str,
        details: Optional[Dict[str, Any]] = None
    ) -> str:
        """Compute hash for an event.
        
        Returns:
            SHA-256 hash of event data
        """
        hash_data = json.dumps({
            "event_type": event_type,
            "timestamp": timestamp,
            "user_id": user_id,
            "action": action,
            "result": result,
            "previous_hash": previous_hash,
            "details": details or {}
        }, sort_keys=True)
        
        return hashlib.sha256(hash_data.encode('utf-8')).hexdigest()
    
    def log(
        self,
        event_type: AuditEventType | str,
        action: str,
        result: str,
        user_id: Optional[str] = None,
        ip_address: Optional[str] = None,
        user_agent: Optional[str] = None,
        resource_type: Optional[str] = None,
        resource_id: Optional[str] = None,
        details: Optional[Dict[str, Any]] = None
    ) -> AuditEvent:
        """Log an audit event.
        
        Args:
            event_type: Type of event (enum or string)
            action: Human-readable action description
            result: Event result (success, failure, denied)
            user_id: User identifier (optional)
            ip_address: Client IP address (optional)
            user_agent: Client User-Agent (optional)
            resource_type: Type of resource accessed (optional)
            resource_id: ID of resource accessed (optional)
            details: Additional event details (optional)
        
        Returns:
            Created AuditEvent object
        """
        timestamp = time.time()
        previous_hash = self._get_last_hash()
        
        # Convert enum to string if needed
        if isinstance(event_type, AuditEventType):
            event_type_str = event_type.value
        else:
            event_type_str = event_type
        
        # Compute event hash
        event_hash = self._compute_event_hash(
            event_type_str,
            timestamp,
            user_id,
            action,
            result,
            previous_hash,
            details
        )
        
        # Serialize details
        details_json = json.dumps(details) if details else None
        
        # Insert into database
        conn = connect(self.db_path)
        try:
            cursor = conn.execute("""
                INSERT INTO audit_log 
                (event_type, timestamp, user_id, action, result, 
                 ip_address, user_agent, resource_type, resource_id, 
                 details, previous_hash, event_hash)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """, (
                event_type_str,
                timestamp,
                user_id,
                action,
                result,
                ip_address,
                user_agent,
                resource_type,
                resource_id,
                details_json,
                previous_hash,
                event_hash
            ))
            
            event_id = cursor.lastrowid
            conn.commit()
            
            # Create and return event object
            return AuditEvent(
                event_id=event_id,
                event_type=event_type_str,
                timestamp=timestamp,
                user_id=user_id,
                action=action,
                result=result,
                ip_address=ip_address,
                user_agent=user_agent,
                resource_type=resource_type,
                resource_id=resource_id,
                details=details,
                previous_hash=previous_hash,
                event_hash=event_hash
            )
        finally:
            conn.close()
    
    # Convenience methods for common events
    
    def log_auth_success(
        self,
        user_id: str,
        ip_address: Optional[str] = None,
        user_agent: Optional[str] = None,
        details: Optional[Dict[str, Any]] = None
    ) -> AuditEvent:
        """Log successful authentication."""
        return self.log(
            event_type=AuditEventType.AUTH_LOGIN_SUCCESS,
            action="User login successful",
            result="success",
            user_id=user_id,
            ip_address=ip_address,
            user_agent=user_agent,
            details=details
        )
    
    def log_auth_failure(
        self,
        user_id: str,
        ip_address: Optional[str] = None,
        user_agent: Optional[str] = None,
        reason: str = "Invalid credentials"
    ) -> AuditEvent:
        """Log failed authentication."""
        return self.log(
            event_type=AuditEventType.AUTH_LOGIN_FAILURE,
            action="User login failed",
            result="failure",
            user_id=user_id,
            ip_address=ip_address,
            user_agent=user_agent,
            details={"reason": reason}
        )
    
    def log_logout(
        self,
        user_id: str,
        ip_address: Optional[str] = None
    ) -> AuditEvent:
        """Log user logout."""
        return self.log(
            event_type=AuditEventType.AUTH_LOGOUT,
            action="User logout",
            result="success",
            user_id=user_id,
            ip_address=ip_address
        )
    
    def log_data_access(
        self,
        user_id: str,
        resource_type: str,
        resource_id: str,
        action: str = "view",
        result: str = "success",
        ip_address: Optional[str] = None
    ) -> AuditEvent:
        """Log data access event."""
        return self.log(
            event_type=AuditEventType.DATA_VIEW,
            action=f"Data {action}: {resource_type}/{resource_id}",
            result=result,
            user_id=user_id,
            ip_address=ip_address,
            resource_type=resource_type,
            resource_id=resource_id
        )
    
    def log_permission_denied(
        self,
        user_id: str,
        resource_type: str,
        resource_id: str,
        ip_address: Optional[str] = None
    ) -> AuditEvent:
        """Log permission denied event."""
        return self.log(
            event_type=AuditEventType.SECURITY_PERMISSION_DENIED,
            action=f"Permission denied: {resource_type}/{resource_id}",
            result="denied",
            user_id=user_id,
            ip_address=ip_address,
            resource_type=resource_type,
            resource_id=resource_id
        )
    
    # Query methods
    
    def get_events(
        self,
        limit: int = 100,
        offset: int = 0,
        user_id: Optional[str] = None,
        event_type: Optional[str] = None,
        start_time: Optional[float] = None,
        end_time: Optional[float] = None
    ) -> List[AuditEvent]:
        """Query audit events.
        
        Args:
            limit: Maximum events to return
            offset: Offset for pagination
            user_id: Filter by user ID
            event_type: Filter by event type
            start_time: Filter by start timestamp
            end_time: Filter by end timestamp
        
        Returns:
            List of AuditEvent objects
        """
        conn = connect(self.db_path)
        try:
            # Build query
            query = "SELECT * FROM audit_log WHERE 1=1"
            params = []
            
            if user_id:
                query += " AND user_id = ?"
                params.append(user_id)
            
            if event_type:
                query += " AND event_type = ?"
                params.append(event_type)
            
            if start_time:
                query += " AND timestamp >= ?"
                params.append(start_time)
            
            if end_time:
                query += " AND timestamp <= ?"
                params.append(end_time)
            
            query += " ORDER BY timestamp DESC LIMIT ? OFFSET ?"
            params.extend([limit, offset])
            
            cursor = conn.execute(query, params)
            return [AuditEvent.from_row(row) for row in cursor.fetchall()]
        finally:
            conn.close()
    
    def verify_integrity(self) -> Dict[str, Any]:
        """Verify audit log integrity.
        
        Checks that hash chain is unbroken.
        
        Returns:
            Dictionary with verification results
        """
        conn = connect(self.db_path)
        try:
            cursor = conn.execute("""
                SELECT event_id, previous_hash, event_hash, details
                FROM audit_log 
                ORDER BY event_id ASC
            """)
            
            events = cursor.fetchall()
            if not events:
                return {
                    "valid": True,
                    "total_events": 0,
                    "message": "Audit log is empty"
                }
            
            # Verify chain
            expected_previous = self._genesis_hash()
            broken_chains = []
            
            for event_id, previous_hash, event_hash, details_json in events:
                # Verify previous hash matches
                if previous_hash != expected_previous:
                    broken_chains.append({
                        "event_id": event_id,
                        "issue": "broken_chain",
                        "expected": expected_previous,
                        "found": previous_hash
                    })
                
                # Recompute and verify event hash using stored details JSON directly
                cursor2 = conn.execute("""
                    SELECT event_type, timestamp, user_id, action, result
                    FROM audit_log WHERE event_id = ?
                """, (event_id,))
                row = cursor2.fetchone()
                
                if row:
                    # Parse details exactly as stored, use {} for None to match hash computation
                    details = json.loads(details_json) if details_json else None
                    
                    computed_hash = self._compute_event_hash(
                        row[0], row[1], row[2], row[3], row[4], 
                        previous_hash,
                        details or {}  # Match the `details or {}` in _compute_event_hash
                    )
                    
                    if computed_hash != event_hash:
                        broken_chains.append({
                            "event_id": event_id,
                            "issue": "hash_mismatch",
                            "expected": computed_hash,
                            "found": event_hash
                        })
                
                expected_previous = event_hash
            
            return {
                "valid": len(broken_chains) == 0,
                "total_events": len(events),
                "broken_chains": broken_chains,
                "message": "Audit log integrity verified" if not broken_chains 
                          else f"Audit log integrity compromised: {len(broken_chains)} issues found"
            }
        finally:
            conn.close()
    
    def export_events(
        self,
        output_path: str | Path,
        start_time: Optional[float] = None,
        end_time: Optional[float] = None
    ) -> int:
        """Export audit events to JSON file.
        
        Args:
            output_path: Path to output file
            start_time: Filter by start timestamp
            end_time: Filter by end timestamp
        
        Returns:
            Number of events exported
        """
        events = self.get_events(
            limit=100000,  # Large limit for export
            start_time=start_time,
            end_time=end_time
        )
        
        export_data = {
            "exported_at": datetime.now(timezone.utc).isoformat(),
            "total_events": len(events),
            "integrity": self.verify_integrity(),
            "events": [event.to_dict() for event in events]
        }
        
        Path(output_path).write_text(json.dumps(export_data, indent=2))
        return len(events)
    
    def get_stats(self) -> Dict[str, Any]:
        """Get audit log statistics.
        
        Returns:
            Dictionary with statistics
        """
        conn = connect(self.db_path)
        try:
            # Total events
            cursor = conn.execute("SELECT COUNT(*) FROM audit_log")
            total = cursor.fetchone()[0]
            
            # Events by type
            cursor = conn.execute("""
                SELECT event_type, COUNT(*) as count 
                FROM audit_log 
                GROUP BY event_type 
                ORDER BY count DESC
                LIMIT 10
            """)
            by_type = {row[0]: row[1] for row in cursor.fetchall()}
            
            # Events by result
            cursor = conn.execute("""
                SELECT result, COUNT(*) as count 
                FROM audit_log 
                GROUP BY result
            """)
            by_result = {row[0]: row[1] for row in cursor.fetchall()}
            
            # Recent activity (last 24 hours)
            now = time.time()
            day_ago = now - 86400
            cursor = conn.execute("""
                SELECT COUNT(*) FROM audit_log 
                WHERE timestamp >= ?
            """, (day_ago,))
            last_24h = cursor.fetchone()[0]
            
            return {
                "total_events": total,
                "by_type": by_type,
                "by_result": by_result,
                "last_24_hours": last_24h
            }
        finally:
            conn.close()


# Global instance for convenience
_default_logger: Optional[AuditLogger] = None


def get_audit_logger(db_path: str | Path = 'fyi_system.db') -> AuditLogger:
    """Get or create default audit logger.
    
    Args:
        db_path: Path to database
    
    Returns:
        AuditLogger instance
    """
    global _default_logger
    if _default_logger is None or _default_logger.db_path != db_path:
        _default_logger = AuditLogger(db_path)
    return _default_logger


# Convenience functions

def log_auth_success(user_id: str, db_path: str | Path = 'fyi_system.db', **kwargs) -> AuditEvent:
    """Log successful authentication."""
    return get_audit_logger(db_path).log_auth_success(user_id, **kwargs)


def log_auth_failure(user_id: str, db_path: str | Path = 'fyi_system.db', **kwargs) -> AuditEvent:
    """Log failed authentication."""
    return get_audit_logger(db_path).log_auth_failure(user_id, **kwargs)


def log_logout(user_id: str, db_path: str | Path = 'fyi_system.db', **kwargs) -> AuditEvent:
    """Log user logout."""
    return get_audit_logger(db_path).log_logout(user_id, **kwargs)


def log_data_access(user_id: str, resource_type: str, resource_id: str, db_path: str | Path = 'fyi_system.db', **kwargs) -> AuditEvent:
    """Log data access."""
    return get_audit_logger(db_path).log_data_access(user_id, resource_type, resource_id, **kwargs)


def log_permission_denied(user_id: str, resource_type: str, resource_id: str, db_path: str | Path = 'fyi_system.db', **kwargs) -> AuditEvent:
    """Log permission denied."""
    return get_audit_logger(db_path).log_permission_denied(user_id, resource_type, resource_id, **kwargs)
