use fyi_core::security::{
    decrypt, encrypt, generate_totp_code, generate_totp_secret, verify_totp_code, EncryptionKey,
    KeyringStore, ZeroizedBytes, ZeroizedString,
};
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
    let store = KeyringStore::new("fyi-cli-test-service");
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
