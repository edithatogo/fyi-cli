use fyi_core::db::DbPool;
use fyi_core::shared_pacing::{read_status, reserve_slot, ReservationInput};

fn input(owner: &str) -> ReservationInput<'_> {
    ReservationInput {
        name: "archive-discovery",
        owner_id: owner,
        now: 1_000.0,
        interval_seconds: 2.0,
        jitter_seconds: 0.25,
        jitter_fraction: 0.5,
        status_code: None,
    }
}

#[tokio::test]
async fn reservations_and_status_match_the_frozen_python_fixture() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/pacing/shared-rate-limit-v1-python.json"
    ))
    .unwrap();
    let db = DbPool::new_in_memory().await.unwrap();
    for (index, entry) in fixture["inputs"].as_array().unwrap().iter().enumerate() {
        let reservation = reserve_slot(
            db.pool(),
            &ReservationInput {
                name: "archive-discovery",
                owner_id: entry["owner_id"].as_str().unwrap(),
                now: fixture["now"].as_f64().unwrap(),
                interval_seconds: entry["interval_seconds"].as_f64().unwrap(),
                jitter_seconds: fixture["jitter_seconds"].as_f64().unwrap(),
                jitter_fraction: fixture["jitter_fraction"].as_f64().unwrap(),
                status_code: entry["status_code"].as_u64().map(|value| value as u16),
            },
        )
        .await
        .unwrap();
        assert_eq!(
            serde_json::to_value(reservation).unwrap(),
            fixture["reservations"][index]
        );
    }
    let status = read_status(db.pool(), "archive-discovery")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(serde_json::to_value(status).unwrap(), fixture["status"]);
}

#[tokio::test]
async fn reservations_preserve_the_python_state_and_status_contract() {
    let db = DbPool::new_in_memory().await.unwrap();
    let first = reserve_slot(db.pool(), &input("worker-a")).await.unwrap();
    let second = reserve_slot(db.pool(), &input("worker-b")).await.unwrap();

    assert_eq!(first.sleep_seconds, 0.0);
    assert_eq!(first.next_allowed_at, 1_002.125);
    assert_eq!(second.sleep_seconds, 2.125);
    assert_eq!(second.next_allowed_at, 1_004.25);
    let status = read_status(db.pool(), "archive-discovery")
        .await
        .unwrap()
        .unwrap();
    let value = serde_json::to_value(status).unwrap();
    assert_eq!(value["name"], "archive-discovery");
    assert_eq!(value["last_owner_id"], "worker-b");
    assert_eq!(value["last_sleep_seconds"], 2.125);
    assert_eq!(value["interval_seconds"], 2.0);
    assert_eq!(value["jitter_seconds"], 0.25);
    assert_eq!(value["recent_events"].as_array().unwrap().len(), 2);
    assert_eq!(value["recent_events"][0]["kind"], "acquired");
    assert_eq!(value["recent_events"][0]["owner_id"], "worker-b");
}

#[tokio::test]
async fn readback_of_an_empty_database_does_not_create_tables() {
    let db = DbPool::new_in_memory().await.unwrap();
    assert!(read_status(db.pool(), "archive-discovery")
        .await
        .unwrap()
        .is_none());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn invalid_input_fails_before_creating_state() {
    let db = DbPool::new_in_memory().await.unwrap();
    let mut request = input("worker-a");
    request.interval_seconds = f64::NAN;
    assert!(reserve_slot(db.pool(), &request).await.is_err());
    assert!(read_status(db.pool(), "archive-discovery")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn independent_pools_cannot_reserve_the_same_slot() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("fyi-pacing-{}-{unique}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let url = format!(
        "sqlite://{}?mode=rwc",
        directory.join("pacing.db").display()
    );
    let first = DbPool::new(&url).await.unwrap();
    let second = DbPool::new(&url).await.unwrap();
    let first_input = input("worker-a");
    let second_input = input("worker-b");
    let (a, b) = tokio::join!(
        reserve_slot(first.pool(), &first_input),
        reserve_slot(second.pool(), &second_input),
    );
    let mut delays = [a.unwrap().sleep_seconds, b.unwrap().sleep_seconds];
    delays.sort_by(f64::total_cmp);
    assert_eq!(delays, [0.0, 2.125]);
    first.pool().close().await;
    second.pool().close().await;
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn backoff_extends_the_shared_reservation() {
    let db = DbPool::new_in_memory().await.unwrap();
    reserve_slot(db.pool(), &input("worker-a")).await.unwrap();
    let mut backoff = input("worker-a");
    backoff.interval_seconds = 10.0;
    backoff.status_code = Some(429);
    let delayed = reserve_slot(db.pool(), &backoff).await.unwrap();
    assert_eq!(delayed.kind, "backoff");
    let next = reserve_slot(db.pool(), &input("worker-b")).await.unwrap();
    assert_eq!(next.sleep_seconds, 12.25);
    let status = read_status(db.pool(), "archive-discovery")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.recent_events[1].status_code, Some(429));
    assert_eq!(status.recent_events[1].delay_seconds, 10.0);
}

#[tokio::test]
async fn an_event_write_failure_rolls_back_the_state_update() {
    let db = DbPool::new_in_memory().await.unwrap();
    let first = reserve_slot(db.pool(), &input("worker-a")).await.unwrap();
    sqlx::raw_sql(
        "CREATE TRIGGER reject_pacing_event BEFORE INSERT ON shared_rate_limit_events
         BEGIN SELECT RAISE(ABORT, 'synthetic event failure'); END;",
    )
    .execute(db.pool())
    .await
    .unwrap();
    assert!(reserve_slot(db.pool(), &input("worker-b")).await.is_err());
    let status = read_status(db.pool(), "archive-discovery")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.next_allowed_at, first.next_allowed_at);
    assert_eq!(status.last_owner_id, "worker-a");
    assert_eq!(status.recent_events.len(), 1);
}

#[tokio::test]
async fn corrupt_stored_values_fail_without_advancing_the_limiter() {
    let db = DbPool::new_in_memory().await.unwrap();
    reserve_slot(db.pool(), &input("worker-a")).await.unwrap();
    sqlx::query("UPDATE shared_rate_limit_state SET next_allowed_at = -1")
        .execute(db.pool())
        .await
        .unwrap();
    assert!(read_status(db.pool(), "archive-discovery").await.is_err());
    assert!(reserve_slot(db.pool(), &input("worker-b")).await.is_err());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM shared_rate_limit_events")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn status_returns_only_the_ten_latest_events_for_the_requested_name() {
    let db = DbPool::new_in_memory().await.unwrap();
    for index in 0..12 {
        reserve_slot(db.pool(), &input(&format!("worker-{index}")))
            .await
            .unwrap();
    }
    let status = read_status(db.pool(), "archive-discovery")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.recent_events.len(), 10);
    assert_eq!(status.recent_events[0].owner_id, "worker-11");
    assert_eq!(status.recent_events[9].owner_id, "worker-2");
    assert!(read_status(db.pool(), "other-source")
        .await
        .unwrap()
        .is_none());
}
