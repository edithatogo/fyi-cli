use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn init_db_refuses_a_legacy_database_without_changing_it() {
    use fyi_core::db::DbPool;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_nanos();
    let work_dir = std::env::temp_dir().join(format!("fyi-legacy-{}-{unique}", std::process::id()));
    fs::create_dir(&work_dir).expect("create isolated directory");
    let db_path = work_dir.join("legacy.db");
    let pool = DbPool::new(&format!("sqlite://{}?mode=rwc", db_path.display()))
        .await
        .expect("create legacy fixture");
    sqlx::query("CREATE TABLE tracked_requests (id INTEGER PRIMARY KEY, body TEXT NOT NULL)")
        .execute(pool.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO tracked_requests (id, body) VALUES (1, 'preserve this draft')")
        .execute(pool.pool())
        .await
        .unwrap();
    pool.pool().close().await;
    drop(pool);
    let before = fs::read(&db_path).unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
        .arg("init-db")
        .arg("--db")
        .arg(&db_path)
        .output()
        .expect("run init-db");
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("legacy Python database"));
    assert_eq!(fs::read(&db_path).unwrap(), before);
    fs::remove_dir_all(work_dir).expect("remove isolated directory");
}

#[tokio::test]
async fn init_db_creates_a_usable_schema_and_preserves_existing_records() {
    use fyi_core::api::AlaveteliRequest;
    use fyi_core::db::DbPool;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_nanos();
    let work_dir = std::env::temp_dir().join(format!("fyi-init-{}-{unique}", std::process::id()));
    fs::create_dir(&work_dir).expect("create isolated directory");
    let db_path = work_dir.join("requests.db");

    let init = || {
        Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .arg("init-db")
            .arg("--db")
            .arg(&db_path)
            .output()
            .expect("run init-db")
    };
    let first = init();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );

    let pool = DbPool::new(&format!("sqlite://{}", db_path.display()))
        .await
        .expect("open initialized database");
    assert!(pool
        .get_request(7)
        .await
        .expect("schema must exist")
        .is_none());
    let request = AlaveteliRequest {
        id: 7,
        title: "Existing request".to_owned(),
        body: "Local draft".to_owned(),
        user_name: None,
        status: Some("draft".to_owned()),
        created_at: None,
        updated_at: None,
        url: None,
        tags: None,
    };
    pool.insert_request(&request)
        .await
        .expect("insert local request");

    let second = init();
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(pool.get_request(7).await.unwrap(), Some(request));
    drop(pool);
    fs::remove_dir_all(work_dir).expect("remove isolated directory");
}

#[test]
fn unimplemented_commands_fail_without_side_effects() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_nanos();
    let work_dir =
        std::env::temp_dir().join(format!("fyi-fail-closed-{}-{unique}", std::process::id()));
    fs::create_dir(&work_dir).expect("create isolated directory");
    fs::write(work_dir.join("protected.txt"), b"preserve existing data")
        .expect("create sentinel file");

    let cases: &[(&str, &[&str])] = &[
        ("register-request", &["authority", "title", "body"]),
        ("list-requests", &[]),
        ("set-status", &["1", "draft"]),
        ("request-timeline", &["1"]),
        ("export-requests", &["--output", "protected.txt"]),
        ("import-requests", &["protected.txt"]),
        ("ingest-feed", &["not-a-url"]),
        ("reconcile-events", &[]),
        ("attention-report", &["--output", "protected.txt"]),
        ("handover", &["--output", "protected.txt"]),
        ("dashboard", &["--output", "protected.txt"]),
        ("run-cycle", &["not-a-url"]),
        ("scheduler", &["not-a-url", "--once"]),
        ("serve", &[]),
        ("request-detail", &["1"]),
        ("export-request", &["1", "--output", "protected.txt"]),
        ("follow-up-draft", &["1", "--output", "protected.txt"]),
        ("attachment-manifest", &["1", "--output", "protected.txt"]),
        (
            "attachment-manifest-csv",
            &["1", "--output", "protected.txt"],
        ),
        ("follow-up-variants", &["1", "--output", "protected.txt"]),
        ("follow-up-pack", &["1", "--output", "protected.txt"]),
        ("triage-report", &["--output", "protected.txt"]),
        ("response-analysis", &["1", "--output", "protected.txt"]),
        ("next-best-action", &["1", "--output", "protected.txt"]),
        ("correspondence-pack", &["1", "--output", "protected.txt"]),
        ("export-bundle", &["1", "--output-dir", "output"]),
        ("show-settings", &["--output", "protected.txt"]),
        ("privacy-audit", &["--output", "protected.txt"]),
        ("mcp-server", &[]),
        ("tui", &[]),
    ];

    for (name, args) in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .current_dir(&work_dir)
            .arg(name)
            .args(*args)
            .output()
            .unwrap_or_else(|error| panic!("could not run {name}: {error}"));
        assert!(!output.status.success(), "{name} falsely returned success");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("not yet implemented"), "{name}: {stderr}");
        assert!(output.stdout.is_empty(), "{name} printed a success message");
        assert_eq!(
            fs::read(work_dir.join("protected.txt")).unwrap(),
            b"preserve existing data"
        );
        assert_eq!(
            fs::read_dir(&work_dir).unwrap().count(),
            1,
            "{name} created files"
        );
    }

    fs::remove_dir_all(work_dir).expect("remove isolated directory");
}
