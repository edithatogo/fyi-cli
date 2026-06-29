use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, Mac};
use keyring::{Entry, Error as KeyringBackendError};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use qrcode::QrCode;
use sha1::Sha1;
use std::collections::BTreeSet;
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

    #[error("Invalid TOTP secret: {0}")]
    InvalidTotpSecret(String),
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

const TOTP_SECRET_BYTES: usize = 20;
const TOTP_TIME_STEP_SECONDS: u64 = 30;
const TOTP_DIGITS: u32 = 6;
const TOTP_SECRET_PREFIX: &str = "totp-secret:";
const TOTP_SECRET_INDEX: &str = "totp-secret-index";
const OTPAUTH_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'&')
    .add(b'+')
    .add(b'/')
    .add(b':')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

type HmacSha1 = Hmac<Sha1>;

/// Generates a cryptographically secure base32 TOTP secret.
pub fn generate_totp_secret() -> Result<ZeroizedString, SecurityError> {
    use aes_gcm::aead::rand_core::RngCore;

    let mut secret = [0u8; TOTP_SECRET_BYTES];
    OsRng.fill_bytes(&mut secret);
    let encoded = BASE32_NOPAD.encode(&secret);
    secret.zeroize();
    Ok(ZeroizedString::new(encoded))
}

/// Generates an RFC 6238 TOTP code for the provided Unix timestamp.
pub fn generate_totp_code(
    secret: &ZeroizedString,
    unix_timestamp: u64,
) -> Result<String, SecurityError> {
    let mut secret_bytes = decode_totp_secret(secret)?;
    let counter = unix_timestamp / TOTP_TIME_STEP_SECONDS;
    let code = hotp(&secret_bytes, counter, TOTP_DIGITS)?;
    secret_bytes.zeroize();
    Ok(format!("{code:0width$}", width = TOTP_DIGITS as usize))
}

/// Verifies a TOTP code using the provided drift tolerance in 30-second windows.
pub fn verify_totp_code(
    secret: &ZeroizedString,
    code: &str,
    unix_timestamp: u64,
    drift_windows: u8,
) -> Result<bool, SecurityError> {
    if !is_valid_totp_code(code) {
        return Ok(false);
    }

    let current_step = unix_timestamp / TOTP_TIME_STEP_SECONDS;
    for offset in -(drift_windows as i64)..=(drift_windows as i64) {
        let Some(step) = current_step.checked_add_signed(offset) else {
            continue;
        };
        let candidate = generate_totp_code(secret, step * TOTP_TIME_STEP_SECONDS)?;
        if constant_time_eq(candidate.as_bytes(), code.as_bytes()) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Builds an otpauth provisioning URI for authenticator applications.
pub fn build_provisioning_uri(
    issuer: &str,
    account: &str,
    secret: &ZeroizedString,
) -> Result<String, SecurityError> {
    decode_totp_secret(secret)?;

    let encoded_issuer = encode_otpauth_component(issuer);
    let encoded_account = encode_otpauth_component(account);

    Ok(format!(
        "otpauth://totp/{encoded_issuer}:{encoded_account}?secret={secret}&issuer={encoded_issuer}&algorithm=SHA1&digits={digits}&period={period}",
        secret = secret.as_str(),
        digits = TOTP_DIGITS,
        period = TOTP_TIME_STEP_SECONDS
    ))
}

/// Renders a provisioning URI as terminal-friendly QR code blocks.
pub fn render_provisioning_qr_ascii(uri: &str) -> Result<String, SecurityError> {
    let code =
        QrCode::new(uri.as_bytes()).map_err(|e| SecurityError::InvalidTotpSecret(e.to_string()))?;

    Ok(code
        .render::<char>()
        .quiet_zone(true)
        .module_dimensions(2, 1)
        .dark_color('█')
        .light_color(' ')
        .build())
}

fn encode_otpauth_component(component: &str) -> String {
    utf8_percent_encode(component, OTPAUTH_ENCODE_SET).to_string()
}

fn decode_totp_secret(secret: &ZeroizedString) -> Result<Vec<u8>, SecurityError> {
    let normalized = secret
        .as_str()
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();

    BASE32_NOPAD
        .decode(normalized.as_bytes())
        .map_err(|e| SecurityError::InvalidTotpSecret(e.to_string()))
}

fn hotp(secret: &[u8], counter: u64, digits: u32) -> Result<u32, SecurityError> {
    let mut mac = <HmacSha1 as Mac>::new_from_slice(secret)
        .map_err(|e| SecurityError::InvalidTotpSecret(e.to_string()))?;
    mac.update(&counter.to_be_bytes());
    let result = mac.finalize().into_bytes();
    let offset = (result[19] & 0x0f) as usize;
    let binary = ((u32::from(result[offset]) & 0x7f) << 24)
        | (u32::from(result[offset + 1]) << 16)
        | (u32::from(result[offset + 2]) << 8)
        | u32::from(result[offset + 3]);
    Ok(binary % 10_u32.pow(digits))
}

fn is_valid_totp_code(code: &str) -> bool {
    matches!(code.len(), 6 | 8) && code.chars().all(|ch| ch.is_ascii_digit())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter()
        .zip(right.iter())
        .fold(0u8, |acc, (left, right)| acc | (left ^ right))
        == 0
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

    pub fn store_totp_secret(
        &self,
        username: &str,
        secret: &ZeroizedString,
    ) -> Result<(), SecurityError> {
        decode_totp_secret(secret)?;
        let entry = Entry::new(&self.service, &totp_secret_key(username))
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        entry
            .set_password(secret.as_str())
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;

        let mut usernames = self.read_totp_secret_index()?;
        usernames.insert(username.to_string());
        self.write_totp_secret_index(&usernames)
    }

    pub fn get_totp_secret(&self, username: &str) -> Result<ZeroizedString, SecurityError> {
        let entry = Entry::new(&self.service, &totp_secret_key(username))
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        let secret = entry
            .get_password()
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        Ok(ZeroizedString::new(secret))
    }

    pub fn delete_totp_secret(&self, username: &str) -> Result<(), SecurityError> {
        let entry = Entry::new(&self.service, &totp_secret_key(username))
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        entry
            .delete_password()
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;

        let mut usernames = self.read_totp_secret_index()?;
        usernames.remove(username);
        self.write_totp_secret_index(&usernames)
    }

    pub fn list_totp_secrets(&self) -> Result<Vec<String>, SecurityError> {
        Ok(self.read_totp_secret_index()?.into_iter().collect())
    }

    fn read_totp_secret_index(&self) -> Result<BTreeSet<String>, SecurityError> {
        let entry = Entry::new(&self.service, TOTP_SECRET_INDEX)
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        let payload = match entry.get_password() {
            Ok(payload) => payload,
            Err(KeyringBackendError::NoEntry) => return Ok(BTreeSet::new()),
            Err(error) => return Err(SecurityError::KeyringError(error.to_string())),
        };

        serde_json::from_str::<Vec<String>>(&payload)
            .map(|usernames| usernames.into_iter().collect())
            .map_err(|e| SecurityError::KeyringError(e.to_string()))
    }

    fn write_totp_secret_index(&self, usernames: &BTreeSet<String>) -> Result<(), SecurityError> {
        let entry = Entry::new(&self.service, TOTP_SECRET_INDEX)
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        let payload = serde_json::to_string(&usernames.iter().collect::<Vec<_>>())
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        entry
            .set_password(&payload)
            .map_err(|e| SecurityError::KeyringError(e.to_string()))
    }
}

fn totp_secret_key(username: &str) -> String {
    format!("{TOTP_SECRET_PREFIX}{username}")
}
