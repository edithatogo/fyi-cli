use proptest::prelude::*;
use fyi_core::security::{
    encrypt, decrypt, EncryptionKey, ZeroizedBytes, ZeroizedString, SecurityError
};
use fyi_core::api::{
    AlaveteliRequest, AlaveteliCorrespondence, CorrespondenceDirection
};
use fyi_core::db::DbPool;
use zeroize::Zeroize;

// Helper to sanitize arbitrary generated strings to prevent SQLite null byte issues.
fn sanitize_str(s: String) -> String {
    s.replace('\0', " ")
}

fn sanitize_opt_str(opt: Option<String>) -> Option<String> {
    opt.map(sanitize_str)
}

fn sanitize_opt_vec_str(opt: Option<Vec<String>>) -> Option<Vec<String>> {
    opt.map(|v| v.into_iter().map(sanitize_str).collect())
}

// Custom strategies for our types.
fn arb_direction() -> impl Strategy<Value = CorrespondenceDirection> {
    prop_oneof![
        Just(CorrespondenceDirection::Request),
        Just(CorrespondenceDirection::Response),
    ]
}

fn arb_request() -> impl Strategy<Value = AlaveteliRequest> {
    (
        any::<i64>(),
        any::<String>().prop_map(sanitize_str),
        any::<String>().prop_map(sanitize_str),
        any::<Option<String>>().prop_map(sanitize_opt_str),
        any::<Option<String>>().prop_map(sanitize_opt_str),
        any::<Option<String>>().prop_map(sanitize_opt_str),
        any::<Option<String>>().prop_map(sanitize_opt_str),
        any::<Option<String>>().prop_map(sanitize_opt_str),
        any::<Option<Vec<String>>>().prop_map(sanitize_opt_vec_str),
    ).prop_map(|(id, title, body, user_name, status, created_at, updated_at, url, tags)| {
        AlaveteliRequest {
            id,
            title,
            body,
            user_name,
            status,
            created_at,
            updated_at,
            url,
            tags,
        }
    })
}

fn arb_correspondence() -> impl Strategy<Value = AlaveteliCorrespondence> {
    (
        arb_direction(),
        any::<String>().prop_map(sanitize_str),
        // Generate alphanumeric string for sent_at to avoid database sorting weirdness with arbitrary byte-like chars
        "[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z",
        any::<Option<String>>().prop_map(sanitize_opt_str),
        any::<Option<Vec<String>>>().prop_map(sanitize_opt_vec_str),
    ).prop_map(|(direction, body, sent_at, state, attachments)| {
        AlaveteliCorrespondence {
            direction,
            body,
            sent_at,
            state,
            attachments,
        }
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn test_encryption_roundtrip(plaintext_bytes in any::<Vec<u8>>(), key_bytes in any::<[u8; 32]>()) {
        let plaintext = ZeroizedBytes::new(plaintext_bytes.clone());
        let key = EncryptionKey(key_bytes);
        
        let encrypted = encrypt(&plaintext, &key).expect("Encryption failed");
        
        // Assert nonce prefix of 12 bytes plus ciphertext
        assert_eq!(encrypted.as_slice().len(), plaintext_bytes.len() + 12);
        
        let decrypted = decrypt(&encrypted, &key).expect("Decryption failed");
        assert_eq!(decrypted.as_slice(), plaintext_bytes.as_slice());
    }

    #[test]
    fn test_encryption_integrity_failure(
        plaintext_bytes in any::<Vec<u8>>(),
        key_bytes in any::<[u8; 32]>(),
        corruption_index in any::<usize>()
    ) {
        let plaintext = ZeroizedBytes::new(plaintext_bytes);
        let key = EncryptionKey(key_bytes);
        let encrypted = encrypt(&plaintext, &key).expect("Encryption failed");
        let encrypted_slice = encrypted.as_slice();
        
        if !encrypted_slice.is_empty() {
            let idx = corruption_index % encrypted_slice.len();
            let mut corrupted_vec = encrypted_slice.to_vec();
            corrupted_vec[idx] ^= 0xFF; // Mutate ciphertext/nonce byte
            
            let corrupted = ZeroizedBytes::new(corrupted_vec);
            let decrypt_result = decrypt(&corrupted, &key);
            assert!(decrypt_result.is_err());
        }
    }

    #[test]
    fn test_encryption_key_mismatch(
        plaintext_bytes in any::<Vec<u8>>(),
        key_bytes_1 in any::<[u8; 32]>(),
        key_bytes_2 in any::<[u8; 32]>()
    ) {
        // Skip if keys are identical by chance
        prop_assume!(key_bytes_1 != key_bytes_2);
        
        let plaintext = ZeroizedBytes::new(plaintext_bytes);
        let key1 = EncryptionKey(key_bytes_1);
        let key2 = EncryptionKey(key_bytes_2);
        
        let encrypted = encrypt(&plaintext, &key1).expect("Encryption failed");
        let decrypt_result = decrypt(&encrypted, &key2);
        assert!(decrypt_result.is_err());
    }

    #[test]
    fn test_encryption_too_short(
        short_bytes in prop::collection::vec(any::<u8>(), 0..12),
        key_bytes in any::<[u8; 32]>()
    ) {
        let payload = ZeroizedBytes::new(short_bytes);
        let key = EncryptionKey(key_bytes);
        let decrypt_result = decrypt(&payload, &key);
        
        assert!(matches!(decrypt_result, Err(SecurityError::CiphertextTooShort(_))));
    }

    #[test]
    fn test_database_request_roundtrip(request in arb_request()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = DbPool::new_in_memory().await.unwrap();
            db.run_migrations().await.unwrap();
            
            db.insert_request(&request).await.unwrap();
            let retrieved = db.get_request(request.id).await.unwrap().unwrap();
            assert_eq!(retrieved, request);
        });
    }

    #[test]
    fn test_database_correspondence_chronological_ordering(
        request in arb_request(),
        mut correspondences in prop::collection::vec(arb_correspondence(), 1..10)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = DbPool::new_in_memory().await.unwrap();
            db.run_migrations().await.unwrap();
            
            // Insert parent request first to satisfy foreign key or logical schema design
            db.insert_request(&request).await.unwrap();
            
            // Insert correspondences
            for corr in &correspondences {
                db.insert_correspondence(request.id, corr).await.unwrap();
            }
            
            // Query correspondences
            let mut retrieved = db.get_correspondence_for_request(request.id).await.unwrap();
            
            // Sort expected locally using stability by sent_at
            correspondences.sort_by(|a, b| a.sent_at.cmp(&b.sent_at));
            
            assert_eq!(retrieved.len(), correspondences.len());
            // Note: Since sqlx retrieves ordered by sent_at ASC, let's verify sorting match
            for (r, e) in retrieved.iter().zip(correspondences.iter()) {
                assert_eq!(r.direction, e.direction);
                assert_eq!(r.body, e.body);
                assert_eq!(r.sent_at, e.sent_at);
                assert_eq!(r.state, e.state);
                assert_eq!(r.attachments, e.attachments);
            }
        });
    }
}
