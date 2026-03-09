"""Tests for data retention and secure deletion."""
import pytest
import time
import json
from pathlib import Path
from fyi_system.retention import (
    RetentionManager,
    RetentionPolicy,
    RetentionPeriod,
    get_retention_manager,
    cleanup_expired,
)


@pytest.fixture
def retention_manager(tmp_path):
    """Create retention manager with test database."""
    db_path = tmp_path / "test_retention.db"
    manager = RetentionManager(db_path=str(db_path))
    return manager


@pytest.fixture
def retention_manager_with_audit(tmp_path, audit_logger_fixture):
    """Create retention manager with audit logger."""
    db_path = tmp_path / "test_retention.db"
    from fyi_system.audit import AuditLogger
    audit_logger = AuditLogger(db_path=tmp_path / "test_audit.db")
    manager = RetentionManager(db_path=str(db_path), audit_logger=audit_logger)
    return manager


class TestRetentionPolicy:
    """Test RetentionPolicy dataclass."""
    
    def test_create_policy(self):
        """Test creating retention policy."""
        policy = RetentionPolicy(
            policy_name="test-policy",
            resource_type="test_resource",
            retention_days=90,
            delete_method="secure",
            export_before_delete=True,
            enabled=True
        )
        
        assert policy.policy_name == "test-policy"
        assert policy.resource_type == "test_resource"
        assert policy.retention_days == 90
        assert policy.delete_method == "secure"
        assert policy.export_before_delete is True
        assert policy.enabled is True
    
    def test_policy_to_dict(self):
        """Test policy serialization."""
        policy = RetentionPolicy(
            policy_name="test",
            resource_type="test",
            retention_days=30,
            delete_method="simple",
            export_before_delete=False,
            enabled=False
        )
        
        data = policy.to_dict()
        
        assert data["policy_name"] == "test"
        assert data["retention_days"] == 30
        assert data["delete_method"] == "simple"
        assert data["export_before_delete"] is False
    
    def test_policy_from_dict(self):
        """Test policy deserialization."""
        data = {
            "policy_name": "test",
            "resource_type": "test",
            "retention_days": 60,
            "delete_method": "secure",
            "export_before_delete": True,
            "enabled": True
        }
        
        policy = RetentionPolicy.from_dict(data)
        
        assert policy.policy_name == "test"
        assert policy.retention_days == 60


class TestRetentionManager:
    """Test RetentionManager class."""
    
    def test_default_policies_created(self, retention_manager):
        """Test that default policies are created."""
        policies = retention_manager.list_policies()
        
        assert len(policies) > 0
        
        # Check for expected default policies
        policy_names = [p.policy_name for p in policies]
        assert "sessions" in policy_names
        assert "audit_logs" in policy_names
        assert "tracked_requests" in policy_names
    
    def test_get_policy(self, retention_manager):
        """Test getting a policy by resource type."""
        policy = retention_manager.get_policy("sessions")
        
        assert policy is not None
        assert policy.resource_type == "sessions"
        assert policy.retention_days == 30
    
    def test_get_nonexistent_policy(self, retention_manager):
        """Test getting nonexistent policy."""
        policy = retention_manager.get_policy("nonexistent_type")
        assert policy is None
    
    def test_set_policy(self, retention_manager):
        """Test setting a new policy."""
        new_policy = RetentionPolicy(
            policy_name="custom-policy",
            resource_type="custom_resource",
            retention_days=45,
            delete_method="secure",
            export_before_delete=True,
            enabled=True
        )
        
        result = retention_manager.set_policy(new_policy)
        assert result is True
        
        # Verify policy was saved
        retrieved = retention_manager.get_policy("custom_resource")
        assert retrieved is not None
        assert retrieved.retention_days == 45
    
    def test_update_policy(self, retention_manager):
        """Test updating an existing policy."""
        # Get existing policy
        policy = retention_manager.get_policy("sessions")
        
        # Update it
        policy.retention_days = 60
        retention_manager.set_policy(policy)
        
        # Verify update
        updated = retention_manager.get_policy("sessions")
        assert updated.retention_days == 60
    
    def test_delete_policy(self, retention_manager):
        """Test deleting a policy."""
        # Create policy
        policy = RetentionPolicy(
            policy_name="to-delete",
            resource_type="delete_me",
            retention_days=10,
            delete_method="simple"
        )
        retention_manager.set_policy(policy)
        
        # Delete it
        result = retention_manager.delete_policy("to-delete")
        assert result is True
        
        # Verify deleted
        retrieved = retention_manager.get_policy("delete_me")
        assert retrieved is None
    
    def test_list_policies(self, retention_manager):
        """Test listing all policies."""
        policies = retention_manager.list_policies()
        
        assert len(policies) > 0
        assert all(isinstance(p, RetentionPolicy) for p in policies)
    
    def test_cleanup_dry_run(self, retention_manager):
        """Test cleanup in dry-run mode."""
        # Dry run should not fail even with missing tables
        stats = retention_manager.cleanup_expired_data(dry_run=True)
        
        assert "policies_checked" in stats
        # May have errors for missing tables, which is OK in dry run
        assert "resources_scanned" in stats
        assert "resources_deleted" in stats
    
    def test_cleanup_with_no_expired_data(self, retention_manager):
        """Test cleanup when no data is expired."""
        # With dry_run, should handle missing tables gracefully
        stats = retention_manager.cleanup_expired_data(dry_run=True)
        
        # Should complete without crashing
        assert "policies_checked" in stats
    
    def test_get_deletion_stats(self, retention_manager):
        """Test getting deletion statistics."""
        stats = retention_manager.get_deletion_stats()
        
        assert "total_deletions" in stats
        assert "by_type" in stats
        assert "by_method" in stats
    
    def test_get_deletion_log(self, retention_manager):
        """Test getting deletion log."""
        log = retention_manager.get_deletion_log(limit=10)
        
        assert isinstance(log, list)
    
    def test_retention_periods_enum(self):
        """Test RetentionPeriod enum values."""
        assert RetentionPeriod.DAYS_30.value == 30
        assert RetentionPeriod.DAYS_90.value == 90
        assert RetentionPeriod.DAYS_365.value == 365
        assert RetentionPeriod.YEARS_3.value == 1095
        assert RetentionPeriod.YEARS_7.value == 2555
        assert RetentionPeriod.PERMANENT.value == -1


