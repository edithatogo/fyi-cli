use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};
use keyring::Entry;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(thiserror::Error, Debug)]
pub enum SecurityError {
    #[error("Encryption failure: {0}")]
    EncryptionError(String),

    #[error("Decryption failure: {0}")]
    DecryptionError(String),

    #[error("Keyring error: {0}")]
    KeyringError(String),

    #[error("Ciphertext too short: must be at least {0} bytes")]
    CiphertextTooShort(usize),
}

/// A wrapper around a String that ensures its contents are zeroed out when dropped.
#[derive(Clone, Default, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct ZeroizedString(String);

impl ZeroizedString {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ZeroizedString {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Debug for ZeroizedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED SECRET STRING]")
    }
}

impl serde::Serialize for ZeroizedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for ZeroizedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self)
    }
}

/// A wrapper around a byte vector that ensures its contents are zeroed out when dropped.
#[derive(Clone, Default, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct ZeroizedBytes(Vec<u8>);

impl ZeroizedBytes {
    pub fn new(b: Vec<u8>) -> Self {
        Self(b)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for ZeroizedBytes {
    fn from(b: Vec<u8>) -> Self {
        Self(b)
    }
}

impl std::fmt::Debug for ZeroizedBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED SECRET BYTES]")
    }
}

impl serde::Serialize for ZeroizedBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for ZeroizedBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Vec::<u8>::deserialize(deserializer).map(Self)
    }
}

/// A wrapper around a 32-byte key that ensures it is zeroed out when dropped.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct EncryptionKey(pub [u8; 32]);

impl EncryptionKey {
    pub fn generate() -> Self {
        use aes_gcm::aead::rand_core::RngCore;
        let mut key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut key_bytes);
        Self(key_bytes)
    }
}

/// Encrypts plaintext using AES-256-GCM and the provided 32-byte key.
/// Returns a `ZeroizedBytes` containing the 12-byte nonce followed by the ciphertext.
pub fn encrypt(
    plaintext: &ZeroizedBytes,
    key: &EncryptionKey,
) -> Result<ZeroizedBytes, SecurityError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key.0));
    
    // Generate a random 96-bit nonce
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_slice())
        .map_err(|e| SecurityError::EncryptionError(e.to_string()))?;
        
    let mut result = Vec::with_capacity(nonce.len() + ciphertext.len());
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);
    
    Ok(ZeroizedBytes::new(result))
}

/// Decrypts ciphertext (which starts with a 12-byte nonce) using AES-256-GCM and the provided 32-byte key.
pub fn decrypt(
    nonce_and_ciphertext: &ZeroizedBytes,
    key: &EncryptionKey,
) -> Result<ZeroizedBytes, SecurityError> {
    let data = nonce_and_ciphertext.as_slice();
    if data.len() < 12 {
        return Err(SecurityError::CiphertextTooShort(12));
    }
    
    let (nonce_bytes, ciphertext_bytes) = data.split_at(12);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key.0));
    let nonce = Nonce::from_slice(nonce_bytes);
    
    let decrypted = cipher
        .decrypt(nonce, ciphertext_bytes)
        .map_err(|e| SecurityError::DecryptionError(e.to_string()))?;
        
    Ok(ZeroizedBytes::new(decrypted))
}

/// Wrapper around the system keyring.
pub struct KeyringStore {
    service: String,
}

impl KeyringStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    pub fn get_credential(&self, username: &str) -> Result<ZeroizedString, SecurityError> {
        let entry = Entry::new(&self.service, username)
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        let password = entry
            .get_password()
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        Ok(ZeroizedString::new(password))
    }

    pub fn set_credential(
        &self,
        username: &str,
        password: &ZeroizedString,
    ) -> Result<(), SecurityError> {
        let entry = Entry::new(&self.service, username)
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        entry
            .set_password(password.as_str())
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        Ok(())
    }

    pub fn delete_credential(&self, username: &str) -> Result<(), SecurityError> {
        let entry = Entry::new(&self.service, username)
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        entry
            .delete_password()
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        Ok(())
    }
}
