//! Read-only projections of locally retained feed and snapshot provenance.

use fyi_core::db::local::provenance::{FeedEvent, RequestSnapshot};
use fyi_core::db::local::requests::TrackedRequest;
use fyi_core::db::DbPool;
use serde_json::{json, Value};

const LEGACY_SQL: &str = include_str!("../../../conformance/local-management/legacy-v1.sql");
const LEGACY_EXPECTED: &str =
    include_str!("../../../conformance/local-management/legacy-v1-expected.json");

async fn legacy_database() -> DbPool {
    let db = DbPool::new_in_memory().await.unwrap();
    sqlx::raw_sql(LEGACY_SQL).execute(db.pool()).await.unwrap();
    db.run_migrations().await.unwrap();
    db
}

async fn source_rows(db: &DbPool) -> (Vec<FeedEvent>, Vec<RequestSnapshot>) {
    let feeds = sqlx::query_as(
        "SELECT id, feed_url, event_id, title, link, published, summary,
                request_id_guess, tracked_request_id, raw_json, seen_at
         FROM feed_events ORDER BY id",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();
    let snapshots = sqlx::query_as(
        "SELECT id, fyi_request_id, source_url, raw_json, fetched_at
         FROM request_snapshots ORDER BY id",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();
    (feeds, snapshots)
}

async fn retained_state(db: &DbPool) -> Value {
    let (feeds, snapshots) = source_rows(db).await;
    let requests: Vec<TrackedRequest> = sqlx::query_as(
        "SELECT id, authority_slug, title, body, tags, fyi_url, fyi_request_id,
                status, created_at, updated_at, last_seen_at, last_event_title
         FROM tracked_requests ORDER BY id",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();
    let cache: Vec<(
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT url, etag, last_modified, response_body, cached_at
         FROM http_cache ORDER BY url",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();
    let remote: Vec<(i64, String, String, Option<String>)> =
        sqlx::query_as("SELECT id, title, body, status FROM requests ORDER BY id")
            .fetch_all(db.pool())
            .await
            .unwrap();
    let counts: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM sync_metadata),
                (SELECT COUNT(*) FROM sync_field_changes),
                (SELECT COUNT(*) FROM sync_outgoing_queue),
                (SELECT COUNT(*) FROM local_request_events),
                (SELECT COUNT(*) FROM correspondence)",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    json!({
        "feeds": feeds,
        "snapshots": snapshots,
        "requests": requests,
        "cache": cache,
        "remote": remote,
        "counts": counts
    })
}

#[tokio::test]
async fn fixture_timeline_preserves_python_projections_with_correct_chronological_order() {
    let db = legacy_database().await;
    let reference: Value = serde_json::from_str(LEGACY_EXPECTED).unwrap();
    let python = &reference["python_outputs"]["request_timeline"]["7"];
    // Python sorts timestamp strings. Rust compares their actual instants while
    // preserving the original text and all four projected fields.
    let expected: Vec<_> = [0, 1, 3, 2, 5, 4]
        .into_iter()
        .map(|index| python[index].clone())
        .collect();
    let timeline = serde_json::to_value(db.local_request_timeline(7).await.unwrap()).unwrap();
    assert_eq!(timeline, json!(expected));
    assert_eq!(timeline.as_array().unwrap().len(), 6);
    for item in timeline.as_array().unwrap() {
        assert_eq!(item.as_object().unwrap().len(), 4);
    }
    let rendered = timeline.to_string();
    for excluded in [
        "Synthetic remote ID 7 is not local ID 7",
        "Synthetic unrelated feed row",
        "SYNTHETIC-PRIVATE-BODY-7",
        "SYNTHETIC-PRIVATE-SNAPSHOT-5",
        "SYNTHETIC-PRIVATE-FEED-3",
        "SYNTHETIC-PRIVATE-CACHE-7",
    ] {
        assert!(!rendered.contains(excluded), "leaked or mislinked {excluded}");
    }
}

#[tokio::test]
async fn local_and_remote_request_ids_never_substitute_for_provenance_links() {
    let db = legacy_database().await;
    assert!(db.local_request_timeline(2147483650).await.unwrap().is_empty());
    sqlx::query(
        "INSERT INTO feed_events (
            id, feed_url, title, request_id_guess, tracked_request_id, seen_at
         ) VALUES (
            101, 'https://example.invalid/feed', 'Explicit local link', 9007,
            2147483650, '2026-02-03T04:05:06Z'
         )",
    )
    .execute(db.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO request_snapshots (id, fyi_request_id, source_url, raw_json, fetched_at)
         VALUES (
            101, 2147483650, 'https://example.invalid/snapshot',
            '{\"title\":\"Same numeric ID is not a remote link\"}', '2026-02-04T04:05:06Z'
         )",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let timeline = db.local_request_timeline(2147483650).await.unwrap();
    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].kind, "feed_event");
    assert_eq!(timeline[0].title, "Explicit local link");
    let other = serde_json::to_value(db.local_request_timeline(7).await.unwrap()).unwrap();
    assert!(!other.to_string().contains("Explicit local link"));
    assert!(!other.to_string().contains("Same numeric ID is not a remote link"));
}

