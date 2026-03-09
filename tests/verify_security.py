#!/usr/bin/env python3
"""Security verification tests for FYI Request System.

This script runs comprehensive security verification tests including:
- Encryption verification
- Credential storage verification
- Session security verification
- Audit log integrity verification
- Input validation verification
- Security header verification

Run with: python tests/verify_security.py
"""
import sys
import time
import tempfile
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from fyi_system.encryption import (
    setup_encryption,
    encrypt_data,
    decrypt_data,
    get_master_key_from_keyring,
    delete_master_key_from_keyring,
    KEYRING_SERVICE_NAME,
)
from fyi_system.credentials import (
    CredentialManager,
    FYICredentials,
    save_fyi_credentials,
    get_fyi_credentials,
    CREDENTIAL_KEYRING_SERVICE,
)
from fyi_system.sessions import SessionManager
from fyi_system.audit import AuditLogger, get_audit_logger
from fyi_system.retention import RetentionManager, cleanup_expired
from fyi_system.security_middleware import (
    CSRFProtection,
    InputValidator,
    SecurityHeaders,
    generate_csrf_token,
    validate_csrf_token,
)


class SecurityVerifier:
    """Run security verification tests."""
    
    def __init__(self):
        self.results = []
        self.temp_dir = tempfile.mkdtemp()
    
    def verify_encryption(self) -> bool:
        """Verify encryption is working correctly."""
        print("\n[1/6] Verifying encryption...")
        
        try:
            # Setup
            password = "test-verification-password-123!"
            setup_encryption(password, KEYRING_SERVICE_NAME)
            
            # Test encryption/decryption
            plaintext = "Sensitive data for testing"
            key = get_master_key_from_keyring(KEYRING_SERVICE_NAME)
            
            encrypted = encrypt_data(plaintext, key)
            decrypted = decrypt_data(encrypted, key)
            
            assert decrypted == plaintext, "Decryption failed"
            assert encrypted != plaintext, "Encryption not working"
            
            # Cleanup
            delete_master_key_from_keyring(KEYRING_SERVICE_NAME)
            
            print("  ✓ Encryption verification passed")
            self.results.append(("Encryption", True))
            return True
            
        except Exception as e:
            print(f"  ✗ Encryption verification failed: {e}")
            self.results.append(("Encryption", False))
            return False
    
    def verify_credentials(self) -> bool:
        """Verify credential storage is working."""
        print("\n[2/6] Verifying credential storage...")
        
        try:
            # Setup encryption first
            password = "test-verification-password-123!"
            setup_encryption(password, KEYRING_SERVICE_NAME)
            
            # Test credential storage
            manager = CredentialManager(CREDENTIAL_KEYRING_SERVICE)
            
            creds = FYICredentials(
                account_id="test-verify",
                email="test@example.com",
                api_token="test-token-123",
                base_url="https://fyi.org.nz"
            )
            
            # Save
            assert manager.save_credentials(creds), "Failed to save credentials"
            
            # Retrieve
            retrieved = manager.get_credentials("test-verify")
            assert retrieved is not None, "Failed to retrieve credentials"
            assert retrieved.email == "test@example.com", "Email mismatch"
            assert retrieved.api_token == "test-token-123", "Token mismatch"
            
            # Cleanup
            manager.delete_credentials("test-verify")
            delete_master_key_from_keyring(KEYRING_SERVICE_NAME)
            
            print("  ✓ Credential storage verification passed")
            self.results.append(("Credential Storage", True))
            return True
            
        except Exception as e:
            print(f"  ✗ Credential storage verification failed: {e}")
            self.results.append(("Credential Storage", False))
            return False
    
    def verify_sessions(self) -> bool:
        """Verify session management is working."""
        print("\n[3/6] Verifying session management...")
        
        try:
            db_path = Path(self.temp_dir) / "test_sessions.db"
            manager = SessionManager(db_path=str(db_path), timeout_minutes=1)
            
            # Create session
            session = manager.create_session(
                user_id="test-user",
                ip_address="127.0.0.1"
            )
            
            assert session.session_id, "Session ID not generated"
            assert len(session.session_id) == 64, "Session ID wrong length"
            
            # Validate session
            validated = manager.validate_session(session.session_id)
            assert validated is not None, "Session validation failed"
            
            # Test invalidation
            manager.invalidate_session(session.session_id)
            invalidated = manager.get_session(session.session_id)
            assert invalidated is None, "Session should be invalidated"
            
            # Cleanup
            db_path.unlink(missing_ok=True)
            
            print("  ✓ Session management verification passed")
            self.results.append(("Session Management", True))
            return True
            
        except Exception as e:
            print(f"  ✗ Session management verification failed: {e}")
            self.results.append(("Session Management", False))
            return False
    
    def verify_audit_log(self) -> bool:
        """Verify audit logging is working."""
        print("\n[4/6] Verifying audit logging...")
        
        try:
            db_path = Path(self.temp_dir) / "test_audit.db"
            logger = AuditLogger(db_path=str(db_path))
            
            # Log events
            logger.log_auth_success("test-user", ip_address="127.0.0.1")
            logger.log_data_access("test-user", "request", "123")
            logger.log_permission_denied("test-user", "admin", "settings")
            
            # Verify integrity
            result = logger.verify_integrity()
            assert result["valid"], f"Audit log integrity failed: {result}"
            
            # Query events
            events = logger.get_events(limit=10)
            assert len(events) == 3, "Wrong number of events"
            
            # Cleanup
            db_path.unlink(missing_ok=True)
            
            print("  ✓ Audit logging verification passed")
            self.results.append(("Audit Logging", True))
            return True
            
        except Exception as e:
            print(f"  ✗ Audit logging verification failed: {e}")
            self.results.append(("Audit Logging", False))
            return False
    
    def verify_retention(self) -> bool:
        """Verify data retention is working."""
        print("\n[5/6] Verifying data retention...")
        
        try:
            db_path = Path(self.temp_dir) / "test_retention.db"
            manager = RetentionManager(db_path=str(db_path))
            
            # List policies
            policies = manager.list_policies()
            assert len(policies) > 0, "No retention policies found"
            
            # Test dry-run cleanup
            stats = manager.cleanup_expired_data(dry_run=True)
            assert "policies_checked" in stats, "Missing stats"
            
            # Get stats
            deletion_stats = manager.get_deletion_stats()
            assert "total_deletions" in deletion_stats, "Missing deletion stats"
            
            # Cleanup
            db_path.unlink(missing_ok=True)
            
            print("  ✓ Data retention verification passed")
            self.results.append(("Data Retention", True))
            return True
            
        except Exception as e:
            print(f"  ✗ Data retention verification failed: {e}")
            self.results.append(("Data Retention", False))
            return False
    
    def verify_security_middleware(self) -> bool:
        """Verify security middleware is working."""
        print("\n[6/6] Verifying security middleware...")
        
        try:
            # CSRF protection
            csrf = CSRFProtection()
            token1 = csrf.generate_token()
            token2 = csrf.generate_token()
            
            assert token1 != token2, "CSRF tokens not unique"
            assert csrf.validate_token(token1, token1), "Token validation failed"
            assert not csrf.validate_token(token1, token2), "Invalid token accepted"
            
            # Input validation
            assert InputValidator.validate_email("test@example.com"), "Valid email rejected"
            assert not InputValidator.validate_email("invalid"), "Invalid email accepted"
            
            assert InputValidator.validate_url("https://example.com"), "Valid URL rejected"
            assert not InputValidator.validate_url("javascript:alert(1)"), "Invalid URL accepted"
            
            # HTML sanitization
            sanitized = InputValidator.sanitize_html("<script>alert(1)</script>")
            assert "<script>" not in sanitized, "HTML not sanitized"
            
            # Security headers
            headers = SecurityHeaders.get_all_headers()
            assert "Content-Security-Policy" in headers, "Missing CSP"
            assert "X-Frame-Options" in headers, "Missing X-Frame-Options"
            assert "Strict-Transport-Security" in headers, "Missing HSTS"
            
            print("  ✓ Security middleware verification passed")
            self.results.append(("Security Middleware", True))
            return True
            
        except Exception as e:
            print(f"  ✗ Security middleware verification failed: {e}")
            self.results.append(("Security Middleware", False))
            return False
    
    def run_all_verifications(self) -> bool:
        """Run all security verifications."""
        print("=" * 60)
        print("FYI Request System - Security Verification")
        print("=" * 60)
        
        # Run all verifications
        self.verify_encryption()
        self.verify_credentials()
        self.verify_sessions()
        self.verify_audit_log()
        self.verify_retention()
        self.verify_security_middleware()
        
        # Summary
        print("\n" + "=" * 60)
        print("VERIFICATION SUMMARY")
        print("=" * 60)
        
        passed = sum(1 for _, result in self.results if result)
        total = len(self.results)
        
        for name, result in self.results:
            status = "✓ PASS" if result else "✗ FAIL"
            print(f"  {status}: {name}")
        
        print("=" * 60)
        print(f"Total: {passed}/{total} verifications passed")
        print("=" * 60)
        
        # Cleanup
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)
        
        return passed == total


def main():
    """Main entry point."""
    verifier = SecurityVerifier()
    success = verifier.run_all_verifications()
    
    if success:
        print("\n✓ All security verifications passed!")
        return 0
    else:
        print("\n✗ Some security verifications failed!")
        return 1


if __name__ == "__main__":
    sys.exit(main())
