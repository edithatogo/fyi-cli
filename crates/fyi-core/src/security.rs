use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, Mac};
use keyring::{Entry, Error as KeyringBackendError};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use qrcode::QrCode;
use sha1::Sha1;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
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

    #[error("MFA verification required for credential access: {0}")]
    MfaRequired(String),

    #[error("MFA verification is rate limited for: {0}")]
    MfaRateLimited(String),
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
const TOTP_SECRET_ACTIVE_PREFIX: &str = "totp-secret-active:";
const TOTP_SECRET_VERSION_PREFIX: &str = "totp-secret-version:";
const TOTP_SECRET_VERSIONS_PREFIX: &str = "totp-secret-versions:";
const TOTP_SECRET_INDEX: &str = "totp-secret-index";
const CREDENTIAL_INDEX: &str = "credential-index";
const MFA_SESSION_TTL_SECONDS: u64 = 300;
const MFA_RATE_LIMIT_MAX_ATTEMPTS: usize = 5;
const MFA_RATE_LIMIT_WINDOW_SECONDS: u64 = 30;
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
    let mut secret = [0u8; TOTP_SECRET_BYTES];
    getrandom::fill(&mut secret).map_err(|e| SecurityError::EncryptionError(e.to_string()))?;
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
        let mut key_bytes = [0u8; 32];
        getrandom::fill(&mut key_bytes).expect("operating-system random source is unavailable");
        Self(key_bytes)
    }
}

/// Encrypts plaintext using AES-256-GCM and the provided 32-byte key.
/// Returns a `ZeroizedBytes` containing the 12-byte nonce followed by the ciphertext.
pub fn encrypt(
    plaintext: &ZeroizedBytes,
    key: &EncryptionKey,
) -> Result<ZeroizedBytes, SecurityError> {
    let cipher = Aes256Gcm::new_from_slice(&key.0)
        .map_err(|e| SecurityError::EncryptionError(e.to_string()))?;

    // Generate an independent random 96-bit nonce for this encryption operation.
    let mut nonce_bytes = [0u8; 12];
    getrandom::fill(&mut nonce_bytes).map_err(|e| SecurityError::EncryptionError(e.to_string()))?;
    let nonce = Nonce::try_from(nonce_bytes.as_slice())
        .map_err(|e| SecurityError::EncryptionError(e.to_string()))?;

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
    let cipher = Aes256Gcm::new_from_slice(&key.0)
        .map_err(|e| SecurityError::DecryptionError(e.to_string()))?;
    let nonce =
        Nonce::try_from(nonce_bytes).map_err(|e| SecurityError::DecryptionError(e.to_string()))?;

    let decrypted = cipher
        .decrypt(&nonce, ciphertext_bytes)
        .map_err(|e| SecurityError::DecryptionError(e.to_string()))?;

    Ok(ZeroizedBytes::new(decrypted))
}

/// Wrapper around the system keyring.
pub struct KeyringStore {
    service: String,
    backend: CredentialBackend,
    mfa_guard: MfaGuard,
}

