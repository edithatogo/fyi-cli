"""Tests for audit logging system."""
import pytest
import time
import json
from pathlib import Path
from fyi_system.audit import (
    AuditLogger,
    AuditEventType,
    AuditEvent,
    get_audit_logger,
    log_auth_success,
    log_auth_failure,
    log_logout,
    log_data_access,
    log_permission_denied,
)


@pytest.fixture
def audit_logger(tmp_path):
    """Create audit logger with test database."""
    db_path = tmp_path / "test_audit.db"
    logger = AuditLogger(db_path=str(db_path))
    return logger


class TestAuditEvent:
    """Test AuditEvent dataclass."""
    
    def test_create_event(self):
        """Test creating audit event."""
        event = AuditEvent(
            event_id=1,
            event_type="auth.login.success",
            timestamp=time.time(),
            user_id="user-123",
            action="User login",
            result="success",
            ip_address="127.0.0.1",
            user_agent="TestBrowser",
            resource_type=None,
            resource_id=None,
            details=None,
            previous_hash="abc123",
            event_hash="def456"
        )
        
        assert event.event_id == 1
        assert event.event_type == "auth.login.success"
        assert event.user_id == "user-123"
        assert event.result == "success"
    
    def test_event_to_dict(self):
        """Test event serialization."""
        event = AuditEvent(
            event_id=1,
            event_type="test",
            timestamp=1234567890.0,
            user_id="user",
            action="test",
            result="success",
            ip_address=None,
            user_agent=None,
            resource_type=None,
            resource_id=None,
            details=None,
            previous_hash="prev",
            event_hash="hash"
        )
        
        data = event.to_dict()
        
        assert data["event_id"] == 1
        assert data["event_type"] == "test"
        assert data["user_id"] == "user"


class TestAuditLogger:
    """Test AuditLogger class."""
    
    def test_log_event(self, audit_logger):
        """Test logging an event."""
        event = audit_logger.log(
            event_type=AuditEventType.AUTH_LOGIN_SUCCESS,
            action="User login",
            result="success",
            user_id="user-123",
            ip_address="192.168.1.1"
        )
        
        assert event.event_id > 0
        assert event.event_type == "auth.login.success"
        assert event.user_id == "user-123"
        assert event.result == "success"
        assert event.ip_address == "192.168.1.1"
        assert event.previous_hash is not None
        assert event.event_hash is not None
    
    def test_log_auth_success(self, audit_logger):
        """Test logging successful authentication."""
        event = audit_logger.log_auth_success(
            user_id="auth-user",
            ip_address="10.0.0.1",
            details={"method": "password"}
        )
        
        assert event.event_type == "auth.login.success"
        assert event.result == "success"
        assert event.details["method"] == "password"
    
    def test_log_auth_failure(self, audit_logger):
        """Test logging failed authentication."""
        event = audit_logger.log_auth_failure(
            user_id="failed-user",
            reason="Invalid password"
        )
        
        assert event.event_type == "auth.login.failure"
        assert event.result == "failure"
        assert event.details["reason"] == "Invalid password"
    
    def test_log_logout(self, audit_logger):
        """Test logging logout."""
        event = audit_logger.log_logout(
            user_id="logout-user",
            ip_address="127.0.0.1"
        )
        
        assert event.event_type == "auth.logout"
        assert event.result == "success"
    
    def test_log_data_access(self, audit_logger):
        """Test logging data access."""
        event = audit_logger.log_data_access(
            user_id="data-user",
            resource_type="tracked_request",
            resource_id="123",
            action="view"
        )
        
        assert "data" in event.event_type
        assert event.resource_type == "tracked_request"
        assert event.resource_id == "123"
    
    def test_log_permission_denied(self, audit_logger):
        """Test logging permission denied."""
        event = audit_logger.log_permission_denied(
            user_id="denied-user",
            resource_type="admin_panel",
            resource_id="settings"
        )
        
        assert event.event_type == "security.permission.denied"
        assert event.result == "denied"
    
    def test_hash_chain_integrity(self, audit_logger):
        """Test that hash chain is maintained."""
        # Log multiple events
        event1 = audit_logger.log("type1", "action1", "success", user_id="user1")
        event2 = audit_logger.log("type2", "action2", "success", user_id="user2")
        event3 = audit_logger.log("type3", "action3", "success", user_id="user3")
        
        # Verify chain
        assert event2.previous_hash == event1.event_hash
        assert event3.previous_hash == event2.event_hash
    
    def test_verify_integrity(self, audit_logger):
        """Test integrity verification."""
        # Log some events
        audit_logger.log("type1", "action1", "success")
        audit_logger.log("type2", "action2", "success")
        audit_logger.log("type3", "action3", "failure")
        
        # Verify
        result = audit_logger.verify_integrity()
        
        assert result["valid"] is True
        assert result["total_events"] == 3
        assert len(result.get("broken_chains", [])) == 0
    
    def test_get_events(self, audit_logger):
        """Test querying events."""
        # Log events
        audit_logger.log_auth_success("user-1")
        audit_logger.log_auth_success("user-2")
        audit_logger.log_auth_failure("user-3")
        
        # Get all
        events = audit_logger.get_events(limit=10)
        assert len(events) == 3
        
        # Get by user
        user1_events = audit_logger.get_events(user_id="user-1")
        assert len(user1_events) == 1
        
        # Get by type
        failure_events = audit_logger.get_events(event_type="auth.login.failure")
        assert len(failure_events) == 1
    
    def test_get_events_pagination(self, audit_logger):
        """Test event pagination."""
        # Log 10 events
        for i in range(10):
            audit_logger.log("test", f"action-{i}", "success", user_id=f"user-{i}")
        
        # First page
        page1 = audit_logger.get_events(limit=5, offset=0)
        assert len(page1) == 5
        
        # Second page
        page2 = audit_logger.get_events(limit=5, offset=5)
        assert len(page2) == 5
    
    def test_get_events_time_filter(self, audit_logger):
        """Test time-based filtering."""
        now = time.time()
        
        # Log events
        audit_logger.log("type1", "action1", "success")
        time.sleep(0.1)
        mid_time = time.time()
        time.sleep(0.1)
        audit_logger.log("type2", "action2", "success")
        
        # Filter by time
        old_events = audit_logger.get_events(end_time=mid_time)
        assert len(old_events) == 1
        assert old_events[0].event_type == "type1"
        
        new_events = audit_logger.get_events(start_time=mid_time)
        assert len(new_events) == 1
        assert new_events[0].event_type == "type2"
    
    def test_export_events(self, audit_logger, tmp_path):
        """Test exporting events."""
        # Log events
        audit_logger.log_auth_success("user-1")
        audit_logger.log_auth_failure("user-2")
        
        # Export
        output_path = tmp_path / "audit_export.json"
        count = audit_logger.export_events(str(output_path))
        
        assert count == 2
        assert output_path.exists()
        
        # Verify export content
        export_data = json.loads(output_path.read_text())
        assert export_data["total_events"] == 2
        assert "events" in export_data
        assert "integrity" in export_data
    
    def test_get_stats(self, audit_logger):
        """Test getting statistics."""
        # Log various events
        audit_logger.log_auth_success("user-1")
        audit_logger.log_auth_success("user-2")
        audit_logger.log_auth_failure("user-3")
        audit_logger.log_data_access("user-1", "request", "123")
        
        stats = audit_logger.get_stats()
        
        assert stats["total_events"] == 4
        assert "auth.login.success" in stats["by_type"]
        assert stats["by_type"]["auth.login.success"] == 2
        assert "success" in stats["by_result"]
        assert stats["by_result"]["success"] == 3
    
    def test_empty_log_integrity(self, audit_logger):
        """Test integrity verification on empty log."""
        result = audit_logger.verify_integrity()
        
        assert result["valid"] is True
        assert result["total_events"] == 0
    
    def test_event_type_enum(self, audit_logger):
        """Test using event type enum."""
        event = audit_logger.log(
            event_type=AuditEventType.AUTH_LOGIN_SUCCESS,
            action="Login",
            result="success"
        )
        
        assert event.event_type == "auth.login.success"
    
    def test_event_type_string(self, audit_logger):
        """Test using event type string."""
        event = audit_logger.log(
            event_type="custom.event.type",
            action="Custom action",
            result="success"
        )
        
        assert event.event_type == "custom.event.type"
    
    def test_details_serialization(self, audit_logger):
        """Test details dictionary serialization."""
        details = {
            "nested": {"key": "value"},
            "list": [1, 2, 3],
            "boolean": True,
            "null": None
        }
        
        event = audit_logger.log(
            event_type="test",
            action="test",
            result="success",
            details=details
        )
        
        assert event.details == details


