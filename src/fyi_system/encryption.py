"""Encryption utilities for security hardening.

This module provides encryption/decryption functions for protecting sensitive data at rest.
Uses AES-256-GCM for encryption with PBKDF2 for key derivation (cryptography library standard).
"""
from __future__ import annotations
import os
import base64
import json
import hashlib
from pathlib import Path
from typing import Optional, Tuple
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.backends import default_backend
import keyring

# Constants
KEYRING_SERVICE_NAME = "fyi-cli"
KEYRING_USERNAME = "encryption-key"
KEYRING_PASSWORD_VERIFIER = "password-verifier"
SALT_LENGTH = 16  # 128 bits
NONCE_LENGTH = 12  # 96 bits for GCM
PBKDF2_ITERATIONS = 100000
KEY_LENGTH = 32  # 256 bits for AES-256
PASSWORD_VERIFIER_SALT = b"fyi-password-verifier-salt"  # Fixed salt for password verification only


def generate_salt() -> bytes:
    """Generate a cryptographically secure random salt."""
    return os.urandom(SALT_LENGTH)


def derive_key(password: str, salt: bytes) -> bytes:
    """Derive an encryption key from a password using PBKDF2-HMAC-SHA256.
    
    Args:
        password: User password
        salt: Random salt (should be stored with encrypted data)
    
    Returns:
        32-byte key suitable for AES-256
    """
    kdf = PBKDF2HMAC(
        algorithm=hashes.SHA256(),
        length=KEY_LENGTH,
        salt=salt,
        iterations=PBKDF2_ITERATIONS,
        backend=default_backend()
    )
    return kdf.derive(password.encode('utf-8'))


def encrypt_data(plaintext: str, key: bytes, salt: Optional[bytes] = None) -> str:
    """Encrypt plaintext using AES-256-GCM.

    Args:
        plaintext: Data to encrypt
        key: 32-byte encryption key (use directly, no derivation)
        salt: Optional salt for key derivation (generated if not provided)

    Returns:
        Base64-encoded ciphertext (salt + nonce + ciphertext + tag)
    """
    if salt is None:
        salt = generate_salt()
    nonce = os.urandom(NONCE_LENGTH)

    # Use key directly (already 32 bytes from master key generation)
    aesgcm = AESGCM(key)
    ciphertext_with_tag = aesgcm.encrypt(nonce, plaintext.encode('utf-8'), None)

    # Combine salt + nonce + ciphertext
    encrypted_data = salt + nonce + ciphertext_with_tag

    return base64.b64encode(encrypted_data).decode('utf-8')


def decrypt_data(encrypted_data: str, key: bytes) -> str:
    """Decrypt ciphertext using AES-256-GCM.

    Args:
        encrypted_data: Base64-encoded ciphertext (salt + nonce + ciphertext + tag)
        key: 32-byte encryption key (use directly)

    Returns:
        Decrypted plaintext

    Raises:
        ValueError: If decryption fails (wrong key or corrupted data)
    """
    try:
        encrypted_bytes = base64.b64decode(encrypted_data)

        # Extract components (salt is included but not used for key derivation)
        salt = encrypted_bytes[:SALT_LENGTH]
        nonce = encrypted_bytes[SALT_LENGTH:SALT_LENGTH + NONCE_LENGTH]
        ciphertext_with_tag = encrypted_bytes[SALT_LENGTH + NONCE_LENGTH:]

        # Use key directly
        aesgcm = AESGCM(key)
        plaintext = aesgcm.decrypt(nonce, ciphertext_with_tag, None)

        return plaintext.decode('utf-8')
    except Exception as e:
        raise ValueError(f"Decryption failed: {str(e)}")


def get_master_key_from_keyring(app_name: str = KEYRING_SERVICE_NAME) -> Optional[bytes]:
    """Retrieve master encryption key from OS keyring.
    
    Args:
        app_name: Keyring service name
    
    Returns:
        Master key bytes or None if not found
    """
    key_b64 = keyring.get_password(app_name, KEYRING_USERNAME)
    if key_b64:
        return base64.b64decode(key_b64)
    return None


def set_master_key_in_keyring(key: bytes, app_name: str = KEYRING_SERVICE_NAME) -> None:
    """Store master encryption key in OS keyring.
    
    Args:
        key: Master key bytes to store
        app_name: Keyring service name
    """
    key_b64 = base64.b64encode(key).decode('utf-8')
    keyring.set_password(app_name, KEYRING_USERNAME, key_b64)


def delete_master_key_from_keyring(app_name: str = KEYRING_SERVICE_NAME) -> None:
    """Delete master encryption key from OS keyring.
    
    Args:
        app_name: Keyring service name
    """
    try:
        keyring.delete_password(app_name, KEYRING_USERNAME)
    except keyring.errors.PasswordDeleteError:
        pass  # Key doesn't exist, which is fine


def generate_master_key() -> bytes:
    """Generate a new master encryption key.
    
    Returns:
        32-byte cryptographically secure random key
    """
    return os.urandom(32)


