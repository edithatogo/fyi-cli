"""Tests for encryption utilities."""
import pytest
from pathlib import Path
import keyring
from fyi_system.encryption import (
    generate_salt,
    derive_key,
    encrypt_data,
    decrypt_data,
    generate_master_key,
    get_master_key_from_keyring,
    set_master_key_in_keyring,
    delete_master_key_from_keyring,
    setup_encryption,
    verify_password,
    export_key_backup,
    import_key_backup,
    EncryptedField,
    KEYRING_SERVICE_NAME,
    KEYRING_USERNAME,
)


class TestEncryptionBasics:
    """Test basic encryption/decryption functions."""
    
    def test_generate_salt(self):
        """Test salt generation."""
        salt1 = generate_salt()
        salt2 = generate_salt()
        
        assert len(salt1) == 16  # 128 bits
        assert len(salt2) == 16
        assert salt1 != salt2  # Should be unique
    
    def test_derive_key(self):
        """Test key derivation."""
        salt = generate_salt()
        key1 = derive_key("password123", salt)
        key2 = derive_key("password123", salt)
        key3 = derive_key("password456", salt)
        
        assert len(key1) == 32  # 256 bits for AES-256
        assert key1 == key2  # Same password + salt = same key
        assert key1 != key3  # Different password = different key
    
    def test_encrypt_decrypt_round_trip(self):
        """Test encryption and decryption round trip."""
        key = generate_master_key()
        plaintext = "Hello, World! This is a test message."
        
        encrypted = encrypt_data(plaintext, key)
        decrypted = decrypt_data(encrypted, key)
        
        assert decrypted == plaintext
        assert encrypted != plaintext  # Should be different
        assert len(encrypted) > len(plaintext)  # Should be longer (salt + nonce + tag)
    
    def test_encrypt_different_ciphertext_each_time(self):
        """Test encryption produces different ciphertext each time."""
        key = generate_master_key()
        plaintext = "Same message"
        
        encrypted1 = encrypt_data(plaintext, key)
        encrypted2 = encrypt_data(plaintext, key)
        
        assert encrypted1 != encrypted2  # Different due to random salt/nonce
        
        # But both should decrypt to same plaintext
        assert decrypt_data(encrypted1, key) == plaintext
        assert decrypt_data(encrypted2, key) == plaintext
    
    def test_decrypt_wrong_key_fails(self):
        """Test decryption with wrong key fails."""
        key1 = generate_master_key()
        key2 = generate_master_key()
        plaintext = "Secret message"
        
        encrypted = encrypt_data(plaintext, key1)
        
        with pytest.raises(ValueError, match="Decryption failed"):
            decrypt_data(encrypted, key2)
    
    def test_decrypt_corrupted_data_fails(self):
        """Test decryption of corrupted data fails."""
        key = generate_master_key()
        plaintext = "Test message"
        
        encrypted = encrypt_data(plaintext, key)
        corrupted = encrypted[:-5] + "XXXXX"  # Corrupt last 5 chars
        
        with pytest.raises(ValueError):
            decrypt_data(corrupted, key)
    
    def test_encrypt_empty_string(self):
        """Test encryption of empty string."""
        key = generate_master_key()
        
        encrypted = encrypt_data("", key)
        decrypted = decrypt_data(encrypted, key)
        
        assert decrypted == ""
    
    def test_encrypt_unicode(self):
        """Test encryption of unicode characters."""
        key = generate_master_key()
        plaintext = "Hello 世界！🌍 Привет!"
        
        encrypted = encrypt_data(plaintext, key)
        decrypted = decrypt_data(encrypted, key)
        
        assert decrypted == plaintext