class TestConvenienceFunctions:
    """Test module-level convenience functions."""
    
    def test_log_auth_success_function(self, tmp_path):
        """Test log_auth_success function."""
        db_path = tmp_path / "test.db"
        
        event = log_auth_success("conv-user", db_path=str(db_path))
        
        assert event is not None
        assert event.event_type == "auth.login.success"
    
    def test_log_auth_failure_function(self, tmp_path):
        """Test log_auth_failure function."""
        db_path = tmp_path / "test.db"
        
        event = log_auth_failure("failed-user", db_path=str(db_path))
        
        assert event.event_type == "auth.login.failure"
    
    def test_log_logout_function(self, tmp_path):
        """Test log_logout function."""
        db_path = tmp_path / "test.db"
        
        event = log_logout("logout-user", db_path=str(db_path))
        
        assert event.event_type == "auth.logout"
    
    def test_log_data_access_function(self, tmp_path):
        """Test log_data_access function."""
        db_path = tmp_path / "test.db"
        
        event = log_data_access(
            "data-user",
            "request",
            "123",
            db_path=str(db_path)
        )
        
        assert "data" in event.event_type
    
    def test_log_permission_denied_function(self, tmp_path):
        """Test log_permission_denied function."""
        db_path = tmp_path / "test.db"
        
        event = log_permission_denied(
            "denied-user",
            "admin",
            "panel",
            db_path=str(db_path)
        )
        
        assert event.event_type == "security.permission.denied"


class TestAuditSecurity:
    """Test audit log security features."""
    
    def test_hash_uniqueness(self, audit_logger):
        """Test that each event has unique hash."""
        hashes = set()
        
        for i in range(10):
            event = audit_logger.log("test", f"action-{i}", "success")
            assert event.event_hash not in hashes
            hashes.add(event.event_hash)
    
    def test_hash_determinism(self, audit_logger):
        """Test that same data produces same hash."""
        # Log two identical events (different IDs)
        event1 = audit_logger.log(
            event_type="test",
            action="same action",
            result="success",
            user_id="user1"
        )
        
        event2 = audit_logger.log(
            event_type="test",
            action="same action",
            result="success",
            user_id="user1"
        )
        
        # Hashes should be different due to different timestamps
        assert event1.event_hash != event2.event_hash
    
    def test_append_only(self, audit_logger):
        """Test that log is append-only."""
        # Log events
        event1 = audit_logger.log("type1", "action1", "success")
        event2 = audit_logger.log("type2", "action2", "success")
        
        # Verify IDs are sequential
        assert event2.event_id == event1.event_id + 1
        
        # Get events and verify order
        events = audit_logger.get_events(limit=10)
        assert events[0].event_id == 2  # Most recent first
        assert events[1].event_id == 1