def setup_encryption(password: str, app_name: str = KEYRING_SERVICE_NAME) -> bool:
    """Set up encryption with a user password.

    This generates a master key, stores it in the OS keyring,
    and creates a password verifier for later validation.

    Args:
        password: User password for key derivation
        app_name: Keyring service name

    Returns:
        True if setup successful
    """
    try:
        # Generate master key
        master_key = generate_master_key()

        # Store in keyring
        set_master_key_in_keyring(master_key, app_name)
        
        # Create and store password verifier
        password_hash = hashlib.pbkdf2_hmac(
            'sha256',
            password.encode('utf-8'),
            PASSWORD_VERIFIER_SALT,
            PBKDF2_ITERATIONS
        )
        keyring.set_password(app_name, KEYRING_PASSWORD_VERIFIER, base64.b64encode(password_hash).decode('utf-8'))

        return True
    except Exception as e:
        print(f"Encryption setup failed: {e}")
        return False


def verify_password(password: str, app_name: str = KEYRING_SERVICE_NAME) -> bool:
    """Verify that a password matches the stored password verifier.

    Args:
        password: User password to verify
        app_name: Keyring service name

    Returns:
        True if password is correct
    """
    try:
        stored_verifier = keyring.get_password(app_name, KEYRING_PASSWORD_VERIFIER)
        if not stored_verifier:
            return False
        
        # Compute hash of provided password
        password_hash = hashlib.pbkdf2_hmac(
            'sha256',
            password.encode('utf-8'),
            PASSWORD_VERIFIER_SALT,
            PBKDF2_ITERATIONS
        )
        
        return base64.b64encode(password_hash).decode('utf-8') == stored_verifier
    except Exception:
        return False


def export_key_backup(password: str, output_path: str | Path) -> bool:
    """Export encrypted key backup to file.
    
    Args:
        password: User password for encrypting backup
        output_path: Path to save backup file
    
    Returns:
        True if backup successful
    """
    try:
        key = get_master_key_from_keyring()
        if not key:
            return False
        
        # Derive backup encryption key from password
        backup_salt = generate_salt()
        backup_key = derive_key(password, backup_salt)
        
        # Encrypt master key
        encrypted_key = encrypt_data(base64.b64encode(key).decode('utf-8'), backup_key, backup_salt)
        
        # Write to file with metadata
        backup_data = {
            "version": 1,
            "encrypted_key": encrypted_key
        }
        Path(output_path).write_text(json.dumps(backup_data))
        return True
    except Exception as e:
        print(f"Key backup failed: {e}")
        return False


def import_key_backup(encrypted_key_path: str | Path, password: str) -> bool:
    """Import master key from encrypted backup file.
    
    Args:
        encrypted_key_path: Path to backup file
        password: User password for decrypting backup
    
    Returns:
        True if import successful
    """
    try:
        backup_data = json.loads(Path(encrypted_key_path).read_text())
        encrypted_key = backup_data["encrypted_key"]
        
        # Extract salt from encrypted data (first 16 bytes after base64 decode)
        # Note: encrypt_data includes salt in the output
        encrypted_bytes = base64.b64decode(encrypted_key)
        backup_salt = encrypted_bytes[:SALT_LENGTH]
        
        # Derive backup decryption key from password
        backup_key = derive_key(password, backup_salt)
        
        # Decrypt master key
        decrypted_key_b64 = decrypt_data(encrypted_key, backup_key)
        master_key = base64.b64decode(decrypted_key_b64)
        
        # Store in keyring
        set_master_key_in_keyring(master_key)
        return True
    except Exception as e:
        print(f"Key import failed: {e}")
        return False


class EncryptedField:
    """Descriptor for encrypting/decrypting model fields.
    
    Usage:
        class TrackedRequest:
            title = EncryptedField('title')
            body = EncryptedField('body')
    """
    
    def __init__(self, field_name: str):
        self.field_name = field_name
        self.internal_name = f"_encrypted_{field_name}"
    
    def __get__(self, obj, objtype=None):
        if obj is None:
            return self
        
        # Return encrypted value if it exists
        value = getattr(obj, self.internal_name, None)
        if value and value.startswith('enc:'):
            # Decrypt on access
            key = get_master_key_from_keyring()
            if key:
                try:
                    return decrypt_data(value[4:], key)
                except ValueError:
                    pass  # Return encrypted value if decryption fails
        return value
    
    def __set__(self, obj, value):
        if value is None:
            setattr(obj, self.internal_name, None)
            return

        # Encrypt on set
        key = get_master_key_from_keyring()
        if key:
            encrypted = encrypt_data(value, key)
            setattr(obj, self.internal_name, f"enc:{encrypted}")
        else:
            # Raise exception if no key available - prevents silent plaintext storage
            raise RuntimeError(
                f"Cannot encrypt field '{self.field_name}': encryption key not found in keyring. "
                "Run setup_encryption() first."
            )
