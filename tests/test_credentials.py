"""Tests for secure credential storage."""
import pytest
from fyi_system.credentials import (
    FYICredentials,
    CredentialManager,
    save_fyi_credentials,
    get_fyi_credentials,
    delete_fyi_credentials,
    list_fyi_accounts,
    CREDENTIAL_KEYRING_SERVICE,
)
from fyi_system.encryption import (
    setup_encryption,
    delete_master_key_from_keyring,
    KEYRING_SERVICE_NAME,
)
import keyring


@pytest.fixture
def setup_encryption_fixture():
    """Set up encryption for credential tests."""
    password = "test-encryption-password"
    setup_encryption(password, KEYRING_SERVICE_NAME)
    yield
    # Cleanup
    delete_all_test_credentials()
    delete_master_key_from_keyring(KEYRING_SERVICE_NAME)


def delete_all_test_credentials():
    """Delete all test credentials from keyring."""
    manager = CredentialManager(CREDENTIAL_KEYRING_SERVICE)
    manager.delete_all_credentials()
    try:
        keyring.delete_password(CREDENTIAL_KEYRING_SERVICE, "master-credential-list")
    except keyring.errors.PasswordDeleteError:
        pass


class TestFYICredentials:
    """Test FYICredentials dataclass."""
    
    def test_create_credentials(self):
        """Test creating credentials object."""
        creds = FYICredentials(
            account_id="test-account",
            email="test@example.com",
            api_token="secret-token-123",
            base_url="https://fyi.org.nz",
            notes="Test account"
        )
        
        assert creds.account_id == "test-account"
        assert creds.email == "test@example.com"
        assert creds.api_token == "secret-token-123"
        assert creds.base_url == "https://fyi.org.nz"
        assert creds.notes == "Test account"
    
    def test_credentials_to_dict(self):
        """Test converting credentials to dictionary."""
        creds = FYICredentials(
            account_id="test",
            email="test@example.com",
            api_token="token",
            base_url="https://fyi.org.nz",
            notes="notes"
        )
        
        data = creds.to_dict()
        
        assert data["account_id"] == "test"
        assert data["email"] == "test@example.com"
        assert data["api_token"] == "token"
        assert data["base_url"] == "https://fyi.org.nz"
        assert data["notes"] == "notes"
    
    def test_credentials_from_dict(self):
        """Test creating credentials from dictionary."""
        data = {
            "account_id": "test",
            "email": "test@example.com",
            "api_token": "token",
            "base_url": "https://fyi.org.nz",
            "notes": "notes"
        }
        
        creds = FYICredentials.from_dict(data)
        
        assert creds.account_id == "test"
        assert creds.email == "test@example.com"
        assert creds.api_token == "token"


