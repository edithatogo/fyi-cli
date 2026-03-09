"""Secure session management for web application.

This module provides secure session handling with timeout, invalidation, and
cryptographically secure token generation.
"""
from __future__ import annotations
import secrets
import time
import json
from pathlib import Path
from typing import Optional, Dict, Any
from dataclasses import dataclass, asdict
from datetime import datetime, timedelta
import sqlite3
from .db import connect


# Session configuration defaults
DEFAULT_SESSION_TIMEOUT_MINUTES = 30
DEFAULT_MAX_CONCURRENT_SESSIONS = 5
SESSION_TOKEN_LENGTH = 32  # 256 bits


@dataclass
class Session:
    """User session data."""
    session_id: str
    user_id: str
    created_at: float
    expires_at: float
    last_activity: float
    ip_address: Optional[str] = None
    user_agent: Optional[str] = None
    is_valid: bool = True
    
    def to_dict(self) -> dict:
        """Convert to dictionary."""
        return asdict(self)
    
    @classmethod
    def from_dict(cls, data: dict) -> Session:
        """Create from dictionary."""
        return cls(**data)
    
    def is_expired(self) -> bool:
        """Check if session has expired."""
        return time.time() > self.expires_at
    
    def touch(self, timeout_minutes: int = DEFAULT_SESSION_TIMEOUT_MINUTES) -> None:
        """Update last activity and extend expiration.
        
        Args:
            timeout_minutes: Session timeout in minutes
        """
        now = time.time()
        self.last_activity = now
        self.expires_at = now + (timeout_minutes * 60)
    
    def invalidate(self) -> None:
        """Invalidate this session."""
        self.is_valid = False