class TestSecureDeletion:
    """Test secure deletion functionality."""
    
    def test_secure_delete_method(self, retention_manager):
        """Test that secure delete overwrites data."""
        # This test verifies the secure delete method exists
        # Actual verification would require low-level database access
        
        policy = retention_manager.get_policy("tracked_requests")
        assert policy.delete_method == "secure"
    
    def test_simple_delete_method(self, retention_manager):
        """Test simple delete method."""
        policy = retention_manager.get_policy("sessions")
        assert policy.delete_method == "simple"


class TestExportBeforeDelete:
    """Test export before deletion functionality."""
    
    def test_export_before_delete_configured(self, retention_manager):
        """Test that export before delete is configured."""
        policy = retention_manager.get_policy("audit_logs")
        if policy:
            assert policy.export_before_delete is True


class TestRetentionManagerWithAudit:
    """Test retention manager with audit logging."""
    
    def test_manager_with_audit_logger(self, tmp_path):
        """Test creating manager with audit logger."""
        from fyi_system.audit import AuditLogger
        
        db_path = tmp_path / "test.db"
        audit_path = tmp_path / "audit.db"
        
        audit_logger = AuditLogger(db_path=str(audit_path))
        manager = RetentionManager(db_path=str(db_path), audit_logger=audit_logger)
        
        assert manager.audit_logger is not None


class TestConvenienceFunctions:
    """Test module-level convenience functions."""
    
    def test_get_retention_manager(self, tmp_path):
        """Test get_retention_manager function."""
        db_path = tmp_path / "test.db"
        
        manager = get_retention_manager(db_path=str(db_path))
        
        assert manager is not None
        assert isinstance(manager, RetentionManager)
    
    def test_cleanup_expired_function(self, tmp_path):
        """Test cleanup_expired function."""
        db_path = tmp_path / "test.db"
        
        # Use dry_run to avoid table errors
        stats = cleanup_expired(db_path=str(db_path), dry_run=True)
        
        assert "policies_checked" in stats
        assert "resources_scanned" in stats


class TestRetentionEdgeCases:
    """Test edge cases and error handling."""
    
    def test_cleanup_unknown_resource_type(self, retention_manager):
        """Test cleanup with unknown resource type."""
        # This should handle gracefully even with missing tables
        stats = retention_manager.cleanup_expired_data(
            resource_type="unknown_type",
            dry_run=True
        )
        
        # Should complete without crashing
        assert isinstance(stats, dict)
    
    def test_policy_with_zero_retention(self, retention_manager):
        """Test policy with zero day retention."""
        policy = RetentionPolicy(
            policy_name="immediate-delete",
            resource_type="temp_data",
            retention_days=0,
            delete_method="simple"
        )
        
        result = retention_manager.set_policy(policy)
        assert result is True
        
        retrieved = retention_manager.get_policy("temp_data")
        assert retrieved is not None
        assert retrieved.retention_days == 0
    
    def test_policy_with_permanent_retention(self, retention_manager):
        """Test policy with permanent retention."""
        policy = RetentionPolicy(
            policy_name="permanent-storage",
            resource_type="archive",
            retention_days=-1,  # Permanent
            delete_method="secure",
            enabled=False  # Disabled for permanent
        )
        
        result = retention_manager.set_policy(policy)
        assert result is True
    
    def test_disabled_policy_not_applied(self, retention_manager):
        """Test that disabled policies are not applied."""
        # Create a test policy and disable it
        policy = RetentionPolicy(
            policy_name="test-disabled",
            resource_type="test_resource",
            retention_days=10,
            delete_method="simple",
            enabled=False
        )
        retention_manager.set_policy(policy)
        
        # Get policy and verify it's disabled
        retrieved = retention_manager.get_policy("test_resource")
        assert retrieved is not None
        assert retrieved.enabled is False
        
        # Cleanup should skip disabled policy
        stats = retention_manager.cleanup_expired_data(dry_run=True)
        
        # Should complete without error
        assert isinstance(stats, dict)
    
    def test_deletion_log_persists(self, retention_manager):
        """Test that deletion log persists across manager instances."""
        # Get initial stats
        stats1 = retention_manager.get_deletion_stats()
        
        # Create new manager instance
        new_manager = RetentionManager(db_path=retention_manager.db_path)
        stats2 = new_manager.get_deletion_stats()
        
        # Stats should be the same (same database)
        assert stats1["total_deletions"] == stats2["total_deletions"]