impl KeyringStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            backend: CredentialBackend::System,
            mfa_guard: MfaGuard::default(),
        }
    }

    pub fn new_in_memory(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            backend: CredentialBackend::InMemory(Arc::new(Mutex::new(BTreeMap::new()))),
            mfa_guard: MfaGuard::default(),
        }
    }

    pub fn get_credential(&self, username: &str) -> Result<ZeroizedString, SecurityError> {
        if self.is_mfa_enabled(username)?
            && !self
                .mfa_guard
                .is_verified(username, current_unix_timestamp()?)
        {
            return Err(SecurityError::MfaRequired(username.to_string()));
        }

        let password = self
            .get_keyring_password(username)?
            .ok_or_else(|| no_keyring_entry_error(username))?;
        Ok(ZeroizedString::new(password))
    }

    pub fn set_credential(
        &self,
        username: &str,
        password: &ZeroizedString,
    ) -> Result<(), SecurityError> {
        self.set_keyring_password(username, password.as_str())?;
        let mut usernames = self.read_credential_index()?;
        usernames.insert(username.to_string());
        self.write_credential_index(&usernames)
    }

    pub fn delete_credential(&self, username: &str) -> Result<(), SecurityError> {
        self.delete_keyring_password(username)?;
        let mut usernames = self.read_credential_index()?;
        usernames.remove(username);
        self.write_credential_index(&usernames)
    }

    pub fn list_credentials(&self) -> Result<Vec<String>, SecurityError> {
        Ok(self.read_credential_index()?.into_iter().collect())
    }

    pub fn store_totp_secret(
        &self,
        username: &str,
        secret: &ZeroizedString,
    ) -> Result<(), SecurityError> {
        self.store_totp_secret_version(username, 1, secret)
    }

    pub fn get_totp_secret(&self, username: &str) -> Result<ZeroizedString, SecurityError> {
        if let Some(version) = self.read_active_totp_secret_version(username)? {
            return self.get_totp_secret_version(username, version);
        }

        let key = totp_secret_key(username);
        let secret = self
            .get_keyring_password(&key)?
            .ok_or_else(|| no_keyring_entry_error(&key))?;
        Ok(ZeroizedString::new(secret))
    }

    pub fn delete_totp_secret(&self, username: &str) -> Result<(), SecurityError> {
        let versions = self.read_totp_secret_versions(username)?;
        for version in versions {
            self.delete_keyring_password(&totp_secret_version_key(username, version))?;
        }
        self.delete_keyring_password(&totp_secret_versions_key(username))?;
        self.delete_keyring_password(&totp_secret_active_key(username))?;
        self.delete_keyring_password(&totp_secret_key(username))?;

        let mut usernames = self.read_totp_secret_index()?;
        usernames.remove(username);
        self.write_totp_secret_index(&usernames)
    }

    pub fn list_totp_secrets(&self) -> Result<Vec<String>, SecurityError> {
        Ok(self.read_totp_secret_index()?.into_iter().collect())
    }

    pub fn verify_mfa_code(
        &self,
        username: &str,
        code: &str,
        unix_timestamp: u64,
        drift_windows: u8,
    ) -> Result<bool, SecurityError> {
        if self.mfa_guard.is_rate_limited(username, unix_timestamp) {
            self.mfa_guard.record_audit_event(
                username,
                MfaAuditEventKind::RateLimited,
                unix_timestamp,
            );
            return Err(SecurityError::MfaRateLimited(username.to_string()));
        }

        let secret = self.get_totp_secret(username)?;
        let verified = verify_totp_code(&secret, code, unix_timestamp, drift_windows)?;
        if verified {
            self.mfa_guard.clear_failed_attempts(username);
            self.mfa_guard
                .mark_verified(username, current_unix_timestamp()?);
            self.mfa_guard.record_audit_event(
                username,
                MfaAuditEventKind::VerificationSucceeded,
                unix_timestamp,
            );
        } else {
            self.mfa_guard
                .record_failed_attempt(username, unix_timestamp);
            self.mfa_guard.record_audit_event(
                username,
                MfaAuditEventKind::VerificationFailed,
                unix_timestamp,
            );
        }
        Ok(verified)
    }

    pub fn mfa_audit_events(&self) -> Vec<MfaAuditEvent> {
        self.mfa_guard.audit_events()
    }

    pub fn store_totp_secret_version(
        &self,
        username: &str,
        version: u32,
        secret: &ZeroizedString,
    ) -> Result<(), SecurityError> {
        if version == 0 {
            return Err(SecurityError::InvalidTotpSecret(
                "TOTP secret version must be greater than zero".to_string(),
            ));
        }

        decode_totp_secret(secret)?;
        let version_key = totp_secret_version_key(username, version);
        self.set_keyring_password(&version_key, secret.as_str())?;

        let mut versions = self.read_totp_secret_versions(username)?;
        versions.insert(version);
        self.write_totp_secret_versions(username, &versions)?;
        self.write_active_totp_secret_version(username, version)?;
        self.write_legacy_totp_secret(username, secret)?;

        let mut usernames = self.read_totp_secret_index()?;
        usernames.insert(username.to_string());
        self.write_totp_secret_index(&usernames)
    }

    pub fn get_totp_secret_version(
        &self,
        username: &str,
        version: u32,
    ) -> Result<ZeroizedString, SecurityError> {
        let version_key = totp_secret_version_key(username, version);
        let secret = self
            .get_keyring_password(&version_key)?
            .ok_or_else(|| no_keyring_entry_error(&version_key))?;
        Ok(ZeroizedString::new(secret))
    }

    pub fn list_totp_secret_versions(&self, username: &str) -> Result<Vec<u32>, SecurityError> {
        Ok(self
            .read_totp_secret_versions(username)?
            .into_iter()
            .collect())
    }

    pub fn rotate_totp_secret(
        &self,
        username: &str,
        new_secret: &ZeroizedString,
    ) -> Result<u32, SecurityError> {
        let next_version = self
            .read_totp_secret_versions(username)?
            .last()
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| {
                SecurityError::InvalidTotpSecret("TOTP secret version overflow".to_string())
            })?;
        self.store_totp_secret_version(username, next_version, new_secret)?;
        Ok(next_version)
    }

    fn write_legacy_totp_secret(
        &self,
        username: &str,
        secret: &ZeroizedString,
    ) -> Result<(), SecurityError> {
        self.set_keyring_password(&totp_secret_key(username), secret.as_str())
    }

    fn is_mfa_enabled(&self, username: &str) -> Result<bool, SecurityError> {
        Ok(self.read_totp_secret_index()?.contains(username))
    }

    fn read_totp_secret_index(&self) -> Result<BTreeSet<String>, SecurityError> {
        let Some(payload) = self.get_keyring_password(TOTP_SECRET_INDEX)? else {
            return Ok(BTreeSet::new());
        };

        serde_json::from_str::<Vec<String>>(&payload)
            .map(|usernames| usernames.into_iter().collect())
            .map_err(|e| SecurityError::KeyringError(e.to_string()))
    }

    fn write_totp_secret_index(&self, usernames: &BTreeSet<String>) -> Result<(), SecurityError> {
        let payload = serde_json::to_string(&usernames.iter().collect::<Vec<_>>())
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        self.set_keyring_password(TOTP_SECRET_INDEX, &payload)
    }

    fn read_totp_secret_versions(&self, username: &str) -> Result<BTreeSet<u32>, SecurityError> {
        let versions_key = totp_secret_versions_key(username);
        let Some(payload) = self.get_keyring_password(&versions_key)? else {
            return Ok(BTreeSet::new());
        };

        serde_json::from_str::<Vec<u32>>(&payload)
            .map(|versions| versions.into_iter().collect())
            .map_err(|e| SecurityError::KeyringError(e.to_string()))
    }

    fn write_totp_secret_versions(
        &self,
        username: &str,
        versions: &BTreeSet<u32>,
    ) -> Result<(), SecurityError> {
        let versions_key = totp_secret_versions_key(username);
        let payload = serde_json::to_string(&versions.iter().collect::<Vec<_>>())
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        self.set_keyring_password(&versions_key, &payload)
    }

    fn read_active_totp_secret_version(
        &self,
        username: &str,
    ) -> Result<Option<u32>, SecurityError> {
        let active_key = totp_secret_active_key(username);
        let Some(payload) = self.get_keyring_password(&active_key)? else {
            return Ok(None);
        };

        payload
            .parse::<u32>()
            .map(Some)
            .map_err(|e| SecurityError::KeyringError(e.to_string()))
    }

    fn write_active_totp_secret_version(
        &self,
        username: &str,
        version: u32,
    ) -> Result<(), SecurityError> {
        let active_key = totp_secret_active_key(username);
        self.set_keyring_password(&active_key, &version.to_string())
    }

    fn read_credential_index(&self) -> Result<BTreeSet<String>, SecurityError> {
        let Some(payload) = self.get_keyring_password(CREDENTIAL_INDEX)? else {
            return Ok(BTreeSet::new());
        };

        serde_json::from_str::<Vec<String>>(&payload)
            .map(|usernames| usernames.into_iter().collect())
            .map_err(|e| SecurityError::KeyringError(e.to_string()))
    }

    fn write_credential_index(&self, usernames: &BTreeSet<String>) -> Result<(), SecurityError> {
        let payload = serde_json::to_string(&usernames.iter().collect::<Vec<_>>())
            .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
        self.set_keyring_password(CREDENTIAL_INDEX, &payload)
    }

    fn get_keyring_password(&self, key: &str) -> Result<Option<String>, SecurityError> {
        match &self.backend {
            CredentialBackend::System => {
                let entry = Entry::new(&self.service, key)
                    .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
                match entry.get_password() {
                    Ok(payload) => Ok(Some(payload)),
                    Err(KeyringBackendError::NoEntry) => Ok(None),
                    Err(error) => Err(SecurityError::KeyringError(error.to_string())),
                }
            }
            CredentialBackend::InMemory(entries) => entries
                .lock()
                .map(|entries| entries.get(key).cloned())
                .map_err(|e| SecurityError::KeyringError(e.to_string())),
        }
    }

    fn set_keyring_password(&self, key: &str, password: &str) -> Result<(), SecurityError> {
        match &self.backend {
            CredentialBackend::System => {
                let entry = Entry::new(&self.service, key)
                    .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
                entry
                    .set_password(password)
                    .map_err(|e| SecurityError::KeyringError(e.to_string()))
            }
            CredentialBackend::InMemory(entries) => entries
                .lock()
                .map(|mut entries| {
                    entries.insert(key.to_string(), password.to_string());
                })
                .map_err(|e| SecurityError::KeyringError(e.to_string())),
        }
    }

    fn delete_keyring_password(&self, key: &str) -> Result<(), SecurityError> {
        match &self.backend {
            CredentialBackend::System => {
                let entry = Entry::new(&self.service, key)
                    .map_err(|e| SecurityError::KeyringError(e.to_string()))?;
                match entry.delete_password() {
                    Ok(()) | Err(KeyringBackendError::NoEntry) => Ok(()),
                    Err(error) => Err(SecurityError::KeyringError(error.to_string())),
                }
            }
            CredentialBackend::InMemory(entries) => entries
                .lock()
                .map(|mut entries| {
                    entries.remove(key);
                })
                .map_err(|e| SecurityError::KeyringError(e.to_string())),
        }
    }
}

