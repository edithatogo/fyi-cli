//! Retained local request lifecycle contracts, independent of remote sync.

use fyi_core::db::DbPool;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("fyi-local-requests-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }

    fn success(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(output.status.success(), "{args:?}: {output:?}");
        String::from_utf8(output.stdout).unwrap()
    }

    fn json(&self, args: &[&str]) -> Value {
        let mut command = vec!["--output-format", "json"];
        command.extend_from_slice(args);
        serde_json::from_str(&self.success(&command)).unwrap()
    }

    fn initialize(&self) {
        self.success(&["init-db", "--db", "local.db"]);
        fs::write(
            self.0.join("authorities.csv"),
            "slug,name,url\nalpha,Alpha Authority,\n",
        )
        .unwrap();
        self.success(&["import-authorities", "authorities.csv", "--db", "local.db"]);
    }

    fn register(&self) {
        self.success(&[
            "register-request",
            "alpha",
            "Local request",
            "PRIVATE-BODY-SENTINEL",
            "--db",
            "local.db",
        ]);
    }

    async fn database(&self) -> DbPool {
        DbPool::new(&format!("sqlite://{}", self.0.join("local.db").display()))
            .await
            .unwrap()
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

async fn event_count(db: &DbPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM local_request_events")
        .fetch_one(db.pool())
        .await
        .unwrap()
}

async fn request_state(db: &DbPool) -> (String, String) {
    sqlx::query_as("SELECT status, updated_at FROM tracked_requests WHERE id = 1")
        .fetch_one(db.pool())
        .await
        .unwrap()
}

async fn assert_remote_tables_empty(db: &DbPool) {
    let counts: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM requests),
                (SELECT COUNT(*) FROM correspondence),
                (SELECT COUNT(*) FROM sync_metadata),
                (SELECT COUNT(*) FROM sync_field_changes),
                (SELECT COUNT(*) FROM sync_outgoing_queue)",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(counts, (0, 0, 0, 0, 0));
}

#[tokio::test]
async fn registration_lists_and_details_preserve_local_fields_without_submission() {
    let work = Workspace::new();
    work.initialize();
    let registered = work.success(&[
        "register-request",
        "alpha",
        "Māori records",
        "PRIVATE-BODY-SENTINEL",
        "--tags",
        "topic:records,local",
        "--status",
        "awaiting clarification",
        "--fyi-request-id",
        "4242",
        "--db",
        "local.db",
    ]);
    assert_eq!(registered, "Registered request\n");
    assert_eq!(
        work.success(&["list-requests", "--db", "local.db"]),
        "1\talpha\tMāori records\tawaiting clarification\t4242\n"
    );

    let summaries = work.json(&["list-requests", "--db", "local.db"]);
    let rows = summaries.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], 1);
    assert_eq!(rows[0]["authority_slug"], "alpha");
    assert_eq!(rows[0]["title"], "Māori records");
    assert_eq!(rows[0]["status"], "awaiting clarification");
    assert_eq!(rows[0]["fyi_request_id"], 4242);
    assert_eq!(rows[0].as_object().unwrap().len(), 5);
    assert!(rows[0].get("body").is_none());
    assert!(!summaries.to_string().contains("PRIVATE-BODY-SENTINEL"));

    let detail = work.json(&["request-detail", "1", "--db", "local.db"]);
    assert_eq!(detail["id"], 1);
    assert_eq!(detail["authority_slug"], "alpha");
    assert_eq!(detail["body"], "PRIVATE-BODY-SENTINEL");
    assert_eq!(detail["tags"], "topic:records,local");
    assert_eq!(detail["fyi_request_id"], 4242);
    assert!(detail["fyi_url"].is_null());
    assert!(detail["last_seen_at"].is_null());
    assert!(detail["last_event_title"].is_null());
    assert!(!detail["created_at"].as_str().unwrap().is_empty());
    assert_eq!(detail["created_at"], detail["updated_at"]);
    for unsupported in [
        "latest_snapshot",
        "follow_up",
        "follow_up_variants",
        "follow_up_pack",
        "response_analysis",
        "next_best_action",
    ] {
        assert!(detail.get(unsupported).is_none());
    }

    let timeline = work.json(&["request-timeline", "1", "--db", "local.db"]);
    assert_eq!(detail["timeline"], timeline);
    let events = timeline.as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].as_object().unwrap().len(), 4);
    assert!(events[0]["kind"].is_string());
    assert_eq!(events[0]["title"], "Tracked request created");
    assert_eq!(events[0]["detail"], "Māori records");
    assert_eq!(events[0]["ts"], detail["created_at"]);

    let db = work.database().await;
    assert_eq!(event_count(&db).await, 1);
    let invalid_references = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(db.pool())
        .await
        .unwrap();
    assert!(invalid_references.is_empty());
    assert_remote_tables_empty(&db).await;
    db.pool().close().await;
}

