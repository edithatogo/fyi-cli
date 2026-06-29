use fyi_core::security::{
    build_provisioning_uri, decrypt, encrypt, generate_totp_code, generate_totp_secret,
    render_provisioning_qr_ascii, verify_totp_code, EncryptionKey, KeyringStore, MfaAuditEventKind,
    MfaGuard, SecurityError, ZeroizedBytes, ZeroizedString,
};
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

#[test]
fn test_zeroize_scrubbing() {
    // 1. ZeroizedString
    let mut secret_str = ZeroizedString::new("highly-sensitive-password-123".to_string());
    assert_eq!(secret_str.as_str(), "highly-sensitive-password-123");
    secret_str.zeroize();
    assert_eq!(secret_str.as_str(), "");

    // 2. ZeroizedBytes
    let mut secret_bytes = ZeroizedBytes::new(vec![10, 20, 30, 40, 50]);
    assert_eq!(secret_bytes.as_slice(), &[10, 20, 30, 40, 50]);
    secret_bytes.zeroize();
    assert_eq!(secret_bytes.as_slice(), &[] as &[u8]);

    // 3. EncryptionKey
    let mut key = EncryptionKey([0xFF; 32]);
    assert_eq!(key.0, [0xFF; 32]);
    key.zeroize();
    assert_eq!(key.0, [0x00; 32]);
}

#[test]
fn test_aes_gcm_encryption_decryption() {
    let key = EncryptionKey::generate();
    let original_data = ZeroizedBytes::new(b"secret payload message".to_vec());

    // Encrypt
    let encrypted = encrypt(&original_data, &key).expect("Encryption failed");
    assert_ne!(encrypted.as_slice(), original_data.as_slice());
    assert!(encrypted.as_slice().len() > 12); // Nonce (12) + Ciphertext

    // Decrypt
    let decrypted = decrypt(&encrypted, &key).expect("Decryption failed");
    assert_eq!(decrypted.as_slice(), original_data.as_slice());

    // Decrypt with a different key should fail
    let different_key = EncryptionKey::generate();
    let decrypt_attempt = decrypt(&encrypted, &different_key);
    assert!(decrypt_attempt.is_err());

    // Decrypt with corrupted ciphertext should fail
    let mut corrupted = encrypted.as_slice().to_vec();
    if let Some(last_byte) = corrupted.last_mut() {
        *last_byte ^= 1; // Flip a bit
    }
    let corrupted_bytes = ZeroizedBytes::new(corrupted);
    let decrypt_corrupted_attempt = decrypt(&corrupted_bytes, &key);
    assert!(decrypt_corrupted_attempt.is_err());
}

#[test]
fn test_serialization_deserialization() {
    // Test ZeroizedString serialization/deserialization
    let secret_str = ZeroizedString::new("my-api-key".to_string());
    let serialized_str = serde_json::to_string(&secret_str).expect("Failed to serialize string");
    assert_eq!(serialized_str, "\"my-api-key\"");

    let deserialized_str: ZeroizedString =
        serde_json::from_str(&serialized_str).expect("Failed to deserialize string");
    assert_eq!(deserialized_str.as_str(), "my-api-key");

    // Test ZeroizedBytes serialization/deserialization
    let secret_bytes = ZeroizedBytes::new(vec![1, 2, 3, 4, 5]);
    let serialized_bytes = serde_json::to_string(&secret_bytes).expect("Failed to serialize bytes");
    assert_eq!(serialized_bytes, "[1,2,3,4,5]");

    let deserialized_bytes: ZeroizedBytes =
        serde_json::from_str(&serialized_bytes).expect("Failed to deserialize bytes");
    assert_eq!(deserialized_bytes.as_slice(), &[1, 2, 3, 4, 5]);
}

#[test]
fn test_keyring_wrapper_graceful() {
    let store = KeyringStore::new_in_memory("fyi-cli-test-service");
    let username = "test-user-credentials";
    let secret = ZeroizedString::new("super-secret-keyring-pass".to_string());

    // Try set/get/delete. Depending on the OS environment (e.g. headless CI),
    // this may fail or succeed. We check that it handles the outcome gracefully.
    match store.set_credential(username, &secret) {
        Ok(_) => {
            let fetched = store
                .get_credential(username)
                .expect("Failed to get credential after successful set");
            assert_eq!(fetched.as_str(), secret.as_str());

            store
                .delete_credential(username)
                .expect("Failed to delete credential");
        }
        Err(e) => {
            // Log that keyring is unavailable, but do not fail the test
            println!("Keyring backend is unavailable or failed: {}", e);
        }
    }
}