enum CredentialBackend {
    System,
    InMemory(Arc<Mutex<BTreeMap<String, String>>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MfaAuditEventKind {
    VerificationSucceeded,
    VerificationFailed,
    RateLimited,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MfaAuditEvent {
    pub username: String,
    pub kind: MfaAuditEventKind,
    pub unix_timestamp: u64,
}

/// Tracks users that have recently passed MFA verification.
pub struct MfaGuard {
    session_ttl_seconds: u64,
    verified_sessions: Mutex<BTreeMap<String, u64>>,
    failed_attempts: Mutex<BTreeMap<String, Vec<u64>>>,
    audit_events: Mutex<Vec<MfaAuditEvent>>,
}

impl MfaGuard {
    pub fn new(session_ttl_seconds: u64) -> Self {
        Self {
            session_ttl_seconds,
            verified_sessions: Mutex::new(BTreeMap::new()),
            failed_attempts: Mutex::new(BTreeMap::new()),
            audit_events: Mutex::new(Vec::new()),
        }
    }

    pub fn mark_verified(&self, username: &str, unix_timestamp: u64) {
        let expires_at = unix_timestamp.saturating_add(self.session_ttl_seconds);
        if let Ok(mut sessions) = self.verified_sessions.lock() {
            sessions.insert(username.to_string(), expires_at);
        }
    }

    pub fn is_verified(&self, username: &str, unix_timestamp: u64) -> bool {
        self.verified_sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(username).copied())
            .is_some_and(|expires_at| unix_timestamp <= expires_at)
    }

    pub fn record_failed_attempt(&self, username: &str, unix_timestamp: u64) {
        if let Ok(mut attempts_by_user) = self.failed_attempts.lock() {
            let attempts = attempts_by_user.entry(username.to_string()).or_default();
            prune_mfa_attempts(attempts, unix_timestamp);
            attempts.push(unix_timestamp);
        }
    }

    pub fn clear_failed_attempts(&self, username: &str) {
        if let Ok(mut attempts_by_user) = self.failed_attempts.lock() {
            attempts_by_user.remove(username);
        }
    }

    pub fn is_rate_limited(&self, username: &str, unix_timestamp: u64) -> bool {
        self.failed_attempts
            .lock()
            .ok()
            .and_then(|mut attempts_by_user| {
                attempts_by_user.get_mut(username).map(|attempts| {
                    prune_mfa_attempts(attempts, unix_timestamp);
                    attempts.len() >= MFA_RATE_LIMIT_MAX_ATTEMPTS
                })
            })
            .unwrap_or(false)
    }

    pub fn record_audit_event(&self, username: &str, kind: MfaAuditEventKind, unix_timestamp: u64) {
        if let Ok(mut audit_events) = self.audit_events.lock() {
            audit_events.push(MfaAuditEvent {
                username: username.to_string(),
                kind,
                unix_timestamp,
            });
        }
    }

    pub fn audit_events(&self) -> Vec<MfaAuditEvent> {
        self.audit_events
            .lock()
            .map(|audit_events| audit_events.clone())
            .unwrap_or_default()
    }
}

impl Default for MfaGuard {
    fn default() -> Self {
        Self::new(MFA_SESSION_TTL_SECONDS)
    }
}

fn totp_secret_key(username: &str) -> String {
    format!("{TOTP_SECRET_PREFIX}{username}")
}

fn totp_secret_active_key(username: &str) -> String {
    format!("{TOTP_SECRET_ACTIVE_PREFIX}{username}")
}

fn totp_secret_version_key(username: &str, version: u32) -> String {
    format!("{TOTP_SECRET_VERSION_PREFIX}{username}:v{version}")
}

fn totp_secret_versions_key(username: &str) -> String {
    format!("{TOTP_SECRET_VERSIONS_PREFIX}{username}")
}

fn current_unix_timestamp() -> Result<u64, SecurityError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|e| SecurityError::KeyringError(e.to_string()))
}

fn no_keyring_entry_error(key: &str) -> SecurityError {
    SecurityError::KeyringError(format!("No keyring entry found for {key}"))
}

fn prune_mfa_attempts(attempts: &mut Vec<u64>, unix_timestamp: u64) {
    let cutoff = unix_timestamp.saturating_sub(MFA_RATE_LIMIT_WINDOW_SECONDS);
    attempts.retain(|attempt| *attempt > cutoff);
}
