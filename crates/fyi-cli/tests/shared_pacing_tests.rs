use fyi_core::db::DbPool;
use fyi_core::shared_pacing::{read_status, reserve_slot, ReservationInput};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn test_directory() -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "fyi-pacing-cli-{}-{unique}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    directory
}

async fn write_fixture(path: &Path) -> serde_json::Value {
    let db = DbPool::new(&format!("sqlite://{}?mode=rwc", path.display()))
        .await
        .unwrap();
    reserve_slot(
        db.pool(),
        &ReservationInput {
            name: "archive-discovery",
            owner_id: "fixture-worker",
            now: 1_000.0,
            interval_seconds: 2.0,
            jitter_seconds: 0.25,
            jitter_fraction: 0.5,
            status_code: None,
        },
    )
    .await
    .unwrap();
    let status = read_status(db.pool(), "archive-discovery").await.unwrap();
    db.pool().close().await;
    serde_json::to_value(status).unwrap()
}

#[test]
fn missing_status_database_fails_without_creating_it() {
    let directory = test_directory();
    let path = directory.join("missing.db");
    let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
        .arg("rate-limit-status")
        .arg("--db")
        .arg(&path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!path.exists());
    assert!(fs::read_dir(&directory).unwrap().next().is_none());
    fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn status_matches_existing_state_without_changing_the_database() {
    let directory = test_directory();
    let path = directory.join("pacing.db");
    let expected = write_fixture(&path).await;
    let before = fs::read(&path).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
        .arg("rate-limit-status")
        .arg("--db")
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        expected
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn unknown_limiter_returns_null_without_creating_state() {
    let directory = test_directory();
    let path = directory.join("pacing.db");
    write_fixture(&path).await;
    let before = fs::read(&path).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
        .arg("rate-limit-status")
        .arg("--db")
        .arg(&path)
        .args(["--name", "other-source"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "null");
    assert_eq!(fs::read(&path).unwrap(), before);
    fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn status_output_is_idempotent_and_preserves_conflicting_files() {
    let directory = test_directory();
    let db = directory.join("pacing.db");
    let expected = write_fixture(&db).await;
    let before = fs::read(&db).unwrap();
    let report = directory.join("status.json");
    let run = |output: &Path| {
        Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .arg("rate-limit-status")
            .arg("--db")
            .arg(&db)
            .arg("--output")
            .arg(output)
            .output()
            .unwrap()
    };
    assert!(run(&report).status.success());
    assert!(run(&report).status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&report).unwrap()).unwrap(),
        expected
    );
    fs::write(&report, b"preserve this report").unwrap();
    assert!(!run(&report).status.success());
    assert_eq!(fs::read(&report).unwrap(), b"preserve this report");
    assert!(!run(&db).status.success());
    assert_eq!(fs::read(&db).unwrap(), before);
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn status_output_rejects_a_symlink_to_the_database() {
    let directory = test_directory();
    let db = directory.join("pacing.db");
    write_fixture(&db).await;
    let before = fs::read(&db).unwrap();
    let output_path = directory.join("status.json");
    std::os::unix::fs::symlink(&db, &output_path).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
        .arg("rate-limit-status")
        .arg("--db")
        .arg(&db)
        .arg("--output")
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(fs::symlink_metadata(&output_path)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(fs::read(&db).unwrap(), before);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn invalid_fetch_pacing_options_fail_before_creating_the_database() {
    let directory = test_directory();
    let db = directory.join("pacing.db");
    for value in ["NaN", "inf", "-1", "1e100", "invalid"] {
        let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .args(["fetch-request-page", "42", "--shared-rate-limit-db"])
            .arg(&db)
            .arg(format!("--minimum-interval-seconds={value}"))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{value}");
        assert!(output.stdout.is_empty());
        assert!(!db.exists());
    }
    for option in ["--rate-limit-name=example", "--minimum-interval-seconds=2"] {
        let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .args(["fetch-request-page", "42", option])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn fetch_rejects_a_database_path_used_for_evidence_before_any_acquisition() {
    let directory = test_directory();
    let db = directory.join("pacing.db");
    let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
        .current_dir(&directory)
        .args(["fetch-request-page", "42", "--shared-rate-limit-db"])
        .arg(&db)
        .arg("--receipt")
        .arg(&db)
        .args(["--raw-file", "raw.bin", "--result-file", "result.json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("shared pacing database and evidence output paths must be distinct"));
    assert!(fs::read_dir(&directory).unwrap().next().is_none());
    fs::remove_dir_all(directory).unwrap();
}
