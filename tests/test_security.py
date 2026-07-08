"""Tests for security module - privacy, redaction, and secure file operations."""
import json
import os
import tempfile
from pathlib import Path
import pytest
from fyi_system.security import (
    PrivacySettings,
    load_settings,
    ensure_private_path,
    sanitize_payload,
    secure_write_text,
    redact_text,
)


class TestPrivacySettings:
    """Test PrivacySettings dataclass."""
    
    def test_default_settings(self):
        """Test default privacy settings."""
        settings = PrivacySettings()
        assert settings.profile == 'standard'
        assert settings.bind_host == '127.0.0.1'
        assert settings.redact_logs is True
        assert settings.sanitize_bundle_exports is True
        assert settings.file_mode == 0o600
        assert settings.dir_mode == 0o700
    
    def test_custom_settings(self):
        """Test custom privacy settings."""
        settings = PrivacySettings(
            profile='strict',
            bind_host='0.0.0.0',
            redact_logs=False,
            sanitize_bundle_exports=False,
        )
        assert settings.profile == 'strict'
        assert settings.bind_host == '0.0.0.0'
        assert settings.redact_logs is False
        assert settings.sanitize_bundle_exports is False


class TestLoadSettings:
    """Test settings loading functionality."""
    
    def test_load_default_settings(self, tmp_path):
        """Test loading default settings when no file exists."""
        settings_path = tmp_path / "nonexistent.json"
        settings = load_settings(settings_path)
        assert settings.profile == 'standard'
        assert settings.bind_host == '127.0.0.1'
    
    def test_load_settings_from_file(self, tmp_path):
        """Test loading settings from JSON file."""
        settings_path = tmp_path / "settings.json"
        settings_path.write_text('{"profile": "strict", "bind_host": "0.0.0.0"}')
        settings = load_settings(settings_path)
        assert settings.profile == 'strict'
        assert settings.bind_host == '0.0.0.0'
    
    def test_load_settings_with_env_override(self, tmp_path, monkeypatch):
        """Test environment variables override file settings."""
        settings_path = tmp_path / "settings.json"
        settings_path.write_text('{"profile": "standard"}')
        
        monkeypatch.setenv('FYI_SYSTEM_PRIVACY_PROFILE', 'strict')
        monkeypatch.setenv('FYI_SYSTEM_BIND_HOST', '0.0.0.0')
        
        settings = load_settings(settings_path)
        assert settings.profile == 'strict'
        assert settings.bind_host == '0.0.0.0'
    
    def test_load_settings_from_path_object(self, tmp_path):
        """Test loading settings with Path object."""
        settings_path = tmp_path / "settings.json"
        settings_path.write_text('{"profile": "strict"}')
        settings = load_settings(settings_path)
        assert settings.profile == 'strict'


class TestEnsurePrivatePath:
    """Test private path creation and permissions."""
    
    def test_ensure_private_path_creates_directory(self, tmp_path):
        """Test that ensure_private_path creates directories."""
        private_dir = tmp_path / "private"
        result = ensure_private_path(private_dir, is_dir=True)
        assert result.exists()
        assert result.is_dir()
    
    def test_ensure_private_path_creates_file_parent(self, tmp_path):
        """Test that ensure_private_path creates parent directories for files."""
        private_file = tmp_path / "private" / "subdir" / "file.txt"
        result = ensure_private_path(private_file, is_dir=False)
        assert result.parent.exists()
    
    def test_ensure_private_path_with_path_object(self, tmp_path):
        """Test ensure_private_path accepts Path objects."""
        private_dir = tmp_path / "private"
        result = ensure_private_path(private_dir, is_dir=True)
        assert isinstance(result, Path)
    
    def test_ensure_private_path_string_path(self, tmp_path):
        """Test ensure_private_path accepts string paths."""
        private_dir = str(tmp_path / "private")
        result = ensure_private_path(private_dir, is_dir=True)
        assert result.exists()


