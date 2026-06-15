use fyi_core::security::{
    decrypt, encrypt, EncryptionKey, KeyringStore, ZeroizedBytes, ZeroizedString,
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
    assert_eq!(secret_bytes.as_slice(), &[]);

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