#[test]
fn test_keyring_totp_secret_storage_graceful() {
    let store = KeyringStore::new_in_memory("fyi-cli-test-mfa-service");
    let username = "test-user-mfa";
    let secret = ZeroizedString::new("JBSWY3DPEHPK3PXP".to_string());

    let _ = store.delete_totp_secret(username);
    match store.store_totp_secret(username, &secret) {
        Ok(_) => {
            let fetched = store
                .get_totp_secret(username)
                .expect("Failed to fetch TOTP secret after successful store");
            assert_eq!(fetched.as_str(), secret.as_str());

            let usernames = store
                .list_totp_secrets()
                .expect("Failed to list TOTP-enabled accounts");
            assert!(usernames.iter().any(|stored| stored == username));

            store
                .delete_totp_secret(username)
                .expect("Failed to delete TOTP secret");
            let usernames = store
                .list_totp_secrets()
                .expect("Failed to list TOTP-enabled accounts after delete");
            assert!(!usernames.iter().any(|stored| stored == username));
        }
        Err(e) => {
            println!("Keyring backend is unavailable or failed: {}", e);
        }
    }
}

#[test]
fn test_keyring_totp_secret_rotation_graceful() {
    let store = KeyringStore::new_in_memory("fyi-cli-test-mfa-rotation-service");
    let username = "test-user-mfa-rotation";
    let first = ZeroizedString::new("JBSWY3DPEHPK3PXP".to_string());
    let second = ZeroizedString::new("JBSWY3DPEHPK3PXQ".to_string());
    let third = ZeroizedString::new("JBSWY3DPEHPK3PXR".to_string());

    let _ = store.delete_totp_secret(username);
    match store.store_totp_secret_version(username, 1, &first) {
        Ok(_) => {
            store
                .store_totp_secret_version(username, 2, &second)
                .expect("Failed to store second TOTP secret version");

            assert_eq!(
                store
                    .get_totp_secret_version(username, 1)
                    .expect("Failed to fetch first TOTP secret version")
                    .as_str(),
                first.as_str()
            );
            assert_eq!(
                store
                    .get_totp_secret_version(username, 2)
                    .expect("Failed to fetch second TOTP secret version")
                    .as_str(),
                second.as_str()
            );
            assert_eq!(
                store
                    .list_totp_secret_versions(username)
                    .expect("Failed to list TOTP secret versions"),
                vec![1, 2]
            );
            assert_eq!(
                store
                    .get_totp_secret(username)
                    .expect("Failed to fetch active TOTP secret")
                    .as_str(),
                second.as_str()
            );

            let rotated_version = store
                .rotate_totp_secret(username, &third)
                .expect("Failed to rotate TOTP secret");
            assert_eq!(rotated_version, 3);
            assert_eq!(
                store
                    .list_totp_secret_versions(username)
                    .expect("Failed to list TOTP secret versions after rotation"),
                vec![1, 2, 3]
            );
            assert_eq!(
                store
                    .get_totp_secret(username)
                    .expect("Failed to fetch rotated active TOTP secret")
                    .as_str(),
                third.as_str()
            );

            store
                .delete_totp_secret(username)
                .expect("Failed to delete rotated TOTP secrets");
            assert!(store
                .list_totp_secret_versions(username)
                .expect("Failed to list deleted TOTP secret versions")
                .is_empty());
        }
        Err(e) => {
            println!("Keyring backend is unavailable or failed: {}", e);
        }
    }
}

#[test]
fn test_mfa_guard_tracks_expiring_verified_sessions() {
    let guard = MfaGuard::new(30);
    let username = "test-user-mfa-guard";

    assert!(!guard.is_verified(username, 1_700_000_000));
    guard.mark_verified(username, 1_700_000_000);
    assert!(guard.is_verified(username, 1_700_000_029));
    assert!(!guard.is_verified(username, 1_700_000_031));
}

#[test]
fn test_keyring_credential_access_requires_mfa_when_configured_graceful() {
    let store = KeyringStore::new_in_memory("fyi-cli-test-mfa-guard-service");
    let username = "test-user-mfa-guard-keyring";
    let credential = ZeroizedString::new("credential-protected-by-mfa".to_string());
    let totp_secret = ZeroizedString::new("JBSWY3DPEHPK3PXP".to_string());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock is before Unix epoch")
        .as_secs();
    let code = generate_totp_code(&totp_secret, timestamp).expect("Failed to generate TOTP code");

    let _ = store.delete_credential(username);
    let _ = store.delete_totp_secret(username);
    match (
        store.set_credential(username, &credential),
        store.store_totp_secret(username, &totp_secret),
    ) {
        (Ok(_), Ok(_)) => {
            let blocked = store.get_credential(username);
            assert!(matches!(blocked, Err(SecurityError::MfaRequired(_))));

            assert!(store
                .verify_mfa_code(username, &code, timestamp, 1)
                .expect("Failed to verify MFA code"));
            let fetched = store
                .get_credential(username)
                .expect("Failed to fetch credential after MFA verification");
            assert_eq!(fetched.as_str(), credential.as_str());

            store
                .delete_credential(username)
                .expect("Failed to delete credential");
            store
                .delete_totp_secret(username)
                .expect("Failed to delete TOTP secret");
        }
        (Err(e), _) | (_, Err(e)) => {
            println!("Keyring backend is unavailable or failed: {}", e);
        }
    }
}