#[tokio::test]
async fn json_acknowledgements_report_identity_and_change_without_request_content() {
    let work = Workspace::new();
    work.initialize();
    let registration = work.json(&[
        "register-request",
        "alpha",
        "PRIVATE-TITLE-SENTINEL",
        "PRIVATE-BODY-SENTINEL",
        "--tags",
        "PRIVATE-TAGS-SENTINEL",
        "--db",
        "local.db",
    ]);
    assert_eq!(registration, json!({"id": 1}));
    assert_eq!(
        work.json(&["set-status", "1", "submitted", "--db", "local.db"]),
        json!({"id": 1, "status": "submitted", "changed": true})
    );
    assert_eq!(
        work.json(&["set-status", "1", "submitted", "--db", "local.db"]),
        json!({"id": 1, "status": "submitted", "changed": false})
    );
    let db = work.database().await;
    assert_eq!(event_count(&db).await, 2);
    assert_remote_tables_empty(&db).await;
    db.pool().close().await;
}

#[tokio::test]
async fn blank_registration_fields_and_invalid_identifiers_fail_before_any_mutation() {
    let work = Workspace::new();
    work.initialize();
    work.register();
    let db = work.database().await;
    let original = work.json(&["request-detail", "1", "--db", "local.db"]);
    let original_events = event_count(&db).await;
    for (title, body) in [
        ("", "PRIVATE-BODY-SENTINEL"),
        ("PRIVATE-TITLE-SENTINEL", ""),
        (" \t ", "PRIVATE-BODY-SENTINEL"),
        ("PRIVATE-TITLE-SENTINEL", " \r\n "),
    ] {
        let output = work.run(&["register-request", "alpha", title, body, "--db", "local.db"]);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!error.contains("PRIVATE-TITLE-SENTINEL"));
        assert!(!error.contains("PRIVATE-BODY-SENTINEL"));
    }
    for identifier in ["0", "-1"] {
        for command in ["request-detail", "request-timeline"] {
            let output = work.run(&[command, "--db", "local.db", "--", identifier]);
            assert_eq!(output.status.code(), Some(1), "{output:?}");
            assert!(output.stdout.is_empty());
        }
        let output = work.run(&[
            "set-status",
            "--db",
            "local.db",
            "--",
            identifier,
            "submitted",
        ]);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty());
        let remote_id = format!("--fyi-request-id={identifier}");
        let output = work.run(&[
            "register-request",
            "alpha",
            "Title",
            "Body",
            &remote_id,
            "--db",
            "local.db",
        ]);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty());
    }
    for arguments in [
        vec!["request-detail", "9223372036854775808"],
        vec!["request-timeline", "9223372036854775808"],
        vec!["set-status", "9223372036854775808", "submitted"],
        vec![
            "register-request",
            "alpha",
            "Title",
            "Body",
            "--fyi-request-id=9223372036854775808",
        ],
    ] {
        let output = work.run(&arguments);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty());
    }
    assert!(!work.0.join("fyi_system.db").exists());
    assert_eq!(
        work.json(&["request-detail", "1", "--db", "local.db"]),
        original
    );
    assert_eq!(event_count(&db).await, original_events);
    assert_eq!(
        work.json(&["list-requests", "--db", "local.db"])
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_remote_tables_empty(&db).await;
    db.pool().close().await;
}

