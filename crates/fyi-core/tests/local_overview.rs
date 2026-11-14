//! Bounded metadata reads use the existing local schema, with no history/body projection.
use fyi_core::db::local::overview::MAX_LOCAL_OVERVIEW_REQUESTS;
use fyi_core::db::local::requests::NewLocalRequest;
use fyi_core::db::local::LocalAuthority;
use fyi_core::db::DbPool;

async fn database() -> DbPool {
    let db = DbPool::new_in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    db.import_local_authorities(&[LocalAuthority {
        slug: "maple".to_owned(),
        name: "Maple Council".to_owned(),
        url: Some("https://catalog.example.invalid/PRIVATE-URL".to_owned()),
    }])
    .await
    .unwrap();
    db
}

#[tokio::test]
async fn metadata_retains_local_identity_actual_authority_and_nullable_values() {
    let db = database().await;
    let id = db
        .register_local_request(&NewLocalRequest {
            authority_slug: "maple".to_owned(),
            title: "Maple records".to_owned(),
            body: "PRIVATE-BODY".to_owned(),
            tags: "PRIVATE-TAGS".to_owned(),
            status: "unrecognized-state".to_owned(),
            fyi_request_id: Some(9001),
        })
        .await
        .unwrap();
    sqlx::query("UPDATE tracked_requests SET updated_at = NULL WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await
        .unwrap();
    let overview = db.local_request_overview(500).await.unwrap();
    assert_eq!(overview.total, 1);
    assert_eq!(overview.requests.len(), 1);
    let row = &overview.requests[0];
    assert_eq!(row.request.id, id);
    assert_eq!(row.request.title, "Maple records");
    assert_eq!(row.request.status.as_deref(), Some("unrecognized-state"));
    assert_eq!(row.authority_name.as_deref(), Some("Maple Council"));
    assert_eq!(row.updated_at, None);
    assert_eq!(
        db.get_local_request(id).await.unwrap().unwrap().body,
        "PRIVATE-BODY"
    );
}

#[tokio::test]
async fn empty_overview_and_invalid_limits_are_explicit() {
    let db = database().await;
    let overview = db.local_request_overview(500).await.unwrap();
    assert_eq!(overview.total, 0);
    assert!(overview.requests.is_empty());
    for limit in [0, MAX_LOCAL_OVERVIEW_REQUESTS + 1, usize::MAX] {
        assert!(db.local_request_overview(limit).await.is_err());
    }
}

#[tokio::test]
async fn overview_bounds_rows_before_allocation_and_orders_by_local_id() {
    let db = database().await;
    sqlx::query(
        "WITH RECURSIVE ids(id) AS (VALUES(1) UNION ALL SELECT id + 1 FROM ids WHERE id < 501)
         INSERT INTO tracked_requests (id, authority_slug, title, body, status)
         SELECT id, 'maple', 'Synthetic request', 'PRIVATE-BODY', 'draft' FROM ids",
    )
    .execute(db.pool())
    .await
    .unwrap();
    for limit in [1, 3, 500] {
        let overview = db.local_request_overview(limit).await.unwrap();
        assert_eq!(overview.total, 501);
        assert_eq!(overview.requests.len(), limit);
        assert_eq!(overview.requests.first().unwrap().request.id, 1);
        assert_eq!(overview.requests.last().unwrap().request.id, limit as i64);
    }
    let remote_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_outgoing_queue")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(remote_count, 0);
}

#[tokio::test]
async fn oversized_text_is_bounded_in_sql_without_splitting_unicode() {
    let db = database().await;
    sqlx::query(
        "INSERT INTO tracked_requests (authority_slug, title, body, status, updated_at)
         VALUES ('maple', ?, 'PRIVATE-BODY', ?, ?)",
    )
    .bind("🧪".repeat(10_000))
    .bind(format!("draft{}unmapped", " ".repeat(10_000)))
    .bind("y".repeat(10_000))
    .execute(db.pool())
    .await
    .unwrap();
    sqlx::query("UPDATE authorities SET name = ?")
        .bind("界".repeat(10_000))
        .execute(db.pool())
        .await
        .unwrap();
    let overview = db.local_request_overview(500).await.unwrap();
    let row = &overview.requests[0];
    assert_eq!(row.request.title, "🧪".repeat(513));
    assert_eq!(
        row.authority_name.as_deref(),
        Some("界".repeat(513).as_str())
    );
    assert_eq!(row.updated_at.as_ref().unwrap().chars().count(), 513);
    // A truncated known-looking prefix must not turn an unknown state into Draft.
    assert_eq!(row.request.status.as_deref(), Some("unrecognized"));
}

#[tokio::test]
async fn missing_authority_is_unavailable_instead_of_a_requester_or_slug_guess() {
    let db = database().await;
    // Simulate an unvalidated caller: LEFT JOIN still does not invent an authority.
    let mut connection = db.pool().acquire().await.unwrap();
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *connection)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO tracked_requests (authority_slug, title, body)
         VALUES ('missing', 'Synthetic request', 'PRIVATE-BODY')",
    )
    .execute(&mut *connection)
    .await
    .unwrap();
    drop(connection);
    let overview = db.local_request_overview(500).await.unwrap();
    assert_eq!(overview.requests[0].authority_name, None);
}
