//! Local request repository contracts; these operations must never submit requests.

use fyi_core::db::local::requests::{NewLocalRequest, TimelineItem, TrackedRequest};
use fyi_core::db::local::LocalAuthority;
use fyi_core::db::DbPool;
use serde_json::json;

fn request() -> NewLocalRequest {
    NewLocalRequest {
        authority_slug: "alpha".to_owned(),
        title: "Local records request".to_owned(),
        body: "PRIVATE-BODY-SENTINEL\r\nMāori cafe\u{301}\t🧪".to_owned(),
        tags: "local, records; preserve spaces".to_owned(),
        status: "draft".to_owned(),
        fyi_request_id: None,
    }
}

async fn database() -> DbPool {
    let db = DbPool::new_in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    db.import_local_authorities(&[LocalAuthority {
        slug: "alpha".to_owned(),
        name: "Alpha Authority".to_owned(),
        url: None,
    }])
    .await
    .unwrap();
    db
}

async fn counts(db: &DbPool) -> (i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM tracked_requests),
                (SELECT COUNT(*) FROM local_request_events)",
    )
    .fetch_one(db.pool())
    .await
    .unwrap()
}

async fn remote_counts(db: &DbPool) -> (i64, i64, i64, i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM requests),
                (SELECT COUNT(*) FROM correspondence),
                (SELECT COUNT(*) FROM sync_metadata),
                (SELECT COUNT(*) FROM sync_field_changes),
                (SELECT COUNT(*) FROM sync_outgoing_queue)",
    )
    .fetch_one(db.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn registration_preserves_content_and_links_one_atomic_creation_event() {
    let db = database().await;
    let mut input = request();
    input.fyi_request_id = Some(4294967296);
    let id = db.register_local_request(&input).await.unwrap();
    assert_eq!(id, 1);
    let stored = db.get_local_request(id).await.unwrap().unwrap();
    assert_eq!(stored.id, id);
    assert_eq!(stored.authority_slug, input.authority_slug);
    assert_eq!(stored.title, input.title);
    assert_eq!(stored.body, input.body);
    assert_eq!(stored.tags.as_deref(), Some(input.tags.as_str()));
    assert_eq!(stored.status.as_deref(), Some("draft"));
    assert_eq!(stored.fyi_request_id, Some(4294967296));
    assert_eq!(stored.fyi_url, None);
    assert_eq!(stored.last_seen_at, None);
    assert_eq!(stored.last_event_title, None);
    assert!(stored
        .created_at
        .as_deref()
        .is_some_and(|time| { chrono::DateTime::parse_from_rfc3339(time).is_ok() }));
    assert_eq!(stored.created_at, stored.updated_at);
    assert_eq!(
        serde_json::to_value(&stored)
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        12
    );
    let event: (i64, String, Option<String>, String, String) = sqlx::query_as(
        "SELECT tracked_request_id, kind, old_status, new_status, occurred_at
         FROM local_request_events",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(event.0, id);
    assert_eq!(event.1, "created");
    assert_eq!(event.2, None);
    assert_eq!(event.3, "draft");
    assert_eq!(Some(event.4), stored.created_at);
    assert_eq!(counts(&db).await, (1, 1));
    assert_eq!(remote_counts(&db).await, (0, 0, 0, 0, 0));
}

#[tokio::test]
async fn summaries_are_ordered_minimal_and_keep_i64_identifiers() {
    let db = database().await;
    db.register_local_request(&request()).await.unwrap();
    sqlx::query("UPDATE sqlite_sequence SET seq = 2147483647 WHERE name = 'tracked_requests'")
        .execute(db.pool())
        .await
        .unwrap();
    let mut second = request();
    second.title = "Second request".to_owned();
    second.fyi_request_id = Some(4294967296);
    assert_eq!(
        db.register_local_request(&second).await.unwrap(),
        2147483648
    );
    let summaries = db.list_local_requests().await.unwrap();
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].id, 1);
    assert_eq!(summaries[1].id, 2147483648);
    assert_eq!(summaries[1].fyi_request_id, Some(4294967296));
    let projection = serde_json::to_value(summaries).unwrap();
    for row in projection.as_array().unwrap() {
        assert_eq!(row.as_object().unwrap().len(), 5);
        assert!(row.get("body").is_none());
        assert!(row.get("tags").is_none());
        assert!(row.get("fyi_url").is_none());
    }
    assert!(!projection.to_string().contains("PRIVATE-BODY-SENTINEL"));
    assert!(db.get_local_request(2147483648).await.unwrap().is_some());
}