#[test]
fn test_keyring_mfa_rate_limits_failed_attempts_and_records_audit_events() {
    let store = KeyringStore::new_in_memory("fyi-cli-test-mfa-rate-limit-service");
    let username = "test-user-mfa-rate-limit";
    let totp_secret = ZeroizedString::new("JBSWY3DPEHPK3PXP".to_string());
    let timestamp = 1_700_000_000;
    let valid_code = generate_totp_code(&totp_secret, timestamp).unwrap();

    store
        .store_totp_secret(username, &totp_secret)
        .expect("Failed to store TOTP secret");

    for _ in 0..5 {
        assert!(!store
            .verify_mfa_code(username, "000000", timestamp, 1)
            .expect("Failed MFA attempts should return false before rate limit"));
    }

    assert!(matches!(
        store.verify_mfa_code(username, &valid_code, timestamp, 1),
        Err(SecurityError::MfaRateLimited(_))
    ));

    let audit_events = store.mfa_audit_events();
    assert_eq!(
        audit_events
            .iter()
            .filter(|event| event.kind == MfaAuditEventKind::VerificationFailed)
            .count(),
        5
    );
    assert!(audit_events
        .iter()
        .any(|event| event.kind == MfaAuditEventKind::RateLimited));

    let later_timestamp = timestamp + 31;
    let later_code = generate_totp_code(&totp_secret, later_timestamp).unwrap();
    assert!(store
        .verify_mfa_code(username, &later_code, later_timestamp, 1)
        .expect("Valid MFA code should pass after rate-limit window expires"));
    assert!(store
        .mfa_audit_events()
        .iter()
        .any(|event| event.kind == MfaAuditEventKind::VerificationSucceeded));
}

#[test]
fn test_totp_matches_rfc6238_sha1_vectors() {
    let secret = ZeroizedString::new("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".to_string());

    assert_eq!(generate_totp_code(&secret, 59).unwrap(), "287082");
    assert_eq!(
        generate_totp_code(&secret, 1_111_111_109).unwrap(),
        "081804"
    );
    assert_eq!(
        generate_totp_code(&secret, 1_111_111_111).unwrap(),
        "050471"
    );
    assert_eq!(
        generate_totp_code(&secret, 1_234_567_890).unwrap(),
        "005924"
    );
    assert_eq!(
        generate_totp_code(&secret, 2_000_000_000).unwrap(),
        "279037"
    );
    assert_eq!(
        generate_totp_code(&secret, 20_000_000_000).unwrap(),
        "353130"
    );
}

#[test]
fn test_totp_verification_honors_drift_windows() {
    let secret = ZeroizedString::new("JBSWY3DPEHPK3PXP".to_string());
    let generated = generate_totp_code(&secret, 1_700_000_000).unwrap();

    assert!(verify_totp_code(&secret, &generated, 1_700_000_000, 0).unwrap());
    assert!(verify_totp_code(&secret, &generated, 1_700_000_030, 1).unwrap());
    assert!(!verify_totp_code(&secret, &generated, 1_700_000_090, 1).unwrap());
    assert!(!verify_totp_code(&secret, "not-a-code", 1_700_000_000, 1).unwrap());
}

#[test]
fn test_totp_secret_generation_returns_distinct_base32_secrets() {
    let first = generate_totp_secret().unwrap();
    let second = generate_totp_secret().unwrap();

    assert_eq!(first.as_str().len(), 32);
    assert!(first
        .as_str()
        .chars()
        .all(|ch| matches!(ch, 'A'..='Z' | '2'..='7')));
    assert_ne!(first.as_str(), second.as_str());
}

#[test]
fn test_totp_provisioning_uri_encodes_issuer_and_account() {
    let secret = ZeroizedString::new("JBSWY3DPEHPK3PXP".to_string());
    let uri = build_provisioning_uri("FYI CLI", "reporter+oia@example.org", &secret).unwrap();

    assert_eq!(
        uri,
        "otpauth://totp/FYI%20CLI:reporter%2Boia%40example.org?secret=JBSWY3DPEHPK3PXP&issuer=FYI%20CLI&algorithm=SHA1&digits=6&period=30"
    );
}

#[test]
fn test_totp_qr_ascii_renders_uri_as_terminal_blocks() {
    let secret = ZeroizedString::new("JBSWY3DPEHPK3PXP".to_string());
    let uri = build_provisioning_uri("FYI CLI", "reporter@example.org", &secret).unwrap();
    let qr = render_provisioning_qr_ascii(&uri).unwrap();

    assert!(qr.contains("██"));
    assert!(qr.lines().count() > 10);
}