class TestCredentialManager:
    """Test CredentialManager class."""
    
    def test_save_and_retrieve_credentials(self, setup_encryption_fixture):
        """Test saving and retrieving credentials."""
        manager = CredentialManager(CREDENTIAL_KEYRING_SERVICE)
        
        creds = FYICredentials(
            account_id="test-account",
            email="test@example.com",
            api_token="secret-token",
            base_url="https://fyi.org.nz",
            notes="Test notes"
        )
        
        # Save
        result = manager.save_credentials(creds)
        assert result is True
        
        # Retrieve
        retrieved = manager.get_credentials("test-account")
        assert retrieved is not None
        assert retrieved.account_id == "test-account"
        assert retrieved.email == "test@example.com"
        assert retrieved.api_token == "secret-token"
        assert retrieved.base_url == "https://fyi.org.nz"
        assert retrieved.notes == "Test notes"
    
    def test_save_multiple_accounts(self, setup_encryption_fixture):
        """Test saving multiple accounts."""
        manager = CredentialManager(CREDENTIAL_KEYRING_SERVICE)
        
        # Save first account
        creds1 = FYICredentials(
            account_id="account-1",
            email="user1@example.com",
            api_token="token-1"
        )
        manager.save_credentials(creds1)
        
        # Save second account
        creds2 = FYICredentials(
            account_id="account-2",
            email="user2@example.com",
            api_token="token-2"
        )
        manager.save_credentials(creds2)
        
        # Verify both exist
        retrieved1 = manager.get_credentials("account-1")
        retrieved2 = manager.get_credentials("account-2")
        
        assert retrieved1 is not None
        assert retrieved1.email == "user1@example.com"
        assert retrieved2 is not None
        assert retrieved2.email == "user2@example.com"
    
    def test_list_accounts(self, setup_encryption_fixture):
        """Test listing accounts."""
        manager = CredentialManager(CREDENTIAL_KEYRING_SERVICE)
        
        # Save two accounts
        manager.save_credentials(FYICredentials("acc-1", "user1@example.com", "token-1"))
        manager.save_credentials(FYICredentials("acc-2", "user2@example.com", "token-2"))
        
        # List accounts
        accounts = manager.list_accounts()
        
        assert len(accounts) == 2
        account_ids = [acc["account_id"] for acc in accounts]
        assert "acc-1" in account_ids
        assert "acc-2" in account_ids
        
        # Verify emails are not exposed in list
        for acc in accounts:
            assert acc["email"] == "[ENCRYPTED]"
    
    def test_delete_credentials(self, setup_encryption_fixture):
        """Test deleting credentials."""
        manager = CredentialManager(CREDENTIAL_KEYRING_SERVICE)
        
        # Save credentials
        creds = FYICredentials("test-delete", "test@example.com", "token")
        manager.save_credentials(creds)
        
        # Verify saved
        assert manager.has_credentials("test-delete") is True
        
        # Delete
        result = manager.delete_credentials("test-delete")
        assert result is True
        
        # Verify deleted
        assert manager.has_credentials("test-delete") is False
        assert manager.get_credentials("test-delete") is None
    
    def test_delete_all_credentials(self, setup_encryption_fixture):
        """Test deleting all credentials."""
        manager = CredentialManager(CREDENTIAL_KEYRING_SERVICE)
        
        # Save multiple accounts
        manager.save_credentials(FYICredentials("acc-1", "user1@example.com", "token-1"))
        manager.save_credentials(FYICredentials("acc-2", "user2@example.com", "token-2"))
        manager.save_credentials(FYICredentials("acc-3", "user3@example.com", "token-3"))
        
        # Verify all saved
        assert len(manager.get_account_ids()) == 3
        
        # Delete all
        result = manager.delete_all_credentials()
        assert result is True
        
        # Verify all deleted
        assert len(manager.get_account_ids()) == 0
    
    def test_get_default_credentials(self, setup_encryption_fixture):
        """Test getting default (first) credentials."""
        manager = CredentialManager(CREDENTIAL_KEYRING_SERVICE)
        
        # Save accounts
        manager.save_credentials(FYICredentials("first", "first@example.com", "token-1"))
        manager.save_credentials(FYICredentials("second", "second@example.com", "token-2"))
        
        # Get default
        default = manager.get_default_credentials()
        
        assert default is not None
        # Should return first account
        assert default.account_id == "first"
    
    def test_get_nonexistent_credentials(self, setup_encryption_fixture):
        """Test getting credentials that don't exist."""
        manager = CredentialManager(CREDENTIAL_KEYRING_SERVICE)
        
        result = manager.get_credentials("nonexistent")
        assert result is None
    
    def test_has_credentials(self, setup_encryption_fixture):
        """Test checking if credentials exist."""
        manager = CredentialManager(CREDENTIAL_KEYRING_SERVICE)
        
        # Check nonexistent
        assert manager.has_credentials("nonexistent") is False
        
        # Save and check
        manager.save_credentials(FYICredentials("exists", "test@example.com", "token"))
        assert manager.has_credentials("exists") is True
    
    def test_update_credentials(self, setup_encryption_fixture):
        """Test updating existing credentials."""
        manager = CredentialManager(CREDENTIAL_KEYRING_SERVICE)
        
        # Save initial
        creds1 = FYICredentials("update-test", "old@example.com", "old-token")
        manager.save_credentials(creds1)
        
        # Update
        creds2 = FYICredentials("update-test", "new@example.com", "new-token", notes="Updated")
        manager.save_credentials(creds2)
        
        # Retrieve and verify updated
        retrieved = manager.get_credentials("update-test")
        assert retrieved is not None
        assert retrieved.email == "new@example.com"
        assert retrieved.api_token == "new-token"
        assert retrieved.notes == "Updated"


class TestConvenienceFunctions:
    """Test module-level convenience functions."""
    
    def test_save_and_get_fyi_credentials(self, setup_encryption_fixture):
        """Test save_fyi_credentials and get_fyi_credentials."""
        # Save
        result = save_fyi_credentials(
            account_id="conv-test",
            email="conv@example.com",
            api_token="conv-token",
            base_url="https://fyi.org.nz",
            notes="Convenience test"
        )
        assert result is True
        
        # Get
        creds = get_fyi_credentials("conv-test")
        assert creds is not None
        assert creds.email == "conv@example.com"
        assert creds.api_token == "conv-token"
    
    def test_delete_fyi_credentials(self, setup_encryption_fixture):
        """Test delete_fyi_credentials."""
        # Save
        save_fyi_credentials("delete-conv", "test@example.com", "token")
        
        # Delete
        result = delete_fyi_credentials("delete-conv")
        assert result is True
        
        # Verify deleted
        creds = get_fyi_credentials("delete-conv")
        assert creds is None
    
    def test_list_fyi_accounts(self, setup_encryption_fixture):
        """Test list_fyi_accounts."""
        # Save accounts
        save_fyi_credentials("list-1", "user1@example.com", "token-1")
        save_fyi_credentials("list-2", "user2@example.com", "token-2")
        
        # List
        accounts = list_fyi_accounts()
        
        assert len(accounts) >= 2
        account_ids = [acc["account_id"] for acc in accounts]
        assert "list-1" in account_ids
        assert "list-2" in account_ids