class TestKeyringIntegration:
    """Test OS keyring integration."""
    
    def test_set_get_delete_key(self):
        """Test setting, getting, and deleting key from keyring."""
        key = generate_master_key()
        
        # Set key
        set_master_key_in_keyring(key, KEYRING_SERVICE_NAME)
        
        # Get key
        retrieved = get_master_key_from_keyring(KEYRING_SERVICE_NAME)
        assert retrieved == key
        
        # Delete key
        delete_master_key_from_keyring(KEYRING_SERVICE_NAME)
        
        # Verify deleted
        retrieved = get_master_key_from_keyring(KEYRING_SERVICE_NAME)
        assert retrieved is None
    
    def test_get_nonexistent_key(self):
        """Test getting key that doesn't exist."""
        # Clean up first
        delete_master_key_from_keyring(KEYRING_SERVICE_NAME)
        
        retrieved = get_master_key_from_keyring(KEYRING_SERVICE_NAME)
        assert retrieved is None


class TestEncryptedField:
    """Test EncryptedField descriptor."""
    
    def test_encrypted_field_set_get(self):
        """Test setting and getting encrypted field."""
        from fyi_system.encryption import EncryptedField
        
        # Setup encryption
        key = generate_master_key()
        set_master_key_in_keyring(key, KEYRING_SERVICE_NAME)
        
        class TestModel:
            secret = EncryptedField('secret')
            
            def __init__(self):
                self._encrypted_secret = None
        
        model = TestModel()
        
        # Set value
        model.secret = "My secret value"
        
        # Internal storage should be encrypted
        assert model._encrypted_secret.startswith('enc:')
        
        # Get value should decrypt
        assert model.secret == "My secret value"
        
        # Cleanup
        delete_master_key_from_keyring(KEYRING_SERVICE_NAME)
    
    def test_encrypted_field_none_value(self):
        """Test encrypted field with None value."""
        from fyi_system.encryption import EncryptedField
        
        key = generate_master_key()
        set_master_key_in_keyring(key, KEYRING_SERVICE_NAME)
        
        class TestModel:
            secret = EncryptedField('secret')
            
            def __init__(self):
                self._encrypted_secret = None
        
        model = TestModel()
        model.secret = None
        
        assert model._encrypted_secret is None
        assert model.secret is None
        
        # Cleanup
        delete_master_key_from_keyring(KEYRING_SERVICE_NAME)


class TestSetupEncryption:
    """Test encryption setup workflow."""
    
    def test_setup_and_verify(self):
        """Test setting up encryption and verifying password."""
        password = "test-password-123"
        
        # Setup
        result = setup_encryption(password, KEYRING_SERVICE_NAME)
        assert result is True
        
        # Verify password works
        assert verify_password(password, KEYRING_SERVICE_NAME) is True
        assert verify_password("wrong-password", KEYRING_SERVICE_NAME) is False
        
        # Test that encryption/decryption works
        key = get_master_key_from_keyring(KEYRING_SERVICE_NAME)
        assert key is not None
        
        encrypted = encrypt_data("test message", key)
        decrypted = decrypt_data(encrypted, key)
        assert decrypted == "test message"
        
        # Cleanup
        delete_master_key_from_keyring(KEYRING_SERVICE_NAME)
    
    def test_key_backup_and_restore(self, tmp_path):
        """Test exporting and importing key backup."""
        password = "backup-password-456"
        backup_path = tmp_path / "key_backup.json"
        
        # Setup encryption
        setup_encryption(password, KEYRING_SERVICE_NAME)
        
        # Export backup
        result = export_key_backup(password, str(backup_path))
        assert result is True
        assert backup_path.exists()
        
        # Verify backup file contains expected structure
        import json
        backup_data = json.loads(backup_path.read_text())
        assert "version" in backup_data
        assert "encrypted_key" in backup_data
        
        # Delete key from keyring
        delete_master_key_from_keyring(KEYRING_SERVICE_NAME)
        
        # Verify key is gone
        key = get_master_key_from_keyring(KEYRING_SERVICE_NAME)
        assert key is None
        
        # Import backup
        result = import_key_backup(str(backup_path), password)
        assert result is True
        
        # Verify key is restored
        key = get_master_key_from_keyring(KEYRING_SERVICE_NAME)
        assert key is not None
        
        # Verify encryption/decryption works with restored key
        encrypted = encrypt_data("restored test", key)
        decrypted = decrypt_data(encrypted, key)
        assert decrypted == "restored test"
        
        # Cleanup
        delete_master_key_from_keyring(KEYRING_SERVICE_NAME)
