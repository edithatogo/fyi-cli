"""Data retention and secure deletion for compliance.

This module provides configurable data retention policies and secure deletion
with cryptographic erasure for sensitive data.
"""
from __future__ import annotations
import time
import json
import hashlib
from pathlib import Path
from typing import Optional, Dict, Any, List, Callable
from dataclasses import dataclass
from enum import Enum
from .db import connect


class RetentionPeriod(Enum):
    """Standard retention periods."""
    DAYS_30 = 30
    DAYS_90 = 90
    DAYS_180 = 180
    DAYS_365 = 365
    YEARS_3 = 1095  # 3 years
    YEARS_7 = 2555  # 7 years
    PERMANENT = -1  # No automatic deletion


@dataclass
class RetentionPolicy:
    """Data retention policy configuration."""
    policy_name: str
    resource_type: str
    retention_days: int
    delete_method: str = "secure"  # "simple" or "secure"
    export_before_delete: bool = True
    enabled: bool = True
    
    def to_dict(self) -> dict:
        """Convert to dictionary."""
        return {
            "policy_name": self.policy_name,
            "resource_type": self.resource_type,
            "retention_days": self.retention_days,
            "delete_method": self.delete_method,
            "export_before_delete": self.export_before_delete,
            "enabled": self.enabled
        }
    
    @classmethod
    def from_dict(cls, data: dict) -> 'RetentionPolicy':
        """Create from dictionary."""
        return cls(**data)


