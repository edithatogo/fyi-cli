"""Secure credential storage for FYI API and other services.

This module provides secure storage and retrieval of credentials using OS keyring.
Supports multiple accounts and secure credential deletion.
"""
from __future__ import annotations
import json
from pathlib import Path
from typing import Optional, Dict, List
from dataclasses import dataclass, asdict
import keyring
from .encryption import (
    get_master_key_from_keyring,
    encrypt_data,
    decrypt_data,
    KEYRING_SERVICE_NAME,
)

# Keyring service names
CREDENTIAL_KEYRING_SERVICE = "fyi-request-system-credentials"
MASTER_CREDENTIAL_KEY = "master-credential-list"


@dataclass
class FYICredentials:
    """FYI.org.nz API credentials."""
    account_id: str
    email: str
    api_token: str  # Encrypted when stored
    base_url: str = "https://fyi.org.nz"
    notes: str = ""
    
    def to_dict(self) -> dict:
        """Convert to dictionary."""
        return asdict(self)
    
    @classmethod
    def from_dict(cls, data: dict) -> FYICredentials:
        """Create from dictionary."""
        return cls(**data)


class CredentialManager:
    """Manage FYI API credentials securely.
    
    Features:
    - Store credentials in OS keyring (encrypted)
    - Support multiple accounts
    - Secure credential deletion
    - Account switching
    """
    
    def __init__(self, app_name: str = CREDENTIAL_KEYRING_SERVICE):
        """Initialize credential manager.
        
        Args:
            app_name: Keyring service name for credentials
        """
        self.app_name = app_name
    
    def _get_account_list_key(self) -> str:
        """Get keyring key for account list."""
        return MASTER_CREDENTIAL_KEY
    
    def _get_credential_key(self, account_id: str) -> str:
        """Get keyring key for specific account.
        
        Args:
            account_id: Account identifier
        
        Returns:
            Keyring key for the account
        """
        return f"credential:{account_id}"
    
    def get_account_ids(self) -> List[str]:
        """Get list of all account IDs.
        
        Returns:
            List of account IDs
        """
        account_list_json = keyring.get_password(self.app_name, self._get_account_list_key())
        if not account_list_json:
            return []
        
        try:
            account_list = json.loads(account_list_json)
            return account_list if isinstance(account_list, list) else []
        except json.JSONDecodeError:
            return []
    
    def _save_account_ids(self, account_ids: List[str]) -> None:
        """Save list of account IDs.
        
        Args:
            account_ids: List of account IDs to save
        """
        keyring.set_password(self.app_name, self._get_account_list_key(), json.dumps(account_ids))
    
    def save_credentials(self, credentials: FYICredentials) -> bool:
        """Save credentials securely.
        
        Args:
            credentials: FYICredentials object to save
        
        Returns:
            True if save successful
        """
        try:
            # Get encryption key
            encryption_key = get_master_key_from_keyring(KEYRING_SERVICE_NAME)
            if not encryption_key:
                raise RuntimeError("Encryption key not found. Run setup_encryption() first.")
            
            # Encrypt sensitive fields
            encrypted_data = encrypt_data(
                json.dumps({
                    "email": credentials.email,
                    "api_token": credentials.api_token,
                    "notes": credentials.notes
                }),
                encryption_key
            )
            
            # Store encrypted credentials with metadata
            credential_data = {
                "account_id": credentials.account_id,
                "base_url": credentials.base_url,
                "encrypted_data": encrypted_data
            }
            
            # Save to keyring
            keyring.set_password(
                self.app_name,
                self._get_credential_key(credentials.account_id),
                json.dumps(credential_data)
            )
            
            # Update account list
            account_ids = self.get_account_ids()
            if credentials.account_id not in account_ids:
                account_ids.append(credentials.account_id)
                self._save_account_ids(account_ids)
            
            return True
        except Exception as e:
            print(f"Failed to save credentials: {e}")
            return False
    
    def get_credentials(self, account_id: str) -> Optional[FYICredentials]:
        """Retrieve credentials for an account.
        
        Args:
            account_id: Account identifier
        
        Returns:
            FYICredentials object or None if not found
        """
        try:
            # Get encryption key
            encryption_key = get_master_key_from_keyring(KEYRING_SERVICE_NAME)
            if not encryption_key:
                raise RuntimeError("Encryption key not found.")
            
            # Get encrypted credentials
            credential_json = keyring.get_password(
                self.app_name,
                self._get_credential_key(account_id)
            )
            if not credential_json:
                return None
            
            credential_data = json.loads(credential_json)
            
            # Decrypt sensitive fields
            decrypted_json = decrypt_data(credential_data["encrypted_data"], encryption_key)
            decrypted_data = json.loads(decrypted_json)
            
            # Reconstruct credentials
            return FYICredentials(
                account_id=account_id,
                email=decrypted_data["email"],
                api_token=decrypted_data["api_token"],
                base_url=credential_data.get("base_url", "https://fyi.org.nz"),
                notes=decrypted_data.get("notes", "")
            )
        except Exception as e:
            print(f"Failed to retrieve credentials: {e}")
            return None
    
    def delete_credentials(self, account_id: str) -> bool:
        """Delete credentials for an account.
        
        Args:
            account_id: Account identifier
        
        Returns:
            True if deletion successful
        """
        try:
            # Delete from keyring
            keyring.delete_password(
                self.app_name,
                self._get_credential_key(account_id)
            )
            
            # Update account list
            account_ids = self.get_account_ids()
            if account_id in account_ids:
                account_ids.remove(account_id)
                self._save_account_ids(account_ids)
            
            return True
        except keyring.errors.PasswordDeleteError:
            # Credential doesn't exist, which is fine
            return True
        except Exception as e:
            print(f"Failed to delete credentials: {e}")
            return False
    
    def delete_all_credentials(self) -> bool:
        """Delete all stored credentials.
        
        Returns:
            True if deletion successful
        """
        try:
            account_ids = self.get_account_ids()
            for account_id in account_ids:
                self.delete_credentials(account_id)
            
            # Clear account list
            keyring.delete_password(self.app_name, self._get_account_list_key())
            return True
        except Exception as e:
            print(f"Failed to delete all credentials: {e}")
            return False
    
    def list_accounts(self) -> List[Dict[str, str]]:
        """List all accounts (metadata only, not secrets).
        
        Returns:
            List of account metadata dictionaries
        """
        accounts = []
        for account_id in self.get_account_ids():
            try:
                credential_json = keyring.get_password(
                    self.app_name,
                    self._get_credential_key(account_id)
                )
                if credential_json:
                    credential_data = json.loads(credential_json)
                    accounts.append({
                        "account_id": account_id,
                        "base_url": credential_data.get("base_url", "https://fyi.org.nz"),
                        "email": "[ENCRYPTED]"  # Don't expose email in list
                    })
            except Exception:
                continue
        
        return accounts
    
    def has_credentials(self, account_id: str) -> bool:
        """Check if credentials exist for an account.
        
        Args:
            account_id: Account identifier
        
        Returns:
            True if credentials exist
        """
        return account_id in self.get_account_ids()
    
    def get_default_credentials(self) -> Optional[FYICredentials]:
        """Get default/first account credentials.
        
        Returns:
            FYICredentials or None if no accounts
        """
        account_ids = self.get_account_ids()
        if not account_ids:
            return None
        
        return self.get_credentials(account_ids[0])