#[tokio::test]
async fn repeated_registration_is_distinct_and_never_changes_a_remote_identity() {
    let db = database().await;
    sqlx::query(
        "INSERT INTO requests (id, title, body, status)
         VALUES (1, 'Remote request', 'Remote body', 'successful')",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let before = remote_counts(&db).await;
    let mut input = request();
    input.fyi_request_id = Some(1);
    assert_eq!(db.register_local_request(&input).await.unwrap(), 1);
    assert_eq!(db.register_local_request(&input).await.unwrap(), 2);
    db.set_local_request_status(1, "withdrawn locally")
        .await
        .unwrap();
    assert_eq!(counts(&db).await, (2, 3));
    assert_eq!(remote_counts(&db).await, before);
    let remote: (String, String, String) =
        sqlx::query_as("SELECT title, body, status FROM requests WHERE id = 1")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(
        remote,
        (
            "Remote request".to_owned(),
            "Remote body".to_owned(),
            "successful".to_owned()
        )
    );
}

#[tokio::test]
async fn registration_validates_direct_callers_before_persisting_any_rows() {
    let db = database().await;
    for invalid in ["", " \t\r\n "] {
        let mut input = request();
        input.title = invalid.to_owned();
        assert!(db.register_local_request(&input).await.is_err());
        let mut input = request();
        input.body = invalid.to_owned();
        assert!(db.register_local_request(&input).await.is_err());
    }
    for slug in ["", "missing", "alpha\0"] {
        let mut input = request();
        input.authority_slug = slug.to_owned();
        let error = db.register_local_request(&input).await.unwrap_err();
        assert!(!error.to_string().contains("PRIVATE-BODY-SENTINEL"));
    }
    for id in [0, -1] {
        let mut input = request();
        input.fyi_request_id = Some(id);
        assert!(db.register_local_request(&input).await.is_err());
    }
    for status in ["", "   ", "bad\nstatus", "bad\tstatus"] {
        let mut input = request();
        input.status = status.to_owned();
        assert!(db.register_local_request(&input).await.is_err());
    }
    assert_eq!(counts(&db).await, (0, 0));
    assert_eq!(remote_counts(&db).await, (0, 0, 0, 0, 0));
}

#[tokio::test]
async fn invalid_and_missing_request_ids_are_distinguished_without_side_effects() {
    let db = database().await;
    for id in [0, -1, i64::MIN] {
        assert!(db.get_local_request(id).await.is_err());
        assert!(db.local_request_timeline(id).await.is_err());
        assert!(db.local_request_detail(id).await.is_err());
        assert!(db.set_local_request_status(id, "reviewed").await.is_err());
    }
    assert!(db.get_local_request(i64::MAX).await.unwrap().is_none());
    assert!(db.local_request_timeline(i64::MAX).await.is_err());
    assert!(db.local_request_detail(i64::MAX).await.is_err());
    assert!(db
        .set_local_request_status(i64::MAX, "reviewed")
        .await
        .is_err());
    assert_eq!(counts(&db).await, (0, 0));
}

#[tokio::test]
async fn status_changes_preserve_noop_time_and_record_previous_status() {
    let db = database().await;
    let id = db.register_local_request(&request()).await.unwrap();
    sqlx::query("UPDATE tracked_requests SET updated_at = '2001-01-01T00:00:00Z'")
        .execute(db.pool())
        .await
        .unwrap();
    let before = serde_json::to_value(db.get_local_request(id).await.unwrap()).unwrap();
    assert!(!db.set_local_request_status(id, "draft").await.unwrap());
    assert_eq!(
        serde_json::to_value(db.get_local_request(id).await.unwrap()).unwrap(),
        before
    );
    assert_eq!(counts(&db).await, (1, 1));
    assert!(db
        .set_local_request_status(id, "awaiting local review")
        .await
        .unwrap());
    let updated = db.get_local_request(id).await.unwrap().unwrap();
    assert_eq!(updated.status.as_deref(), Some("awaiting local review"));
    assert_ne!(updated.updated_at.as_deref(), Some("2001-01-01T00:00:00Z"));
    let event: (Option<String>, String, String) = sqlx::query_as(
        "SELECT old_status, new_status, occurred_at
         FROM local_request_events WHERE kind = 'status_changed'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(event.0.as_deref(), Some("draft"));
    assert_eq!(Some(event.1), updated.status);
    assert_eq!(Some(event.2), updated.updated_at);
    assert!(!db
        .set_local_request_status(id, "awaiting local review")
        .await
        .unwrap());
    assert_eq!(counts(&db).await, (1, 2));
}

#[tokio::test]
async fn status_limits_are_utf8_byte_bounds_and_never_mutate_on_failure() {
    let db = database().await;
    let id = db.register_local_request(&request()).await.unwrap();
    let before = serde_json::to_value(db.get_local_request(id).await.unwrap()).unwrap();
    for status in [
        String::new(),
        "   ".to_owned(),
        "a".repeat(129),
        "é".repeat(65),
        "bad\u{1b}status".to_owned(),
        "bad\nstatus".to_owned(),
    ] {
        assert!(db.set_local_request_status(id, &status).await.is_err());
        assert_eq!(
            serde_json::to_value(db.get_local_request(id).await.unwrap()).unwrap(),
            before
        );
        assert_eq!(counts(&db).await, (1, 1));
    }
    assert!(db
        .set_local_request_status(id, &"é".repeat(64))
        .await
        .unwrap());
    assert_eq!(
        db.get_local_request(id).await.unwrap().unwrap().status,
        Some("é".repeat(64))
    );
}

#[tokio::test]
async fn registration_event_failure_rolls_back_rows_and_sequence_allocation() {
    let db = database().await;
    sqlx::query(
        "CREATE TRIGGER reject_registration_event BEFORE INSERT ON local_request_events
         BEGIN SELECT RAISE(ABORT, 'PRIVATE-TRIGGER-SENTINEL'); END",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let error = db.register_local_request(&request()).await.unwrap_err();
    assert!(!error.to_string().contains("PRIVATE-TRIGGER-SENTINEL"));
    assert!(!error.to_string().contains("PRIVATE-BODY-SENTINEL"));
    assert_eq!(counts(&db).await, (0, 0));
    sqlx::query("DROP TRIGGER reject_registration_event")
        .execute(db.pool())
        .await
        .unwrap();
    assert_eq!(db.register_local_request(&request()).await.unwrap(), 1);
    assert_eq!(counts(&db).await, (1, 1));
}

#[tokio::test]
async fn status_event_failure_rolls_back_status_timestamp_and_history() {
    let db = database().await;
    let id = db.register_local_request(&request()).await.unwrap();
    let before = serde_json::to_value(db.local_request_detail(id).await.unwrap()).unwrap();
    sqlx::query(
        "CREATE TRIGGER reject_status_event BEFORE INSERT ON local_request_events
         WHEN NEW.kind = 'status_changed'
         BEGIN SELECT RAISE(ABORT, 'PRIVATE-TRIGGER-SENTINEL'); END",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let error = db
        .set_local_request_status(id, "reviewed")
        .await
        .unwrap_err();
    assert!(!error.to_string().contains("PRIVATE-TRIGGER-SENTINEL"));
    assert_eq!(
        serde_json::to_value(db.local_request_detail(id).await.unwrap()).unwrap(),
        before
    );
    assert_eq!(counts(&db).await, (1, 1));
    assert_eq!(remote_counts(&db).await, (0, 0, 0, 0, 0));
}

#[tokio::test]
async fn nullable_legacy_fields_remain_null_and_null_status_is_audited_correctly() {
    let db = database().await;
    sqlx::query(
        "INSERT INTO tracked_requests (
            id, authority_slug, title, body, tags, status, created_at, updated_at
         ) VALUES (42, 'alpha', 'Legacy request', 'Legacy body', NULL, NULL, NULL, NULL)",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let stored = db.get_local_request(42).await.unwrap().unwrap();
    assert_eq!(stored.tags, None);
    assert_eq!(stored.status, None);
    assert_eq!(stored.created_at, None);
    assert_eq!(stored.updated_at, None);
    assert_eq!(stored.fyi_request_id, None);
    assert_eq!(stored.fyi_url, None);
    assert_eq!(stored.last_seen_at, None);
    assert_eq!(stored.last_event_title, None);
    assert!(db.local_request_timeline(42).await.unwrap().is_empty());
    assert_eq!(counts(&db).await, (1, 0));
    assert!(db.set_local_request_status(42, "reviewed").await.unwrap());
    let event: (Option<String>, String) =
        sqlx::query_as("SELECT old_status, new_status FROM local_request_events")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(event, (None, "reviewed".to_owned()));
    assert_eq!(db.get_local_request(42).await.unwrap().unwrap().tags, None);
}

#[tokio::test]
async fn timeline_falls_back_to_legacy_row_without_inventing_stored_events() {
    let db = database().await;
    sqlx::query(
        "INSERT INTO tracked_requests (
            id, authority_slug, title, body, status, created_at, updated_at
         ) VALUES (
            7, 'alpha', 'Historical request', 'Historical body', 'waiting_response',
            '2026-01-02T10:00:00+10:00', '2026-01-02 01:00:00'
         )",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let timeline = db.local_request_timeline(7).await.unwrap();
    assert_eq!(
        serde_json::to_value(&timeline).unwrap(),
        json!([
            {
                "ts": "2026-01-02 01:00:00",
                "kind": "tracked_request",
                "title": "Tracked request updated",
                "detail": "Status: waiting_response"
            },
            {
                "ts": "2026-01-02T10:00:00+10:00",
                "kind": "tracked_request",
                "title": "Tracked request created",
                "detail": "Historical request"
            }
        ])
    );
    assert_eq!(counts(&db).await, (1, 0));
    assert_eq!(
        db.get_local_request(7).await.unwrap().unwrap().created_at,
        Some("2026-01-02T10:00:00+10:00".to_owned())
    );
}

#[tokio::test]
async fn native_timeline_uses_stable_event_order_without_duplicate_row_fallbacks() {
    let db = database().await;
    let id = db.register_local_request(&request()).await.unwrap();
    db.set_local_request_status(id, "first review")
        .await
        .unwrap();
    db.set_local_request_status(id, "second review")
        .await
        .unwrap();
    sqlx::query("UPDATE local_request_events SET occurred_at = '2026-01-02T03:04:05Z'")
        .execute(db.pool())
        .await
        .unwrap();
    let timeline = db.local_request_timeline(id).await.unwrap();
    assert_eq!(timeline.len(), 3);
    assert_eq!(timeline[0].detail, "Status: second review");
    assert_eq!(timeline[1].detail, "Status: first review");
    assert_eq!(timeline[2].title, "Tracked request created");
    for item in &timeline {
        assert_eq!(item.ts.as_deref(), Some("2026-01-02T03:04:05Z"));
        assert_eq!(item.kind, "tracked_request");
    }
    assert_eq!(counts(&db).await, (1, 3));
}

#[tokio::test]
async fn detail_exposes_exact_local_projection_and_its_timeline_without_derived_claims() {
    let db = database().await;
    let id = db.register_local_request(&request()).await.unwrap();
    db.set_local_request_status(id, "reviewed").await.unwrap();
    let detail = db.local_request_detail(id).await.unwrap();
    assert_eq!(detail.request.id, id);
    assert_eq!(detail.request.body, request().body);
    let value = serde_json::to_value(detail).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 13);
    assert_eq!(value["status"], "reviewed");
    assert_eq!(
        value["timeline"],
        serde_json::to_value(db.local_request_timeline(id).await.unwrap()).unwrap()
    );
    for unsupported in [
        "follow_up",
        "response_analysis",
        "latest_snapshot",
        "next_best_action",
    ] {
        assert!(value.get(unsupported).is_none());
    }
    assert!(value["body"]
        .as_str()
        .unwrap()
        .contains("PRIVATE-BODY-SENTINEL"));
}

#[tokio::test]
async fn foreign_keys_reject_orphans_and_deletion_cascades_only_local_history() {
    let db = database().await;
    assert!(sqlx::query(
        "INSERT INTO tracked_requests (authority_slug, title, body)
         VALUES ('missing', 'Orphan', 'Body')"
    )
    .execute(db.pool())
    .await
    .is_err());
    assert!(sqlx::query(
        "INSERT INTO local_request_events (
            tracked_request_id, kind, old_status, new_status, occurred_at
         ) VALUES (99, 'created', NULL, 'draft', '2026-01-02T03:04:05Z')"
    )
    .execute(db.pool())
    .await
    .is_err());
    let id = db.register_local_request(&request()).await.unwrap();
    assert!(sqlx::query("DELETE FROM authorities WHERE slug = 'alpha'")
        .execute(db.pool())
        .await
        .is_err());
    sqlx::query("DELETE FROM tracked_requests WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await
        .unwrap();
    assert_eq!(counts(&db).await, (0, 0));
    assert_eq!(db.list_local_authorities().await.unwrap().len(), 1);
    assert_eq!(remote_counts(&db).await, (0, 0, 0, 0, 0));
}

#[tokio::test]
async fn concurrent_status_updates_have_a_serial_previous_status_chain() {
    let db = database().await;
    let id = db.register_local_request(&request()).await.unwrap();
    let (first, second) = tokio::join!(
        db.set_local_request_status(id, "first local decision"),
        db.set_local_request_status(id, "second local decision")
    );
    assert!(first.unwrap());
    assert!(second.unwrap());
    let transitions: Vec<(Option<String>, String)> = sqlx::query_as(
        "SELECT old_status, new_status FROM local_request_events
         WHERE kind = 'status_changed' ORDER BY id",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();
    assert_eq!(transitions.len(), 2);
    assert_eq!(transitions[0].0.as_deref(), Some("draft"));
    assert_eq!(transitions[1].0.as_ref(), Some(&transitions[0].1));
    assert_eq!(
        db.get_local_request(id)
            .await
            .unwrap()
            .unwrap()
            .status
            .as_ref(),
        Some(&transitions[1].1)
    );
    assert_eq!(counts(&db).await, (1, 3));
}

#[test]
fn serialized_local_records_require_every_nullable_key_and_reject_unknown_fields() {
    let complete = json!({
        "id": 7,
        "authority_slug": "alpha",
        "title": "Legacy request",
        "body": "Private local text",
        "tags": null,
        "fyi_url": null,
        "fyi_request_id": null,
        "status": null,
        "created_at": null,
        "updated_at": null,
        "last_seen_at": null,
        "last_event_title": null
    });
    let decoded: TrackedRequest = serde_json::from_value(complete.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), complete);
    for field in [
        "id",
        "authority_slug",
        "title",
        "body",
        "tags",
        "fyi_url",
        "fyi_request_id",
        "status",
        "created_at",
        "updated_at",
        "last_seen_at",
        "last_event_title",
    ] {
        let mut missing = complete.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<TrackedRequest>(missing).is_err(),
            "missing {field} was silently defaulted"
        );
    }
    let mut unknown = complete.clone();
    unknown["submit_remotely"] = json!(true);
    assert!(serde_json::from_value::<TrackedRequest>(unknown).is_err());
    let mut invalid_id = complete.clone();
    invalid_id["fyi_request_id"] = json!("7");
    assert!(serde_json::from_value::<TrackedRequest>(invalid_id).is_err());
    let mut invalid_tags = complete;
    invalid_tags["tags"] = json!(["not", "the", "legacy", "text", "field"]);
    assert!(serde_json::from_value::<TrackedRequest>(invalid_tags).is_err());
}

#[test]
fn timeline_json_preserves_explicit_null_time_and_rejects_missing_time() {
    let complete = json!({
        "ts": null,
        "kind": "feed_event",
        "title": "Undated event",
        "detail": "Known text with unknown time"
    });
    let decoded: TimelineItem = serde_json::from_value(complete.clone()).unwrap();
    assert_eq!(decoded.ts, None);
    assert_eq!(serde_json::to_value(decoded).unwrap(), complete);
    let mut missing = complete;
    missing.as_object_mut().unwrap().remove("ts");
    assert!(serde_json::from_value::<TimelineItem>(missing).is_err());
}

#[tokio::test]
async fn oversized_registration_fields_are_rejected_without_allocating_an_id() {
    let db = database().await;
    let mut long_title = request();
    long_title.title = "x".repeat(16 * 1024 + 1);
    assert!(db.register_local_request(&long_title).await.is_err());
    let mut long_tags = request();
    long_tags.tags = "x".repeat(16 * 1024 + 1);
    assert!(db.register_local_request(&long_tags).await.is_err());
    let mut long_body = request();
    long_body.body = "x".repeat(16 * 1024 * 1024 + 1);
    assert!(db.register_local_request(&long_body).await.is_err());
    let mut long_status = request();
    long_status.status = "x".repeat(129);
    assert!(db.register_local_request(&long_status).await.is_err());
    assert_eq!(counts(&db).await, (0, 0));
    let mut boundary = request();
    boundary.title = "x".repeat(16 * 1024);
    boundary.tags = "x".repeat(16 * 1024);
    boundary.status = "x".repeat(128);
    assert_eq!(db.register_local_request(&boundary).await.unwrap(), 1);
}

#[tokio::test]
async fn legacy_creation_remains_visible_after_the_first_native_status_change() {
    let db = database().await;
    sqlx::query(
        "INSERT INTO tracked_requests (
            id, authority_slug, title, body, status, created_at, updated_at
         ) VALUES (
            7, 'alpha', 'Historical request', 'Historical body', 'waiting_response',
            '2026-01-02 03:04:05', '2026-01-03 04:05:06'
         )",
    )
    .execute(db.pool())
    .await
    .unwrap();
    db.set_local_request_status(7, "reviewed").await.unwrap();
    let timeline = db.local_request_timeline(7).await.unwrap();
    assert_eq!(timeline.len(), 2);
    assert!(timeline.iter().any(|item| {
        item.title == "Tracked request created" && item.ts.as_deref() == Some("2026-01-02 03:04:05")
    }));
    assert!(timeline
        .iter()
        .any(|item| item.detail == "Status: reviewed"));
    assert_eq!(counts(&db).await, (1, 1));
}

#[tokio::test]
async fn legacy_unknown_times_remain_unchanged_and_sort_after_known_times() {
    let db = database().await;
    sqlx::query(
        "INSERT INTO tracked_requests (
            id, authority_slug, title, body, status, created_at, updated_at
         ) VALUES (
            7, 'alpha', 'Historical request', 'Historical body', NULL,
            'unknown historic time', '2026-01-02T03:04:05'
         )",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let timeline = db.local_request_timeline(7).await.unwrap();
    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline[0].ts.as_deref(), Some("2026-01-02T03:04:05"));
    assert_eq!(timeline[0].detail, "Status: None");
    assert_eq!(timeline[1].ts.as_deref(), Some("unknown historic time"));
    sqlx::query("UPDATE tracked_requests SET updated_at = 'unknown later time' WHERE id = 7")
        .execute(db.pool())
        .await
        .unwrap();
    let timeline = db.local_request_timeline(7).await.unwrap();
    assert_eq!(timeline[0].ts.as_deref(), Some("unknown later time"));
    assert_eq!(timeline[1].ts.as_deref(), Some("unknown historic time"));
    assert_eq!(counts(&db).await, (1, 0));
}

#[tokio::test]
async fn unknown_stored_event_kind_fails_without_returning_a_partial_timeline() {
    let db = database().await;
    let id = db.register_local_request(&request()).await.unwrap();
    sqlx::query("PRAGMA ignore_check_constraints = ON")
        .execute(db.pool())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE local_request_events SET kind = 'unrecognized' WHERE tracked_request_id = ?",
    )
    .bind(id)
    .execute(db.pool())
    .await
    .unwrap();
    assert!(db.local_request_timeline(id).await.is_err());
    assert!(db.local_request_detail(id).await.is_err());
    assert_eq!(counts(&db).await, (1, 1));
}