class TestSanitizePayload:
    """Test payload sanitization for sensitive data."""
    
    def test_sanitize_payload_redacts_email(self):
        """Test that sanitize_payload redacts email addresses."""
        payload = {'email': 'test@example.com', 'name': 'Test'}
        result = sanitize_payload(payload)
        # Email should be redacted via redact_text
        assert result['email'] != 'test@example.com'
        assert 'redacted' in result['email'].lower()
        assert result['name'] == 'Test'
    
    def test_sanitize_payload_redacts_body_strict_profile(self):
        """Test that sanitize_payload redacts body field in strict mode."""
        payload = {'body': 'sensitive content', 'title': 'Public'}
        result = sanitize_payload(payload, profile='strict')
        # Body should be redacted in strict mode
        assert result['body'] != 'sensitive content'
        assert 'redacted' in result['body'].lower()
        assert result['title'] == 'Public'
    
    def test_sanitize_payload_preserves_body_standard_profile(self):
        """Test that sanitize_payload preserves body in standard mode."""
        payload = {'body': 'sensitive content', 'title': 'Public'}
        result = sanitize_payload(payload, profile='standard')
        # Body is preserved in standard mode (only email redacted)
        assert result['body'] == 'sensitive content'
        assert result['title'] == 'Public'
    
    def test_sanitize_payload_preserves_structure(self):
        """Test that sanitize_payload preserves dict structure."""
        payload = {'key1': 'value1', 'key2': 'value2'}
        result = sanitize_payload(payload)
        assert isinstance(result, dict)
        assert 'key1' in result
        assert 'key2' in result
    
    def test_sanitize_payload_handles_none(self):
        """Test sanitize_payload handles None values."""
        payload = {'key': None}
        result = sanitize_payload(payload)
        assert result['key'] is None
    
    def test_sanitize_payload_handles_nested_dict(self):
        """Test sanitize_payload handles nested dictionaries."""
        payload = {
            'outer': {
                'email': 'nested@example.com',
                'safe': 'value'
            }
        }
        result = sanitize_payload(payload)
        assert 'email' in result['outer']
        assert result['outer']['safe'] == 'value'


class TestSecureWriteText:
    """Test secure file writing functionality."""
    
    def test_secure_write_text_creates_file(self, tmp_path):
        """Test that secure_write_text creates files."""
        file_path = tmp_path / "secure.txt"
        content = "test content"
        secure_write_text(file_path, content)
        assert file_path.exists()
        assert file_path.read_text() == content
    
    def test_secure_write_text_creates_parent_dirs(self, tmp_path):
        """Test that secure_write_text creates parent directories."""
        file_path = tmp_path / "subdir" / "secure.txt"
        content = "test content"
        secure_write_text(file_path, content)
        assert file_path.parent.exists()
        assert file_path.read_text() == content
    
    def test_secure_write_text_with_path_object(self, tmp_path):
        """Test secure_write_text accepts Path objects."""
        file_path = tmp_path / "secure.txt"
        content = "test content"
        secure_write_text(file_path, content)
        assert file_path.exists()
    
    def test_secure_write_text_with_string_path(self, tmp_path):
        """Test secure_write_text accepts string paths."""
        file_path = str(tmp_path / "secure.txt")
        content = "test content"
        secure_write_text(file_path, content)
        assert Path(file_path).exists()
    
    def test_secure_write_text_unicode(self, tmp_path):
        """Test secure_write_text handles unicode content."""
        file_path = tmp_path / "unicode.txt"
        content = "Hello 世界 🌍"
        secure_write_text(file_path, content)
        assert file_path.read_text(encoding='utf-8') == content


class TestRedactText:
    """Test text redaction functionality."""
    
    def test_redact_text_redacts_urls(self):
        """Test that redact_text redacts URL query secrets."""
        # redact_text doesn't redact full URLs, only query secrets
        text = "Visit https://example.com?api_key=secret123 for more info"
        result = redact_text(text)
        # Query secret should be redacted (URL-encoded)
        assert 'secret123' not in result
        assert 'redacted' in result.lower()
    
    def test_redact_text_preserves_urls_without_secrets(self):
        """Test that redact_text preserves URLs without secrets."""
        text = "Visit https://example.com for more info"
        result = redact_text(text)
        # Plain URL should be preserved exactly.
        assert result == text
    
    def test_redact_text_redacts_emails(self):
        """Test that redact_text redacts email addresses."""
        text = "Contact test@example.com for help"
        result = redact_text(text)
        # Email should be redacted
        assert 'test@example.com' not in result
        assert '[redacted-email]' in result
    
    def test_redact_text_redacts_bearer_tokens(self):
        """Test that redact_text redacts bearer tokens."""
        text = "Authorization: Bearer abc123xyz"
        result = redact_text(text)
        # Token should be redacted
        assert 'abc123xyz' not in result
        assert '[redacted-token]' in result
    
    def test_redact_text_preserves_plain_text(self):
        """Test that redact_text preserves plain text."""
        text = "This is plain text with no sensitive data"
        result = redact_text(text)
        assert result == text
    
    def test_redact_text_multiple_redactions(self):
        """Test redact_text handles multiple sensitive items."""
        text = "Email test@example.com with Authorization: Bearer secret456"
        result = redact_text(text)
        # Both should be redacted
        assert 'test@example.com' not in result
        assert 'secret456' not in result
        assert '[redacted-email]' in result
        assert '[redacted-token]' in result