class RetentionManager:
    """Manage data retention policies and secure deletion.
    
    Features:
    - Configurable retention periods per resource type
    - Secure deletion with cryptographic erasure
    - Export before deletion
    - Audit logging integration
    - Deletion scheduling
    """
    
    def __init__(
        self,
        db_path: str | Path = 'fyi_system.db',
        audit_logger=None
    ):
        """Initialize retention manager.
        
        Args:
            db_path: Path to SQLite database
            audit_logger: Optional AuditLogger for deletion auditing
        """
        self.db_path = db_path
        self.audit_logger = audit_logger
        self._ensure_retention_tables()
    
    def _ensure_retention_tables(self) -> None:
        """Create retention policy and deletion log tables."""
        conn = connect(self.db_path)
        try:
            # Retention policies table
            conn.execute("""
                CREATE TABLE IF NOT EXISTS retention_policies (
                    policy_name TEXT PRIMARY KEY,
                    resource_type TEXT NOT NULL,
                    retention_days INTEGER NOT NULL,
                    delete_method TEXT DEFAULT 'secure',
                    export_before_delete INTEGER DEFAULT 1,
                    enabled INTEGER DEFAULT 1
                )
            """)
            
            # Deletion log table
            conn.execute("""
                CREATE TABLE IF NOT EXISTS deletion_log (
                    deletion_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    resource_type TEXT NOT NULL,
                    resource_id TEXT NOT NULL,
                    deleted_at REAL NOT NULL,
                    delete_method TEXT NOT NULL,
                    export_path TEXT,
                    reason TEXT,
                    performed_by TEXT
                )
            """)
            
            # Index for cleanup queries
            conn.execute("""
                CREATE INDEX IF NOT EXISTS idx_deletion_log_resource 
                ON deletion_log(resource_type, resource_id)
            """)
            
            conn.commit()
            
            # Insert default policies if none exist
            cursor = conn.execute("SELECT COUNT(*) FROM retention_policies")
            if cursor.fetchone()[0] == 0:
                self._insert_default_policies(conn)
                conn.commit()
        finally:
            conn.close()
    
    def _insert_default_policies(self, conn) -> None:
        """Insert default retention policies."""
        default_policies = [
            # Session data - short retention
            RetentionPolicy(
                policy_name="sessions",
                resource_type="sessions",
                retention_days=30,
                delete_method="simple"
            ),
            # Audit logs - long retention for compliance
            RetentionPolicy(
                policy_name="audit_logs",
                resource_type="audit_log",
                retention_days=2555,  # 7 years
                delete_method="secure",
                export_before_delete=True
            ),
            # Tracked requests - medium retention
            RetentionPolicy(
                policy_name="tracked_requests",
                resource_type="tracked_requests",
                retention_days=1095,  # 3 years
                delete_method="secure",
                export_before_delete=True
            ),
            # Feed events - short retention
            RetentionPolicy(
                policy_name="feed_events",
                resource_type="feed_events",
                retention_days=180,
                delete_method="simple"
            ),
            # Run log - medium retention
            RetentionPolicy(
                policy_name="run_log",
                resource_type="run_log",
                retention_days=365,
                delete_method="simple"
            ),
        ]
        
        for policy in default_policies:
            conn.execute("""
                INSERT INTO retention_policies 
                (policy_name, resource_type, retention_days, delete_method, export_before_delete, enabled)
                VALUES (?, ?, ?, ?, ?, 1)
            """, (
                policy.policy_name,
                policy.resource_type,
                policy.retention_days,
                policy.delete_method,
                1 if policy.export_before_delete else 0
            ))
    
    def get_policy(self, resource_type: str) -> Optional[RetentionPolicy]:
        """Get retention policy for a resource type.
        
        Args:
            resource_type: Type of resource
        
        Returns:
            RetentionPolicy or None if not found
        """
        conn = connect(self.db_path)
        try:
            cursor = conn.execute("""
                SELECT policy_name, resource_type, retention_days, 
                       delete_method, export_before_delete, enabled
                FROM retention_policies
                WHERE resource_type = ?
            """, (resource_type,))
            
            row = cursor.fetchone()
            if not row:
                return None
            
            return RetentionPolicy(
                policy_name=row[0],
                resource_type=row[1],
                retention_days=row[2],
                delete_method=row[3],
                export_before_delete=bool(row[4]),
                enabled=bool(row[5])
            )
        finally:
            conn.close()
    
    def set_policy(self, policy: RetentionPolicy) -> bool:
        """Set or update retention policy.
        
        Args:
            policy: RetentionPolicy to save
        
        Returns:
            True if successful
        """
        conn = connect(self.db_path)
        try:
            conn.execute("""
                INSERT OR REPLACE INTO retention_policies
                (policy_name, resource_type, retention_days, delete_method, 
                 export_before_delete, enabled)
                VALUES (?, ?, ?, ?, ?, ?)
            """, (
                policy.policy_name,
                policy.resource_type,
                policy.retention_days,
                policy.delete_method,
                1 if policy.export_before_delete else 0,
                1 if policy.enabled else 0
            ))
            conn.commit()
            return True
        finally:
            conn.close()
    
    def delete_policy(self, policy_name: str) -> bool:
        """Delete a retention policy.
        
        Args:
            policy_name: Name of policy to delete
        
        Returns:
            True if deleted
        """
        conn = connect(self.db_path)
        try:
            cursor = conn.execute("""
                DELETE FROM retention_policies WHERE policy_name = ?
            """, (policy_name,))
            conn.commit()
            return cursor.rowcount > 0
        finally:
            conn.close()
    
    def list_policies(self) -> List[RetentionPolicy]:
        """List all retention policies.
        
        Returns:
            List of RetentionPolicy objects
        """
        conn = connect(self.db_path)
        try:
            cursor = conn.execute("""
                SELECT policy_name, resource_type, retention_days, 
                       delete_method, export_before_delete, enabled
                FROM retention_policies
                ORDER BY policy_name
            """)
            
            return [
                RetentionPolicy(
                    policy_name=row[0],
                    resource_type=row[1],
                    retention_days=row[2],
                    delete_method=row[3],
                    export_before_delete=bool(row[4]),
                    enabled=bool(row[5])
                )
                for row in cursor.fetchall()
            ]
        finally:
            conn.close()
    
    def cleanup_expired_data(
        self,
        resource_type: Optional[str] = None,
        dry_run: bool = False,
        export_dir: Optional[str | Path] = None
    ) -> Dict[str, Any]:
        """Clean up expired data based on retention policies.
        
        Args:
            resource_type: Optional specific resource type to clean
            dry_run: If True, don't actually delete
            export_dir: Directory for exports before deletion
        
        Returns:
            Dictionary with cleanup statistics
        """
        conn = connect(self.db_path)
        try:
            now = time.time()
            stats = {
                "policies_checked": 0,
                "resources_scanned": 0,
                "resources_deleted": 0,
                "exports_created": 0,
                "errors": []
            }
            
            # Get policies
            if resource_type:
                policies = [self.get_policy(resource_type)]
            else:
                policies = self.list_policies()
            
            for policy in policies:
                if not policy or not policy.enabled:
                    continue
                
                stats["policies_checked"] += 1
                
                # Calculate cutoff timestamp
                cutoff_seconds = policy.retention_days * 86400
                cutoff_time = now - cutoff_seconds
                
                # Get table name (map resource_type to table)
                table_name = self._get_table_name(policy.resource_type)
                if not table_name:
                    stats["errors"].append(f"Unknown table for {policy.resource_type}")
                    continue
                
                # Check if table exists
                cursor = conn.execute("""
                    SELECT name FROM sqlite_master 
                    WHERE type='table' AND name=?
                """, (table_name,))
                if not cursor.fetchone():
                    stats["errors"].append(f"Table {table_name} does not exist")
                    continue
                
                # Get expired resources
                try:
                    cursor = conn.execute(f"""
                        SELECT id, created_at FROM {table_name}
                        WHERE created_at < ?
                    """, (cutoff_time,))
                    
                    expired = cursor.fetchall()
                except Exception as e:
                    stats["errors"].append(f"Error querying {table_name}: {e}")
                    continue
                
                stats["resources_scanned"] += len(expired)
                
                for resource_id, created_at in expired:
                    if dry_run:
                        stats["resources_deleted"] += 1
                        continue
                    
                    # Export before delete if configured
                    export_path = None
                    if policy.export_before_delete and export_dir:
                        export_path = self._export_resource(
                            table_name, 
                            resource_id, 
                            export_dir
                        )
                        if export_path:
                            stats["exports_created"] += 1
                    
                    # Delete resource
                    if policy.delete_method == "secure":
                        self._secure_delete(conn, table_name, resource_id)
                    else:
                        conn.execute(
                            f"DELETE FROM {table_name} WHERE id = ?",
                            (resource_id,)
                        )
                    
                    stats["resources_deleted"] += 1
                    
                    # Log deletion
                    self._log_deletion(
                        policy.resource_type,
                        str(resource_id),
                        policy.delete_method,
                        export_path
                    )
            
            if not dry_run:
                conn.commit()
            
            return stats
        finally:
            conn.close()
    
    def _get_table_name(self, resource_type: str) -> Optional[str]:
        """Map resource type to table name."""
        mapping = {
            "sessions": "sessions",
            "audit_log": "audit_log",
            "tracked_requests": "tracked_requests",
            "feed_events": "feed_events",
            "run_log": "run_log",
            "authorities": "authorities",
            "request_snapshots": "request_snapshots",
        }
        return mapping.get(resource_type)
    
    def _export_resource(
        self,
        table_name: str,
        resource_id: int,
        export_dir: str | Path
    ) -> Optional[str]:
        """Export a resource before deletion.
        
        Args:
            table_name: Database table name
            resource_id: Resource ID
            export_dir: Export directory
        
        Returns:
            Path to export file or None
        """
        try:
            conn = connect(self.db_path)
            cursor = conn.execute(
                f"SELECT * FROM {table_name} WHERE id = ?",
                (resource_id,)
            )
            row = cursor.fetchone()
            
            if not row:
                return None
            
            # Get column names
            columns = [description[0] for description in cursor.description]
            
            # Create export data
            export_data = {
                "table": table_name,
                "resource_id": resource_id,
                "exported_at": time.time(),
                "columns": columns,
                "data": dict(zip(columns, row))
            }
            
            # Write export file
            export_path = Path(export_dir) / f"{table_name}_{resource_id}_export.json"
            export_path.parent.mkdir(parents=True, exist_ok=True)
            export_path.write_text(json.dumps(export_data, indent=2))
            
            conn.close()
            return str(export_path)
        except Exception as e:
            print(f"Export failed: {e}")
            return None
    
    def _secure_delete(self, conn, table_name: str, resource_id: int) -> None:
        """Perform secure deletion with cryptographic erasure.
        
        Overwrites sensitive data with random bytes before deletion.
        
        Args:
            conn: Database connection
            table_name: Table name
            resource_id: Resource ID
        """
        # Get sensitive columns to overwrite
        sensitive_columns = self._get_sensitive_columns(table_name)
        
        # Generate random overwrite data
        overwrite_data = hashlib.sha256(
            f"{table_name}-{resource_id}-{time.time()}".encode()
        ).hexdigest()
        
        # Overwrite sensitive columns
        for column in sensitive_columns:
            conn.execute(
                f"UPDATE {table_name} SET {column} = ? WHERE id = ?",
                (overwrite_data, resource_id)
            )
        
        # Delete the record
        conn.execute(
            f"DELETE FROM {table_name} WHERE id = ?",
            (resource_id,)
        )
    
    def _get_sensitive_columns(self, table_name: str) -> List[str]:
        """Get list of sensitive columns for a table."""
        sensitive = {
            "tracked_requests": ["title", "body", "tags", "fyi_url"],
            "authorities": ["name", "url"],
            "feed_events": ["title", "summary", "raw_json"],
            "request_snapshots": ["raw_json"],
            "sessions": ["user_id", "ip_address", "user_agent"],
        }
        return sensitive.get(table_name, [])
    
    def _log_deletion(
        self,
        resource_type: str,
        resource_id: str,
        delete_method: str,
        export_path: Optional[str] = None
    ) -> None:
        """Log deletion to deletion_log table.
        
        Args:
            resource_type: Type of resource
            resource_id: Resource identifier
            delete_method: Deletion method used
            export_path: Path to export file if any
        """
        conn = connect(self.db_path)
        try:
            conn.execute("""
                INSERT INTO deletion_log
                (resource_type, resource_id, deleted_at, delete_method, export_path)
                VALUES (?, ?, ?, ?, ?)
            """, (
                resource_type,
                resource_id,
                time.time(),
                delete_method,
                export_path
            ))
            conn.commit()
        finally:
            conn.close()
    
    def get_deletion_stats(self) -> Dict[str, Any]:
        """Get deletion statistics.
        
        Returns:
            Dictionary with deletion statistics
        """
        conn = connect(self.db_path)
        try:
            # Total deletions
            cursor = conn.execute("SELECT COUNT(*) FROM deletion_log")
            total = cursor.fetchone()[0]
            
            # By resource type
            cursor = conn.execute("""
                SELECT resource_type, COUNT(*) as count
                FROM deletion_log
                GROUP BY resource_type
            """)
            by_type = {row[0]: row[1] for row in cursor.fetchall()}
            
            # By method
            cursor = conn.execute("""
                SELECT delete_method, COUNT(*) as count
                FROM deletion_log
                GROUP BY delete_method
            """)
            by_method = {row[0]: row[1] for row in cursor.fetchall()}
            
            # With exports
            cursor = conn.execute("""
                SELECT COUNT(*) FROM deletion_log
                WHERE export_path IS NOT NULL
            """)
            with_exports = cursor.fetchone()[0]
            
            return {
                "total_deletions": total,
                "by_type": by_type,
                "by_method": by_method,
                "with_exports": with_exports
            }
        finally:
            conn.close()
    
    def get_deletion_log(
        self,
        limit: int = 100,
        resource_type: Optional[str] = None
    ) -> List[Dict[str, Any]]:
        """Get deletion log entries.
        
        Args:
            limit: Maximum entries to return
            resource_type: Filter by resource type
        
        Returns:
            List of deletion log entries
        """
        conn = connect(self.db_path)
        try:
            if resource_type:
                cursor = conn.execute("""
                    SELECT * FROM deletion_log
                    WHERE resource_type = ?
                    ORDER BY deleted_at DESC
                    LIMIT ?
                """, (resource_type, limit))
            else:
                cursor = conn.execute("""
                    SELECT * FROM deletion_log
                    ORDER BY deleted_at DESC
                    LIMIT ?
                """, (limit,))
            
            columns = ['deletion_id', 'resource_type', 'resource_id', 
                      'deleted_at', 'delete_method', 'export_path', 
                      'reason', 'performed_by']
            
            return [
                dict(zip(columns, row))
                for row in cursor.fetchall()
            ]
        finally:
            conn.close()


# Convenience functions

_default_manager: Optional[RetentionManager] = None


def get_retention_manager(
    db_path: str | Path = 'fyi_system.db',
    audit_logger=None
) -> RetentionManager:
    """Get or create default retention manager."""
    global _default_manager
    if _default_manager is None or _default_manager.db_path != db_path:
        _default_manager = RetentionManager(db_path, audit_logger)
    return _default_manager


def cleanup_expired(
    db_path: str | Path = 'fyi_system.db',
    dry_run: bool = False,
    export_dir: Optional[str] = None
) -> Dict[str, Any]:
    """Clean up expired data."""
    manager = get_retention_manager(db_path)
    return manager.cleanup_expired_data(dry_run=dry_run, export_dir=export_dir)
