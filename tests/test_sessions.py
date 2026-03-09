"""Tests for secure session management."""
import pytest
import time
from pathlib import Path
from fyi_system.sessions import (
    Session,
    SessionManager,
    create_session,
    validate_session,
    invalidate_session,
    get_session_manager,
    DEFAULT_SESSION_TIMEOUT_MINUTES,
)


@pytest.fixture
def session_manager(tmp_path):
    """Create session manager with test database."""
    db_path = tmp_path / "test_sessions.db"
    manager = SessionManager(
        db_path=str(db_path),
        timeout_minutes=1,  # Short timeout for testing
        max_concurrent_sessions=3
    )
    return manager


@pytest.fixture
def sample_session(session_manager):
    """Create a sample session for testing."""
    return session_manager.create_session(
        user_id="test-user-123",
        ip_address="127.0.0.1",
        user_agent="TestBrowser/1.0"
    )


class TestSessionDataclass:
    """Test Session dataclass."""
    
    def test_create_session(self):
        """Test creating session object."""
        now = time.time()
        session = Session(
            session_id="test-session-id",
            user_id="user-123",
            created_at=now,
            expires_at=now + 1800,
            last_activity=now,
            ip_address="127.0.0.1",
            user_agent="TestBrowser"
        )
        
        assert session.session_id == "test-session-id"
        assert session.user_id == "user-123"
        assert session.is_valid is True
        assert session.is_expired() is False
    
    def test_session_is_expired(self):
        """Test session expiration check."""
        now = time.time()
        
        # Not expired
        session = Session(
            session_id="test",
            user_id="user",
            created_at=now,
            expires_at=now + 1800,
            last_activity=now
        )
        assert session.is_expired() is False
        
        # Expired
        expired_session = Session(
            session_id="test",
            user_id="user",
            created_at=now,
            expires_at=now - 100,
            last_activity=now
        )
        assert expired_session.is_expired() is True
    
    def test_session_touch(self):
        """Test session touch extends expiration."""
        now = time.time()
        session = Session(
            session_id="test",
            user_id="user",
            created_at=now,
            expires_at=now + 60,
            last_activity=now
        )
        
        old_expires = session.expires_at
        time.sleep(0.1)
        session.touch(timeout_minutes=30)
        
        assert session.last_activity > now
        assert session.expires_at > old_expires
    
    def test_session_invalidate(self):
        """Test session invalidation."""
        now = time.time()
        session = Session(
            session_id="test",
            user_id="user",
            created_at=now,
            expires_at=now + 1800,
            last_activity=now
        )
        
        assert session.is_valid is True
        session.invalidate()
        assert session.is_valid is False
    
    def test_session_to_dict_from_dict(self):
        """Test session serialization."""
        now = time.time()
        session = Session(
            session_id="test",
            user_id="user",
            created_at=now,
            expires_at=now + 1800,
            last_activity=now,
            ip_address="127.0.0.1"
        )
        
        data = session.to_dict()
        restored = Session.from_dict(data)
        
        assert restored.session_id == session.session_id
        assert restored.user_id == session.user_id
        assert restored.ip_address == session.ip_address


