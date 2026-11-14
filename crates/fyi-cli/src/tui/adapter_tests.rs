use super::*;
use crate::tui::{AppState, PrivacyMode};
use fyi_core::db::local::requests::NewLocalRequest;
use fyi_core::db::local::LocalAuthority;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fyi-tui-data-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> PathBuf {
        self.0.join("PRIVATE-PATH.db")
    }

    async fn initialize(&self) -> DbPool {
        fs::File::create(self.path()).unwrap();
        let db = DbPool::open_local_for_initialization(&self.path()).await.unwrap();
        db.initialize_rust_schema().await.unwrap();
        db.import_local_authorities(&[LocalAuthority {
            slug: "maple".to_owned(),
            name: "Maple\u{202e}Council".to_owned(),
            url: Some("https://catalog.example.invalid/PRIVATE-AUTHORITY-URL".to_owned()),
        }])
        .await
        .unwrap();
        db
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn file_adapter_is_redacted_bounded_and_preserves_the_database_bytes() {
    let fixture = Fixture::new();
    let db = fixture.initialize().await;
    db.register_local_request(&NewLocalRequest {
        authority_slug: "maple".to_owned(),
        title: "Records\u{1b}[31m".to_owned(),
        body: "PRIVATE-BODY".to_owned(),
        tags: "PRIVATE-TAGS".to_owned(),
        status: "unmapped PRIVATE-STATUS".to_owned(),
        fyi_request_id: Some(9001),
    })
    .await
    .unwrap();
    sqlx::query(
        "UPDATE tracked_requests SET updated_at = '2026-10-01T00:00:00Z',
         fyi_url = 'https://portal.example.invalid/PRIVATE-REQUEST-URL',
         last_event_title = 'PRIVATE-CORRESPONDENCE'",
    )
    .execute(db.pool())
    .await
    .unwrap();
    db.pool().close().await;
    let before = fs::read(fixture.path()).unwrap();
    let entries = fs::read_dir(&fixture.0).unwrap().count();
    for _ in 0..2 {
        let snapshot = load_snapshot(&fixture.path()).await.unwrap();
        assert_eq!(snapshot.scope, SnapshotScope::Complete);
        assert_eq!(snapshot.requests.len(), 1);
        let request = &snapshot.requests[0];
        assert_eq!(request.id, 1);
        assert_eq!(request.authority.as_deref(), Some("Maple�Council"));
        assert_eq!(request.title, "Records�[31m");
        assert_eq!(request.status, RequestStatus::Other);
        assert_eq!(request.needs_attention, None);
        assert_eq!(request.updated_at.as_deref(), Some("2026-10-01T00:00:00Z"));
        assert_eq!(request.timeline, TimelineState::Unavailable);
        assert!(!format!("{snapshot:?}").contains("PRIVATE"));
        assert_eq!(AppState::from_snapshot(snapshot).privacy, PrivacyMode::Redacted);
        assert_eq!(fs::read(fixture.path()).unwrap(), before);
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), entries);
    }
}

#[tokio::test]
async fn adapter_reports_exact_partial_total_and_never_invents_legacy_history() {
    let fixture = Fixture::new();
    let db = fixture.initialize().await;
    sqlx::query(
        "WITH RECURSIVE ids(id) AS (VALUES(1) UNION ALL SELECT id + 1 FROM ids WHERE id < 501)
         INSERT INTO tracked_requests (id, authority_slug, title, body, status, created_at, updated_at)
         SELECT id, 'maple', 'Synthetic request', 'PRIVATE-BODY', NULL,
         '2026-01-01T00:00:00Z', '2026-01-02T00:00:00Z' FROM ids",
    )
    .execute(db.pool())
    .await
    .unwrap();
    db.pool().close().await;
    let snapshot = load_snapshot(&fixture.path()).await.unwrap();
    assert_eq!(snapshot.scope, SnapshotScope::Partial { total: Some(501) });
    assert_eq!(snapshot.requests.len(), MAX_REQUESTS);
    assert_eq!(snapshot.requests[0].id, 1);
    assert_eq!(snapshot.requests[MAX_REQUESTS - 1].id, 500);
    assert!(snapshot.requests.iter().all(|row| {
        row.timeline == TimelineState::Unavailable
            && row.status == RequestStatus::Unknown
            && row.needs_attention.is_none()
    }));
}

#[tokio::test]
async fn empty_data_is_a_complete_ready_snapshot() {
    let fixture = Fixture::new();
    let db = fixture.initialize().await;
    db.pool().close().await;
    let snapshot = load_snapshot(&fixture.path()).await.unwrap();
    assert_eq!(snapshot, PresentationSnapshot::default());
}

#[tokio::test]
async fn missing_corrupt_and_unsupported_files_fail_without_creation_or_migration() {
    let fixture = Fixture::new();
    assert_eq!(load_snapshot(&fixture.path()).await, Err(PresentationError::Unavailable));
    assert!(!fixture.path().exists());
    assert_eq!(load_snapshot(&fixture.0).await, Err(PresentationError::Unavailable));
    for bytes in [b"PRIVATE-INVALID-DATABASE".as_slice(), b"".as_slice()] {
        fs::write(fixture.path(), bytes).unwrap();
        assert_eq!(load_snapshot(&fixture.path()).await, Err(PresentationError::Unavailable));
        assert_eq!(fs::read(fixture.path()).unwrap(), bytes);
    }
    fs::remove_file(fixture.path()).unwrap();
    let db = fixture.initialize().await;
    sqlx::query("INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (20990101000000, 'future', 1, x'00', 1)")
        .execute(db.pool()).await.unwrap();
    db.pool().close().await;
    let before = fs::read(fixture.path()).unwrap();
    assert_eq!(load_snapshot(&fixture.path()).await, Err(PresentationError::Unavailable));
    assert_eq!(fs::read(fixture.path()).unwrap(), before);
}

#[cfg(unix)]
#[tokio::test]
async fn symlinks_are_rejected_by_the_accepted_open_policy() {
    let fixture = Fixture::new();
    let db = fixture.initialize().await;
    db.pool().close().await;
    let link = fixture.0.join("link.db");
    std::os::unix::fs::symlink(fixture.path(), &link).unwrap();
    assert_eq!(load_snapshot(&link).await, Err(PresentationError::Unavailable));
}