class SessionManager:
    """Manage user sessions securely.
    
    Features:
    - Cryptographically secure session tokens
    - Configurable timeout
    - Automatic expiration
    - Session invalidation
    - Concurrent session limits
    - IP/User-Agent binding (optional)
    """
    
    def __init__(
        self,
        db_path: str | Path = 'fyi_system.db',
        timeout_minutes: int = DEFAULT_SESSION_TIMEOUT_MINUTES,
        max_concurrent_sessions: int = DEFAULT_MAX_CONCURRENT_SESSIONS
    ):
        """Initialize session manager.
        
        Args:
            db_path: Path to SQLite database
            timeout_minutes: Session timeout in minutes
            max_concurrent_sessions: Maximum concurrent sessions per user
        """
        self.db_path = db_path
        self.timeout_minutes = timeout_minutes
        self.max_concurrent_sessions = max_concurrent_sessions
        self._ensure_session_table()
    
    def _ensure_session_table(self) -> None:
        """Create sessions table if it doesn't exist."""
        conn = connect(self.db_path)
        try:
            conn.execute("""
                CREATE TABLE IF NOT EXISTS sessions (
                    session_id TEXT PRIMARY KEY,
                    user_id TEXT NOT NULL,
                    created_at REAL NOT NULL,
                    expires_at REAL NOT NULL,
                    last_activity REAL NOT NULL,
                    ip_address TEXT,
                    user_agent TEXT,
                    is_valid INTEGER DEFAULT 1,
                    data TEXT
                )
            """)
            conn.execute("""
                CREATE INDEX IF NOT EXISTS idx_sessions_user_id 
                ON sessions(user_id)
            """)
            conn.execute("""
                CREATE INDEX IF NOT EXISTS idx_sessions_expires 
                ON sessions(expires_at)
            """)
            conn.commit()
        finally:
            conn.close()
    
    def _generate_session_token(self) -> str:
        """Generate cryptographically secure session token.
        
        Returns:
            64-character hex string (256 bits of entropy)
        """
        return secrets.token_hex(SESSION_TOKEN_LENGTH)
    
    def create_session(
        self,
        user_id: str,
        ip_address: Optional[str] = None,
        user_agent: Optional[str] = None
    ) -> Session:
        """Create a new session for a user.
        
        Args:
            user_id: User identifier
            ip_address: Client IP address (optional)
            user_agent: Client User-Agent header (optional)
        
        Returns:
            New Session object
        """
        now = time.time()
        
        # Enforce concurrent session limit
        self._enforce_session_limit(user_id)
        
        # Create session
        session = Session(
            session_id=self._generate_session_token(),
            user_id=user_id,
            created_at=now,
            expires_at=now + (self.timeout_minutes * 60),
            last_activity=now,
            ip_address=ip_address,
            user_agent=user_agent,
            is_valid=True
        )
        
        # Save to database
        self._save_session(session)
        
        return session
    
    def _enforce_session_limit(self, user_id: str) -> None:
        """Remove oldest sessions if limit exceeded.
        
        Args:
            user_id: User identifier
        """
        conn = connect(self.db_path)
        try:
            # Get active sessions for user, ordered by creation
            cursor = conn.execute("""
                SELECT session_id, created_at 
                FROM sessions 
                WHERE user_id = ? AND is_valid = 1
                ORDER BY created_at ASC
            """, (user_id,))
            
            sessions = cursor.fetchall()
            
            # Remove oldest sessions if over limit
            while len(sessions) >= self.max_concurrent_sessions:
                oldest_session_id = sessions[0][0]
                conn.execute("""
                    UPDATE sessions 
                    SET is_valid = 0 
                    WHERE session_id = ?
                """, (oldest_session_id,))
                sessions.pop(0)
            
            conn.commit()
        finally:
            conn.close()
    
    def _save_session(self, session: Session) -> None:
        """Save session to database.
        
        Args:
            session: Session object to save
        """
        conn = connect(self.db_path)
        try:
            conn.execute("""
                INSERT OR REPLACE INTO sessions 
                (session_id, user_id, created_at, expires_at, last_activity, 
                 ip_address, user_agent, is_valid)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """, (
                session.session_id,
                session.user_id,
                session.created_at,
                session.expires_at,
                session.last_activity,
                session.ip_address,
                session.user_agent,
                1 if session.is_valid else 0
            ))
            conn.commit()
        finally:
            conn.close()
    
    def get_session(self, session_id: str) -> Optional[Session]:
        """Retrieve session by ID.
        
        Args:
            session_id: Session identifier
        
        Returns:
            Session object or None if not found/invalid/expired
        """
        conn = connect(self.db_path)
        try:
            cursor = conn.execute("""
                SELECT session_id, user_id, created_at, expires_at, 
                       last_activity, ip_address, user_agent, is_valid
                FROM sessions
                WHERE session_id = ?
            """, (session_id,))
            
            row = cursor.fetchone()
            if not row:
                return None
            
            session = Session(
                session_id=row[0],
                user_id=row[1],
                created_at=row[2],
                expires_at=row[3],
                last_activity=row[4],
                ip_address=row[5],
                user_agent=row[6],
                is_valid=bool(row[7])
            )
            
            # Check if valid and not expired
            if not session.is_valid or session.is_expired():
                return None
            
            return session
        finally:
            conn.close()
    
    def validate_session(self, session_id: str) -> Optional[Session]:
        """Validate and touch session.
        
        Args:
            session_id: Session identifier
        
        Returns:
            Session object if valid, None otherwise
        """
        session = self.get_session(session_id)
        if session:
            # Touch session to extend expiration
            session.touch(self.timeout_minutes)
            self._save_session(session)
        return session
    
    def invalidate_session(self, session_id: str) -> bool:
        """Invalidate a session (logout).
        
        Args:
            session_id: Session identifier
        
        Returns:
            True if session was invalidated
        """
        conn = connect(self.db_path)
        try:
            cursor = conn.execute("""
                UPDATE sessions 
                SET is_valid = 0 
                WHERE session_id = ?
            """, (session_id,))
            conn.commit()
            return cursor.rowcount > 0
        finally:
            conn.close()
    
    def invalidate_all_user_sessions(self, user_id: str) -> int:
        """Invalidate all sessions for a user.
        
        Args:
            user_id: User identifier
        
        Returns:
            Number of sessions invalidated
        """
        conn = connect(self.db_path)
        try:
            cursor = conn.execute("""
                UPDATE sessions 
                SET is_valid = 0 
                WHERE user_id = ?
            """, (user_id,))
            conn.commit()
            return cursor.rowcount
        finally:
            conn.close()
    
    def cleanup_expired_sessions(self) -> int:
        """Remove expired sessions from database.
        
        Returns:
            Number of sessions cleaned up
        """
        conn = connect(self.db_path)
        try:
            now = time.time()
            cursor = conn.execute("""
                DELETE FROM sessions 
                WHERE expires_at < ? OR is_valid = 0
            """, (now,))
            conn.commit()
            return cursor.rowcount
        finally:
            conn.close()
    
    def get_user_sessions(self, user_id: str) -> list[Session]:
        """Get all active sessions for a user.
        
        Args:
            user_id: User identifier
        
        Returns:
            List of active Session objects
        """
        conn = connect(self.db_path)
        try:
            cursor = conn.execute("""
                SELECT session_id, user_id, created_at, expires_at, 
                       last_activity, ip_address, user_agent, is_valid
                FROM sessions
                WHERE user_id = ? AND is_valid = 1
                ORDER BY created_at DESC
            """, (user_id,))
            
            sessions = []
            for row in cursor.fetchall():
                session = Session(
                    session_id=row[0],
                    user_id=row[1],
                    created_at=row[2],
                    expires_at=row[3],
                    last_activity=row[4],
                    ip_address=row[5],
                    user_agent=row[6],
                    is_valid=bool(row[7])
                )
                if not session.is_expired():
                    sessions.append(session)
            
            return sessions
        finally:
            conn.close()
    
    def get_session_stats(self) -> dict:
        """Get session statistics.
        
        Returns:
            Dictionary with session statistics
        """
        conn = connect(self.db_path)
        try:
            now = time.time()
            
            # Total active sessions
            cursor = conn.execute("""
                SELECT COUNT(*) FROM sessions 
                WHERE is_valid = 1 AND expires_at > ?
            """, (now,))
            active_count = cursor.fetchone()[0]
            
            # Total users with active sessions
            cursor = conn.execute("""
                SELECT COUNT(DISTINCT user_id) FROM sessions 
                WHERE is_valid = 1 AND expires_at > ?
            """, (now,))
            user_count = cursor.fetchone()[0]
            
            # Expired but not cleaned
            cursor = conn.execute("""
                SELECT COUNT(*) FROM sessions 
                WHERE expires_at < ?
            """, (now,))
            expired_count = cursor.fetchone()[0]
            
            return {
                "active_sessions": active_count,
                "active_users": user_count,
                "expired_sessions": expired_count
            }
        finally:
            conn.close()


