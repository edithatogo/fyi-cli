//! Filesystem database paths must identify the file that the caller validated.

use fyi_core::db::DbPool;
use std::fs;
use std::path::Path;

async fn assert_persisted_at_literal_path(path: &Path) {
    let db = DbPool::new_file(path).await.unwrap();
    sqlx::query("CREATE TABLE file_identity (value INTEGER NOT NULL)")
        .execute(db.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO file_identity (value) VALUES (310)")
        .execute(db.pool())
        .await
        .unwrap();
    db.pool().close().await;
    assert!(
        path.is_file(),
        "literal database path was not created: {path:?}"
    );

    let reopened = DbPool::new_file(path).await.unwrap();
    let value: i64 = sqlx::query_scalar("SELECT value FROM file_identity")
        .fetch_one(reopened.pool())
        .await
        .unwrap();
    assert_eq!(value, 310);
    reopened.pool().close().await;
}

#[tokio::test]
async fn percent_encoded_filename_is_literal_and_persistent() {
    let directory = tempfile::tempdir_in(fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
    let literal = directory.path().join("sync%2edb");
    assert_persisted_at_literal_path(&literal).await;
    assert!(!directory.path().join("sync.db").exists());
}

#[cfg(unix)]
#[tokio::test]
async fn query_characters_are_filename_bytes_not_connection_options() {
    let directory = tempfile::tempdir_in(fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
    let literal = directory.path().join("sync?mode=memory&cache=shared");
    assert_persisted_at_literal_path(&literal).await;
    assert!(!directory.path().join("sync").exists());
}

#[cfg(unix)]
#[tokio::test]
async fn backslash_is_a_literal_unix_filename_character() {
    let directory = tempfile::tempdir_in(fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
    // Create the alternate parent so a slash-rewriting implementation would
    // open a different existing location instead of failing incidentally.
    fs::create_dir(directory.path().join("nested")).unwrap();
    let literal = directory.path().join(r"nested\sync.db");
    assert_persisted_at_literal_path(&literal).await;
    assert!(!directory.path().join("nested/sync.db").exists());
}

#[cfg(unix)]
#[test]
fn relative_sqlite_special_names_are_physical_files() {
    const CHILD_FILENAME: &str = "FYI_DB_FILE_PATH_TEST_FILENAME";
    if let Ok(filename) = std::env::var(CHILD_FILENAME) {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(assert_persisted_at_literal_path(Path::new(&filename)));
        return;
    }

    // Isolate cwd in a child process; changing the parent cwd would race other
    // tests. Passing these bare names specifically checks absolute-path handling.
    let directory = tempfile::tempdir_in(fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
    for filename in [":memory:", "file:shadow.db?mode=memory&cache=shared"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "relative_sqlite_special_names_are_physical_files",
                "--nocapture",
            ])
            .current_dir(directory.path())
            .env(CHILD_FILENAME, filename)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{filename}: stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(directory.path().join(filename).is_file());
    }
    assert!(!directory.path().join("shadow.db").exists());
}