# Convenience functions for simple usage

def save_fyi_credentials(
    account_id: str,
    email: str,
    api_token: str,
    base_url: str = "https://fyi.org.nz",
    notes: str = ""
) -> bool:
    """Save FYI API credentials.
    
    Args:
        account_id: Unique account identifier
        email: FYI account email
        api_token: FYI API token
        base_url: FYI base URL (default: https://fyi.org.nz)
        notes: Optional notes
    
    Returns:
        True if save successful
    """
    manager = CredentialManager()
    credentials = FYICredentials(
        account_id=account_id,
        email=email,
        api_token=api_token,
        base_url=base_url,
        notes=notes
    )
    return manager.save_credentials(credentials)


def get_fyi_credentials(account_id: str) -> Optional[FYICredentials]:
    """Get FYI API credentials.
    
    Args:
        account_id: Account identifier
    
    Returns:
        FYICredentials or None
    """
    manager = CredentialManager()
    return manager.get_credentials(account_id)


def delete_fyi_credentials(account_id: str) -> bool:
    """Delete FYI API credentials.
    
    Args:
        account_id: Account identifier
    
    Returns:
        True if deletion successful
    """
    manager = CredentialManager()
    return manager.delete_credentials(account_id)


def list_fyi_accounts() -> List[Dict[str, str]]:
    """List all FYI accounts.
    
    Returns:
        List of account metadata
    """
    manager = CredentialManager()
    return manager.list_accounts()