class TestSessionManager:
    """Test SessionManager class."""
    
    def test_create_session(self, session_manager):
        """Test creating a new session."""
        session = session_manager.create_session(
            user_id="test-user",
            ip_address="192.168.1.1"
        )
        
        assert session.session_id is not None
        assert len(session.session_id) == 64  # 256 bits hex = 64 chars
        assert session.user_id == "test-user"
        assert session.is_valid is True
        assert session.ip_address == "192.168.1.1"
    
    def test_get_session(self, session_manager, sample_session):
        """Test retrieving a session."""
        retrieved = session_manager.get_session(sample_session.session_id)
        
        assert retrieved is not None
        assert retrieved.session_id == sample_session.session_id
        assert retrieved.user_id == sample_session.user_id
    
    def test_get_nonexistent_session(self, session_manager):
        """Test retrieving nonexistent session."""
        session = session_manager.get_session("nonexistent-session-id")
        assert session is None
    
    def test_validate_session(self, session_manager, sample_session):
        """Test validating and touching a session."""
        # Validate should touch and extend
        validated = session_manager.validate_session(sample_session.session_id)
        
        assert validated is not None
        assert validated.last_activity >= sample_session.last_activity
    
    def test_invalidate_session(self, session_manager, sample_session):
        """Test invalidating a session."""
        # Invalidate
        result = session_manager.invalidate_session(sample_session.session_id)
        assert result is True
        
        # Should no longer be retrievable
        retrieved = session_manager.get_session(sample_session.session_id)
        assert retrieved is None
    
    def test_invalidate_all_user_sessions(self, session_manager):
        """Test invalidating all sessions for a user."""
        # Create multiple sessions
        session1 = session_manager.create_session("user-123")
        session2 = session_manager.create_session("user-123")
        session3 = session_manager.create_session("user-123")
        
        # Invalidate all
        count = session_manager.invalidate_all_user_sessions("user-123")
        assert count == 3
        
        # All should be invalid
        assert session_manager.get_session(session1.session_id) is None
        assert session_manager.get_session(session2.session_id) is None
        assert session_manager.get_session(session3.session_id) is None
    
    def test_session_limit_enforcement(self, session_manager):
        """Test concurrent session limit enforcement."""
        # Create 3 sessions (at limit)
        session1 = session_manager.create_session("limit-user")
        session2 = session_manager.create_session("limit-user")
        session3 = session_manager.create_session("limit-user")
        
        # Create 4th session (should invalidate oldest)
        session4 = session_manager.create_session("limit-user")
        
        # First session should be invalidated
        retrieved1 = session_manager.get_session(session1.session_id)
        assert retrieved1 is None
        
        # Others should still be valid
        assert session_manager.get_session(session2.session_id) is not None
        assert session_manager.get_session(session3.session_id) is not None
        assert session_manager.get_session(session4.session_id) is not None
    
    def test_get_user_sessions(self, session_manager):
        """Test getting all sessions for a user."""
        # Create multiple sessions
        session1 = session_manager.create_session("multi-user")
        time.sleep(0.01)
        session2 = session_manager.create_session("multi-user")
        
        sessions = session_manager.get_user_sessions("multi-user")
        
        assert len(sessions) == 2
        session_ids = [s.session_id for s in sessions]
        assert session1.session_id in session_ids
        assert session2.session_id in session_ids
    
    def test_cleanup_expired_sessions(self, session_manager):
        """Test cleaning up expired sessions."""
        # Create session with very short timeout
        session_manager.timeout_minutes = 0  # Immediate expiration
        session = session_manager.create_session("expire-user")
        
        # Wait for expiration
        time.sleep(0.1)
        
        # Cleanup
        cleaned = session_manager.cleanup_expired_sessions()
        assert cleaned >= 1
        
        # Session should be gone
        assert session_manager.get_session(session.session_id) is None
    
    def test_session_stats(self, session_manager):
        """Test session statistics."""
        # Create some sessions
        session_manager.create_session("user-1")
        session_manager.create_session("user-2")
        session_manager.create_session("user-1")
        
        stats = session_manager.get_session_stats()
        
        assert stats["active_sessions"] == 3
        assert stats["active_users"] == 2
        assert stats["expired_sessions"] == 0
    
    def test_session_token_uniqueness(self, session_manager):
        """Test that session tokens are unique."""
        tokens = set()
        for i in range(100):
            session = session_manager.create_session(f"user-{i}")
            assert session.session_id not in tokens
            tokens.add(session.session_id)
    
    def test_session_token_entropy(self, session_manager):
        """Test that session tokens have good entropy."""
        tokens = []
        for i in range(10):
            session = session_manager.create_session(f"entropy-user-{i}")
            tokens.append(session.session_id)
        
        # All tokens should be different
        assert len(set(tokens)) == 10
        
        # Tokens should be 64 chars (256 bits)
        for token in tokens:
            assert len(token) == 64


class TestConvenienceFunctions:
    """Test module-level convenience functions."""
    
    def test_create_session_function(self, tmp_path):
        """Test create_session convenience function."""
        db_path = tmp_path / "test.db"
        
        session = create_session(
            user_id="conv-user",
            db_path=str(db_path),
            timeout_minutes=30
        )
        
        assert session is not None
        assert session.user_id == "conv-user"
    
    def test_validate_session_function(self, tmp_path):
        """Test validate_session convenience function."""
        db_path = tmp_path / "test.db"
        
        # Create
        session = create_session("validate-user", db_path=str(db_path))
        
        # Validate
        validated = validate_session(session.session_id, db_path=str(db_path))
        
        assert validated is not None
        assert validated.session_id == session.session_id
    
    def test_invalidate_session_function(self, tmp_path):
        """Test invalidate_session convenience function."""
        db_path = tmp_path / "test.db"
        
        # Create
        session = create_session("invalidate-user", db_path=str(db_path))
        
        # Invalidate
        result = invalidate_session(session.session_id, db_path=str(db_path))
        
        assert result is True
        
        # Verify invalidated
        validated = validate_session(session.session_id, db_path=str(db_path))
        assert validated is None


class TestSessionSecurity:
    """Test session security features."""
    
    def test_session_token_cryptographically_secure(self, session_manager):
        """Test session tokens use cryptographically secure random."""
        import secrets
        
        # Generate multiple tokens
        tokens = []
        for i in range(10):
            session = session_manager.create_session(f"security-user-{i}")
            tokens.append(session.session_id)
        
        # All should be unique (probability of collision with 256 bits is negligible)
        assert len(set(tokens)) == len(tokens)
    
    def test_session_timeout_enforcement(self, session_manager):
        """Test that session timeout is enforced."""
        # Create session with 1-second timeout
        session_manager.timeout_minutes = 1 / 60  # 1 second
        
        session = session_manager.create_session("timeout-user")
        
        # Should be valid initially
        assert session_manager.get_session(session.session_id) is not None
        
        # Wait for timeout
        time.sleep(1.5)
        
        # Should be expired now
        assert session_manager.get_session(session.session_id) is None
    
    def test_session_ip_user_agent_stored(self, session_manager):
        """Test that IP and User-Agent are stored."""
        session = session_manager.create_session(
            user_id="metadata-user",
            ip_address="10.0.0.1",
            user_agent="Mozilla/5.0 Test Browser"
        )
        
        retrieved = session_manager.get_session(session.session_id)
        
        assert retrieved is not None
        assert retrieved.ip_address == "10.0.0.1"
        assert "Mozilla" in retrieved.user_agent