#[tokio::test]
async fn feed_fallbacks_preserve_null_time_and_never_project_raw_payloads() {
    let db = legacy_database().await;
    sqlx::query(
        "INSERT INTO feed_events (
            id, feed_url, title, link, published, summary, tracked_request_id, raw_json, seen_at
         ) VALUES (
            101, 'https://example.invalid/feed', NULL, NULL, NULL, NULL, 7,
            '{\"body\":\"PRIVATE-RAW-FEED\"}', NULL
         )",
    )
    .execute(db.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO feed_events (
            id, feed_url, title, link, published, summary, tracked_request_id, seen_at
         ) VALUES (
            102, 'https://example.invalid/feed', '', 'https://example.invalid/link', '', '', 7,
            '2026-02-03 04:05:06'
         )",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let timeline = db.local_request_timeline(7).await.unwrap();
    let undated = timeline.iter().find(|item| item.ts.is_none()).unwrap();
    assert_eq!(
        serde_json::to_value(undated).unwrap(),
        json!({"ts": null, "kind": "feed_event", "title": "Feed event", "detail": ""})
    );
    assert_eq!(
        serde_json::to_value(&timeline[0]).unwrap(),
        json!({
            "ts": "2026-02-03 04:05:06",
            "kind": "feed_event",
            "title": "Feed event",
            "detail": "https://example.invalid/link"
        })
    );
    assert!(!serde_json::to_string(&timeline).unwrap().contains("PRIVATE-RAW-FEED"));
}

#[tokio::test]
async fn feed_timeline_does_not_decode_or_modify_unused_raw_payload_bytes() {
    let db = legacy_database().await;
    sqlx::query(
        "INSERT INTO feed_events (
            id, feed_url, title, tracked_request_id, raw_json, seen_at
         ) VALUES (
            101, 'https://example.invalid/feed', 'Known projection', 7,
            CAST(X'80FF' AS TEXT), '2026-02-03T04:05:06Z'
         )",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let timeline = db.local_request_timeline(7).await.unwrap();
    assert_eq!(timeline[0].title, "Known projection");
    assert_eq!(timeline[0].kind, "feed_event");
    let preserved: String =
        sqlx::query_scalar("SELECT hex(raw_json) FROM feed_events WHERE id = 101")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(preserved, "80FF");
}