# Convenience functions for simple usage

_default_manager: Optional[SessionManager] = None


def get_session_manager(
    db_path: str | Path = 'fyi_system.db',
    timeout_minutes: int = DEFAULT_SESSION_TIMEOUT_MINUTES
) -> SessionManager:
    """Get or create default session manager.
    
    Args:
        db_path: Path to SQLite database
        timeout_minutes: Session timeout in minutes
    
    Returns:
        SessionManager instance
    """
    global _default_manager
    if _default_manager is None or _default_manager.db_path != db_path:
        _default_manager = SessionManager(db_path, timeout_minutes)
    return _default_manager


def create_session(
    user_id: str,
    db_path: str | Path = 'fyi_system.db',
    timeout_minutes: int = DEFAULT_SESSION_TIMEOUT_MINUTES,
    ip_address: Optional[str] = None,
    user_agent: Optional[str] = None
) -> Session:
    """Create a new session.
    
    Args:
        user_id: User identifier
        db_path: Database path
        timeout_minutes: Session timeout
        ip_address: Client IP
        user_agent: Client User-Agent
    
    Returns:
        New Session object
    """
    manager = get_session_manager(db_path, timeout_minutes)
    return manager.create_session(user_id, ip_address, user_agent)


def validate_session(
    session_id: str,
    db_path: str | Path = 'fyi_system.db',
    timeout_minutes: int = DEFAULT_SESSION_TIMEOUT_MINUTES
) -> Optional[Session]:
    """Validate and touch a session.
    
    Args:
        session_id: Session identifier
        db_path: Database path
        timeout_minutes: Session timeout
    
    Returns:
        Session if valid, None otherwise
    """
    manager = get_session_manager(db_path, timeout_minutes)
    return manager.validate_session(session_id)


def invalidate_session(
    session_id: str,
    db_path: str | Path = 'fyi_system.db'
) -> bool:
    """Invalidate a session.
    
    Args:
        session_id: Session identifier
        db_path: Database path
    
    Returns:
        True if invalidated
    """
    manager = get_session_manager(db_path)
    return manager.invalidate_session(session_id)