#[tokio::test]
async fn local_reads_preserve_nullable_stored_fields_without_inventing_defaults() {
    let work = Workspace::new();
    work.initialize();
    work.register();
    let db = work.database().await;
    sqlx::query(
        "UPDATE tracked_requests
         SET tags = NULL, status = NULL, created_at = NULL, updated_at = NULL",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let detail = work.json(&["request-detail", "1", "--db", "local.db"]);
    for field in ["tags", "status", "created_at", "updated_at"] {
        assert_eq!(detail.get(field), Some(&Value::Null));
    }
    assert_eq!(detail["body"], "PRIVATE-BODY-SENTINEL");
    assert_eq!(
        work.json(&["list-requests", "--db", "local.db"]),
        json!([{
            "id": 1,
            "authority_slug": "alpha",
            "title": "Local request",
            "status": null,
            "fyi_request_id": null
        }])
    );
    let stored: (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = sqlx::query_as("SELECT tags, status, created_at, updated_at FROM tracked_requests")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(stored, (None, None, None, None));
    assert_eq!(event_count(&db).await, 1);
    db.pool().close().await;
}

#[tokio::test]
async fn status_change_is_audited_and_repeated_status_preserves_timestamp_and_history() {
    let work = Workspace::new();
    work.initialize();
    work.register();
    let db = work.database().await;
    sqlx::query("UPDATE tracked_requests SET updated_at = '2001-01-01T00:00:00Z'")
        .execute(db.pool())
        .await
        .unwrap();
    let initial = request_state(&db).await;
    let initial_events = event_count(&db).await;
    work.success(&["set-status", "1", "draft", "--db", "local.db"]);
    assert_eq!(request_state(&db).await, initial);
    assert_eq!(event_count(&db).await, initial_events);

    assert_eq!(
        work.success(&["set-status", "1", "submitted", "--db", "local.db"]),
        "Updated request 1 to submitted\n"
    );
    let changed = request_state(&db).await;
    assert_eq!(changed.0, "submitted");
    assert_ne!(changed.1, initial.1);
    assert_eq!(event_count(&db).await, initial_events + 1);
    let event: (Option<String>, String, String) = sqlx::query_as(
        "SELECT old_status, new_status, occurred_at
         FROM local_request_events ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(event, (Some("draft".to_string()), changed.0, changed.1));
    let state_before_repeat = request_state(&db).await;
    work.success(&["set-status", "1", "submitted", "--db", "local.db"]);
    assert_eq!(request_state(&db).await, state_before_repeat);
    assert_eq!(event_count(&db).await, initial_events + 1);
    let timeline = work.json(&["request-timeline", "1", "--db", "local.db"]);
    assert_eq!(timeline.as_array().unwrap().len(), 2);
    assert_remote_tables_empty(&db).await;
    db.pool().close().await;
}

#[tokio::test]
async fn invalid_registration_and_missing_requests_fail_without_mutation_or_sensitive_errors() {
    let work = Workspace::new();
    work.initialize();
    for arguments in [
        vec![
            "register-request",
            "missing-authority",
            "PRIVATE-TITLE-SENTINEL",
            "PRIVATE-BODY-SENTINEL",
            "--db",
            "local.db",
        ],
        vec!["set-status", "99", "submitted", "--db", "local.db"],
        vec!["request-timeline", "99", "--db", "local.db"],
        vec!["request-detail", "99", "--db", "local.db"],
    ] {
        let output = work.run(&arguments);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!error.contains("PRIVATE-TITLE-SENTINEL"));
        assert!(!error.contains("PRIVATE-BODY-SENTINEL"));
    }
    assert_eq!(work.run(&["register-request"]).status.code(), Some(2));
    assert_eq!(
        work.run(&["request-detail", "not-an-integer"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(work.json(&["list-requests", "--db", "local.db"]), json!([]));
    let db = work.database().await;
    assert_eq!(event_count(&db).await, 0);
    assert_remote_tables_empty(&db).await;
    db.pool().close().await;
}

#[tokio::test]
async fn status_validation_preserves_arbitrary_labels_and_rejects_empty_long_or_control_values() {
    let work = Workspace::new();
    work.initialize();
    work.register();
    let db = work.database().await;
    let initial = request_state(&db).await;
    let initial_events = event_count(&db).await;
    let overlong = "s".repeat(129);
    for status in ["", "bad\nstatus", "bad\tstatus", overlong.as_str()] {
        let output = work.run(&["set-status", "1", status, "--db", "local.db"]);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(request_state(&db).await, initial);
        assert_eq!(event_count(&db).await, initial_events);
    }
    let longest = "s".repeat(128);
    work.success(&["set-status", "1", &longest, "--db", "local.db"]);
    assert_eq!(request_state(&db).await.0, longest);
    work.success(&[
        "set-status",
        "1",
        "awaiting local review",
        "--db",
        "local.db",
    ]);
    assert_eq!(request_state(&db).await.0, "awaiting local review");
    db.pool().close().await;
}

#[tokio::test]
async fn event_failure_rolls_back_status_and_does_not_disclose_database_error_details() {
    let work = Workspace::new();
    work.initialize();
    work.register();
    let db = work.database().await;
    let initial = request_state(&db).await;
    let initial_events = event_count(&db).await;
    sqlx::query(
        "CREATE TRIGGER reject_local_event BEFORE INSERT ON local_request_events
         BEGIN SELECT RAISE(ABORT, 'PRIVATE-TRIGGER-SENTINEL'); END",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let output = work.run(&["set-status", "1", "refused", "--db", "local.db"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE-TRIGGER-SENTINEL"));
    assert_eq!(request_state(&db).await, initial);
    assert_eq!(event_count(&db).await, initial_events);
    assert_remote_tables_empty(&db).await;
    db.pool().close().await;
}

#[tokio::test]
async fn event_failure_rolls_back_registration_and_autoincrement_allocation() {
    let work = Workspace::new();
    work.initialize();
    let db = work.database().await;
    sqlx::query(
        "CREATE TRIGGER reject_registration_event BEFORE INSERT ON local_request_events
         BEGIN SELECT RAISE(ABORT, 'PRIVATE-TRIGGER-SENTINEL'); END",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let output = work.run(&[
        "register-request",
        "alpha",
        "Local request",
        "PRIVATE-BODY-SENTINEL",
        "--db",
        "local.db",
    ]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE-TRIGGER-SENTINEL"));
    assert_eq!(work.json(&["list-requests", "--db", "local.db"]), json!([]));
    assert_eq!(event_count(&db).await, 0);
    sqlx::query("DROP TRIGGER reject_registration_event")
        .execute(db.pool())
        .await
        .unwrap();
    work.register();
    assert_eq!(
        work.json(&["list-requests", "--db", "local.db"])[0]["id"],
        1
    );
    assert_remote_tables_empty(&db).await;
    db.pool().close().await;
}

#[tokio::test]
async fn repeated_registration_keeps_local_ids_distinct_from_remote_request_identity() {
    let work = Workspace::new();
    work.initialize();
    let db = work.database().await;
    sqlx::query(
        "INSERT INTO requests(id, title, body, status)
         VALUES (1, 'Remote record', 'Remote body', 'successful')",
    )
    .execute(db.pool())
    .await
    .unwrap();
    work.register();
    work.register();
    let local = work.json(&["list-requests", "--db", "local.db"]);
    assert_eq!(local.as_array().unwrap().len(), 2);
    assert_eq!(local[0]["id"], 1);
    assert_eq!(local[1]["id"], 2);
    work.success(&["set-status", "1", "withdrawn locally", "--db", "local.db"]);
    let remote: (String, String, String) =
        sqlx::query_as("SELECT title, body, status FROM requests WHERE id = 1")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(
        remote,
        (
            "Remote record".to_string(),
            "Remote body".to_string(),
            "successful".to_string()
        )
    );
    let counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM sync_metadata),
                (SELECT COUNT(*) FROM sync_field_changes),
                (SELECT COUNT(*) FROM sync_outgoing_queue)",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(counts, (0, 0, 0));
    db.pool().close().await;
}

#[tokio::test]
async fn local_and_remote_identifiers_beyond_i32_remain_usable_without_truncation() {
    let work = Workspace::new();
    work.initialize();
    work.register();
    let db = work.database().await;
    sqlx::query("UPDATE sqlite_sequence SET seq = 2147483647 WHERE name = 'tracked_requests'")
        .execute(db.pool())
        .await
        .unwrap();
    work.success(&[
        "register-request",
        "alpha",
        "Large identifiers",
        "Local body",
        "--fyi-request-id",
        "4294967296",
        "--db",
        "local.db",
    ]);
    let detail = work.json(&["request-detail", "2147483648", "--db", "local.db"]);
    assert_eq!(detail["id"], json!(2147483648_i64));
    assert_eq!(detail["fyi_request_id"], json!(4294967296_i64));
    work.success(&["set-status", "2147483648", "reviewed", "--db", "local.db"]);
    let timeline = work.json(&["request-timeline", "2147483648", "--db", "local.db"]);
    assert_eq!(timeline.as_array().unwrap().len(), 2);
    assert_remote_tables_empty(&db).await;
    db.pool().close().await;
}

#[tokio::test]
async fn read_commands_neither_create_databases_nor_migrate_an_existing_unrecognized_schema() {
    let work = Workspace::new();
    for arguments in [
        vec!["list-requests", "--db", "missing.db"],
        vec!["request-detail", "1", "--db", "missing.db"],
        vec!["request-timeline", "1", "--db", "missing.db"],
    ] {
        assert_eq!(work.run(&arguments).status.code(), Some(1));
        assert!(!work.0.join("missing.db").exists());
    }

    fs::write(work.0.join("local.db"), []).unwrap();
    let db = work.database().await;
    sqlx::query("CREATE TABLE unrelated_marker(value TEXT)")
        .execute(db.pool())
        .await
        .unwrap();
    db.pool().close().await;
    let original_bytes = fs::read(work.0.join("local.db")).unwrap();
    for arguments in [
        vec!["list-requests", "--db", "local.db"],
        vec!["request-detail", "1", "--db", "local.db"],
        vec!["request-timeline", "1", "--db", "local.db"],
    ] {
        assert_eq!(work.run(&arguments).status.code(), Some(1));
        assert_eq!(fs::read(work.0.join("local.db")).unwrap(), original_bytes);
    }
}