#[tokio::test]
async fn snapshot_projection_uses_top_level_then_nested_strings_without_normalizing_text() {
    let db = legacy_database().await;
    let cases = [
        (
            "{\"title\":\"  Top title — Māori  \",\"described_state\":\"top state\",\
             \"info_request\":{\"title\":\"Nested title\",\"described_state\":\"nested state\"}}",
            "  Top title — Māori  ",
            "top state",
        ),
        (
            "{\"title\":\"\",\"described_state\":null,\
             \"info_request\":{\"title\":\"Nested title\",\"described_state\":\"nested state\"}}",
            "Nested title",
            "nested state",
        ),
        ("{}", "Request snapshot", ""),
    ];
    for (index, (raw, title, state)) in cases.into_iter().enumerate() {
        let id = 101 + i64::try_from(index).unwrap();
        sqlx::query(
            "INSERT INTO request_snapshots (id, fyi_request_id, source_url, raw_json, fetched_at)
             VALUES (?, 9007, 'https://example.invalid/snapshot', ?, '2026-02-03T04:05:06Z')",
        )
        .bind(id)
        .bind(raw)
        .execute(db.pool())
        .await
        .unwrap();
        let timeline = db.local_request_timeline(7).await.unwrap();
        assert_eq!(timeline[0].title, title);
        assert_eq!(timeline[0].detail, state);
        assert_eq!(timeline[0].kind, "request_snapshot");
    }
}

#[tokio::test]
async fn malformed_or_nonobject_snapshot_json_has_safe_defaults_and_preserves_storage() {
    let db = legacy_database().await;
    for raw in [
        "not JSON: PRIVATE-SNAPSHOT-RAW",
        "{\"body\":\"PRIVATE-SNAPSHOT-RAW\",",
        "[\"PRIVATE-SNAPSHOT-RAW\"]",
        "null",
        "{\"info_request\":null,\"body\":\"PRIVATE-SNAPSHOT-RAW\"}",
        "{\"info_request\":[\"PRIVATE-SNAPSHOT-RAW\"]}",
        "{\"title\":{\"body\":\"PRIVATE-SNAPSHOT-RAW\"},\"described_state\":[123]}",
    ] {
        sqlx::query(
            "INSERT INTO request_snapshots (id, fyi_request_id, source_url, raw_json, fetched_at)
             VALUES (101, 9007, 'https://example.invalid/snapshot', ?, NULL)
             ON CONFLICT(id) DO UPDATE SET raw_json = excluded.raw_json",
        )
        .bind(raw)
        .execute(db.pool())
        .await
        .unwrap();
        let timeline = db.local_request_timeline(7).await.unwrap();
        let undated = timeline.iter().find(|item| item.ts.is_none()).unwrap();
        assert_eq!(
            serde_json::to_value(undated).unwrap(),
            json!({
                "ts": null,
                "kind": "request_snapshot",
                "title": "Request snapshot",
                "detail": ""
            })
        );
        assert!(!serde_json::to_string(&timeline).unwrap().contains("PRIVATE-SNAPSHOT-RAW"));
        let stored: String =
            sqlx::query_scalar("SELECT raw_json FROM request_snapshots WHERE id = 101")
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(stored, raw);
    }
}

#[tokio::test]
async fn equal_instants_keep_descending_ids_within_deterministic_source_groups() {
    let db = legacy_database().await;
    for (id, timestamp) in [(101_i64, "2026-02-03T14:05:06+10:00"), (102, "2026-02-03T04:05:06Z")] {
        sqlx::query(
            "INSERT INTO feed_events (id, feed_url, title, tracked_request_id, published, seen_at)
             VALUES (?, 'https://example.invalid/feed', ?, 7, ?, NULL)",
        )
        .bind(id)
        .bind(format!("feed-{id}"))
        .bind(timestamp)
        .execute(db.pool())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO request_snapshots (id, fyi_request_id, source_url, raw_json, fetched_at)
             VALUES (?, 9007, 'https://example.invalid/snapshot', ?, ?)",
        )
        .bind(id)
        .bind(json!({"title": format!("snapshot-{id}")}).to_string())
        .bind(timestamp)
        .execute(db.pool())
        .await
        .unwrap();
    }
    let timeline = db.local_request_timeline(7).await.unwrap();
    let first: Vec<_> = timeline.iter().take(4).map(|item| item.title.as_str()).collect();
    assert_eq!(first, ["feed-102", "feed-101", "snapshot-102", "snapshot-101"]);
    assert_eq!(timeline[0].ts.as_deref(), Some("2026-02-03T04:05:06Z"));
    assert_eq!(timeline[1].ts.as_deref(), Some("2026-02-03T14:05:06+10:00"));
}

