//! Durable journal tests use private temporary directories and synthetic inputs.

use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self(std::env::temp_dir().join(format!(
            "fyi-mcp-journal-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scope<'a>(instance: &'a str, key: &'a str) -> WriteScope<'a> {
    WriteScope::new(instance, "remote_create_request", key).unwrap()
}

async fn events(journal: &WriteJournal) -> Vec<String> {
    sqlx::query_scalar("SELECT event FROM mcp_write_audit ORDER BY sequence")
        .fetch_all(&journal.pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn reservations_survive_restart_and_keep_unknown_outcomes_blocked() {
    let directory = Directory::new();
    let request = scope("synthetic", "retry-same-key");
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    journal.check_prepare(&request, false).await.unwrap();
    journal.reserve(&request).await.unwrap();
    journal.pool.close().await;

    let restarted = WriteJournal::open(&directory.0).await.unwrap();
    assert_eq!(
        restarted.check_prepare(&request, false).await.unwrap_err(),
        REPLAY_BLOCKED
    );
    assert_eq!(
        restarted.reserve(&request).await.unwrap_err(),
        REPLAY_BLOCKED
    );
    assert_eq!(
        events(&restarted).await,
        ["prepare", "commit", "replay", "replay"]
    );
    restarted.pool.close().await;
}

#[tokio::test]
async fn concurrent_process_connections_admit_exactly_one_dispatch() {
    let directory = Directory::new();
    let first = WriteJournal::open(&directory.0).await.unwrap();
    let second = WriteJournal::open(&directory.0).await.unwrap();
    let request = scope("synthetic", "simultaneous");
    let (left, right) = tokio::join!(first.reserve(&request), second.reserve(&request));
    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    assert_eq!(left.err().or(right.err()).unwrap(), REPLAY_BLOCKED);
    assert_eq!(events(&first).await, ["commit", "replay"]);
    first.pool.close().await;
    second.pool.close().await;
}

#[tokio::test]
async fn completion_and_cached_replay_require_a_matching_durable_reservation() {
    let directory = Directory::new();
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    let request = scope("synthetic", "completed");
    assert!(journal.complete(&request, true).await.is_err());
    assert!(journal.check_prepare(&request, true).await.is_err());
    journal.reserve(&request).await.unwrap();
    assert!(journal.check_prepare(&request, true).await.is_err());
    journal.complete(&request, true).await.unwrap();
    journal.check_prepare(&request, true).await.unwrap();
    assert!(journal.complete(&request, true).await.is_err());
    assert!(journal.reserve(&request).await.is_err());
    assert!(events(&journal).await.contains(&"success".to_string()));
    journal.pool.close().await;
    let restarted = WriteJournal::open(&directory.0).await.unwrap();
    assert!(restarted.check_prepare(&request, false).await.is_err());
    restarted.pool.close().await;
}

#[tokio::test]
async fn failed_or_cancelled_outcomes_never_release_the_durable_key() {
    let directory = Directory::new();
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    let request = scope("synthetic", "uncertain");
    journal.reserve(&request).await.unwrap();
    journal.complete(&request, false).await.unwrap();
    assert!(journal.check_prepare(&request, true).await.is_err());
    assert!(journal.check_prepare(&request, false).await.is_err());
    assert!(journal.reserve(&request).await.is_err());
    assert!(events(&journal).await.contains(&"failure".to_string()));
    journal.pool.close().await;
}

#[tokio::test]
async fn audit_failure_rolls_back_reservation_and_blocks_dispatch() {
    let directory = Directory::new();
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    sqlx::query("CREATE TRIGGER audit_unavailable BEFORE INSERT ON mcp_write_audit BEGIN SELECT RAISE(ABORT, 'synthetic-private-storage-error'); END")
        .execute(&journal.pool).await.unwrap();
    let request = scope("synthetic", "storage-failure");
    assert_eq!(
        journal.reserve(&request).await.unwrap_err(),
        STORAGE_UNAVAILABLE
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mcp_write_attempts")
        .fetch_one(&journal.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    sqlx::query("DROP TRIGGER audit_unavailable")
        .execute(&journal.pool)
        .await
        .unwrap();
    journal.reserve(&request).await.unwrap();
    journal.pool.close().await;
}

#[tokio::test]
async fn storage_and_audit_capacity_fail_closed_without_evicting_attempts() {
    let directory = Directory::new();
    let journal = WriteJournal::open_with_limits(&directory.0, 1, 4)
        .await
        .unwrap();
    let first = scope("synthetic", "one");
    let second = scope("synthetic", "two");
    journal.reserve(&first).await.unwrap();
    assert_eq!(
        journal.reserve(&second).await.unwrap_err(),
        CAPACITY_REACHED
    );
    assert_eq!(journal.reserve(&first).await.unwrap_err(), REPLAY_BLOCKED);
    journal
        .record_rejection("remote_create_request")
        .await
        .unwrap();
    assert_eq!(
        journal
            .record_rejection("remote_create_request")
            .await
            .unwrap_err(),
        CAPACITY_REACHED
    );
    assert_eq!(
        journal.reserve(&second).await.unwrap_err(),
        CAPACITY_REACHED
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mcp_write_attempts")
        .fetch_one(&journal.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    journal.pool.close().await;
}

#[tokio::test]
async fn opaque_scopes_preserve_identity_without_ambiguous_concatenation_or_raw_data() {
    let directory = Directory::new();
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    let mut tags = std::collections::BTreeSet::new();
    for instance in [
        "synthetic-private-a",
        "synthetic-private-ab",
        "SYNTHETIC_PRIVATE_A",
    ] {
        for operation in [
            "remote_create_request",
            "remote_add_correspondence",
            "remote_update_state",
        ] {
            for key in [
                "synthetic-private-bc",
                "synthetic-private-c",
                "synthetic-private-b_c",
            ] {
                let request = WriteScope::new(instance, operation, key).unwrap();
                assert!(tags.insert(journal.tag(&request)));
                journal.reserve(&request).await.unwrap();
            }
        }
    }
    assert_eq!(tags.len(), 27);
    journal.pool.close().await;
    let bytes = std::fs::read(directory.0.join(DATABASE_FILE)).unwrap();
    assert!(!bytes
        .windows(b"synthetic-private".len())
        .any(|chunk| chunk == b"synthetic-private"));
    let restarted = WriteJournal::open(&directory.0).await.unwrap();
    assert_eq!(
        journal.tag(&scope("synthetic", "stable")),
        restarted.tag(&scope("synthetic", "stable"))
    );
    restarted.pool.close().await;
}

#[tokio::test]
async fn missing_or_changed_journal_files_are_not_silently_reinitialized() {
    let directory = Directory::new();
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    journal
        .reserve(&scope("synthetic", "persist"))
        .await
        .unwrap();
    journal.pool.close().await;
    std::fs::remove_file(directory.0.join(DATABASE_FILE)).unwrap();
    assert!(WriteJournal::open(&directory.0).await.is_err());
    assert!(!directory.0.join(DATABASE_FILE).exists());
}

#[tokio::test]
async fn unknown_format_and_corrupt_storage_fail_privately() {
    let directory = Directory::new();
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    sqlx::query("PRAGMA user_version = 99")
        .execute(&journal.pool)
        .await
        .unwrap();
    journal.pool.close().await;
    assert!(WriteJournal::open(&directory.0).await.is_err());
    std::fs::write(
        directory.0.join(DATABASE_FILE),
        b"synthetic-private-corruption",
    )
    .unwrap();
    let error = WriteJournal::open(&directory.0).await.unwrap_err();
    assert_eq!(error, STORAGE_UNAVAILABLE);
    assert!(!error.contains("synthetic"));
    assert!(!error.contains(directory.0.to_str().unwrap()));
}

#[tokio::test]
async fn closed_storage_never_allows_preparation_reservation_or_outcome_publication() {
    let directory = Directory::new();
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    journal.pool.close().await;
    let request = scope("synthetic", "closed");
    assert_eq!(
        journal.check_prepare(&request, false).await.unwrap_err(),
        STORAGE_UNAVAILABLE
    );
    assert_eq!(
        journal.reserve(&request).await.unwrap_err(),
        STORAGE_UNAVAILABLE
    );
    assert_eq!(
        journal.complete(&request, true).await.unwrap_err(),
        STORAGE_UNAVAILABLE
    );
    assert_eq!(
        journal
            .record_rejection("remote_create_request")
            .await
            .unwrap_err(),
        STORAGE_UNAVAILABLE
    );
}

#[test]
fn journal_scopes_reject_unknown_operations_and_unbounded_inputs() {
    for (instance, operation, key) in [
        ("", "remote_create_request", "key"),
        ("synthetic", "unknown", "key"),
        ("synthetic", "remote_create_request", ""),
        ("synthetic", "remote_create_request", "bad/key"),
    ] {
        assert!(WriteScope::new(instance, operation, key).is_err());
    }
    assert!(WriteScope::new(&"a".repeat(65), "remote_create_request", "key").is_err());
    assert!(WriteScope::new("synthetic", "remote_create_request", &"k".repeat(129)).is_err());
}

#[tokio::test]
async fn invalid_paths_markers_and_changed_keys_fail_closed() {
    assert!(WriteJournal::open(Path::new("relative-state"))
        .await
        .is_err());
    let directory = Directory::new();
    assert!(WriteJournal::open_with_limits(&directory.0, 0, 10)
        .await
        .is_err());
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    assert!(journal
        .record_rejection("synthetic-private-operation")
        .await
        .is_err());
    sqlx::query("UPDATE mcp_write_meta SET mac_key = zeroblob(32)")
        .execute(&journal.pool)
        .await
        .unwrap();
    journal.close().await;
    assert!(WriteJournal::open(&directory.0).await.is_err());
    std::fs::write(
        directory.0.join(MARKER_FILE),
        b"synthetic-private-invalid-marker",
    )
    .unwrap();
    assert!(WriteJournal::open(&directory.0).await.is_err());
    std::fs::remove_file(directory.0.join(MARKER_FILE)).unwrap();
    assert!(WriteJournal::open(&directory.0).await.is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn private_modes_and_symlink_rejection_protect_journal_storage() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let directory = Directory::new();
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    assert_eq!(
        std::fs::metadata(&directory.0)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(directory.0.join(DATABASE_FILE))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    journal.pool.close().await;
    std::fs::set_permissions(&directory.0, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(WriteJournal::open(&directory.0).await.is_err());
    std::fs::set_permissions(&directory.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    let alias = Directory::new();
    symlink(&directory.0, &alias.0).unwrap();
    assert!(WriteJournal::open(&alias.0).await.is_err());
    std::fs::remove_file(&alias.0).unwrap();
    std::fs::rename(
        directory.0.join(DATABASE_FILE),
        directory.0.join("saved.sqlite"),
    )
    .unwrap();
    symlink(
        directory.0.join("saved.sqlite"),
        directory.0.join(DATABASE_FILE),
    )
    .unwrap();
    assert!(WriteJournal::open(&directory.0).await.is_err());
}