#[tokio::test]
async fn timeline_and_detail_reads_leave_all_retained_content_and_sync_state_unchanged() {
    let db = legacy_database().await;
    sqlx::query(
        "INSERT INTO requests (id, title, body, status)
         VALUES (7, 'Remote record', 'PRIVATE-REMOTE-BODY', 'successful')",
    )
    .execute(db.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO sync_metadata (request_id, sync_status, local_updated_at)
         VALUES (7, 'clean', '2026-01-02T03:04:05Z')",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let before = retained_state(&db).await;
    let timeline = db.local_request_timeline(7).await.unwrap();
    let detail = db.local_request_detail(7).await.unwrap();
    assert_eq!(
        serde_json::to_value(detail.timeline).unwrap(),
        serde_json::to_value(&timeline).unwrap()
    );
    assert_eq!(timeline.len(), 6);
    assert!(detail.request.body.contains("SYNTHETIC-PRIVATE-BODY-7"));
    assert_eq!(retained_state(&db).await, before);
    let state: String =
        sqlx::query_scalar("SELECT sync_status FROM sync_metadata WHERE request_id = 7")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(state, "clean");
}

#[tokio::test]
async fn source_models_round_trip_every_fixture_field_and_raw_json_byte() {
    let db = legacy_database().await;
    let reference: Value = serde_json::from_str(LEGACY_EXPECTED).unwrap();
    let (feeds, snapshots) = source_rows(&db).await;
    let feed_json = serde_json::to_value(feeds).unwrap();
    let snapshot_json = serde_json::to_value(snapshots).unwrap();
    assert_eq!(feed_json, reference["tables"]["feed_events"]);
    assert_eq!(snapshot_json, reference["tables"]["request_snapshots"]);
    for row in feed_json.as_array().unwrap() {
        assert_eq!(row.as_object().unwrap().len(), 11);
        let decoded: FeedEvent = serde_json::from_value(row.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), *row);
    }
    for row in snapshot_json.as_array().unwrap() {
        assert_eq!(row.as_object().unwrap().len(), 5);
        let decoded: RequestSnapshot = serde_json::from_value(row.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), *row);
    }
}

#[test]
fn feed_json_requires_explicit_nullable_keys_and_rejects_extensions() {
    let complete = json!({
        "id": 7,
        "feed_url": "https://example.invalid/feed",
        "event_id": null,
        "title": null,
        "link": null,
        "published": null,
        "summary": null,
        "request_id_guess": null,
        "tracked_request_id": null,
        "raw_json": null,
        "seen_at": null
    });
    let decoded: FeedEvent = serde_json::from_value(complete.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), complete);
    for field in complete.as_object().unwrap().keys() {
        let mut missing = complete.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<FeedEvent>(missing).is_err(), "{field}");
    }
    let mut unknown = complete.clone();
    unknown["private_payload"] = json!("not an allowed extension");
    assert!(serde_json::from_value::<FeedEvent>(unknown).is_err());
    let mut incorrect_id = complete;
    incorrect_id["tracked_request_id"] = json!("7");
    assert!(serde_json::from_value::<FeedEvent>(incorrect_id).is_err());
}

#[test]
fn snapshot_json_requires_timestamp_key_and_preserves_explicit_null() {
    let complete = json!({
        "id": 7,
        "fyi_request_id": 9007,
        "source_url": "https://example.invalid/snapshot",
        "raw_json": "{ \"body\": \"PRIVATE-SNAPSHOT-RAW\" }\r\n",
        "fetched_at": null
    });
    let decoded: RequestSnapshot = serde_json::from_value(complete.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), complete);
    for field in complete.as_object().unwrap().keys() {
        let mut missing = complete.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<RequestSnapshot>(missing).is_err(), "{field}");
    }
    let mut unknown = complete;
    unknown["body"] = json!("not a top-level snapshot field");
    assert!(serde_json::from_value::<RequestSnapshot>(unknown).is_err());
}
