//! Local authority CLI contracts: no transports, receipts, or network operations.
use fyi_core::db::DbPool;
use serde_json::json;
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
            std::env::temp_dir().join(format!("fyi-authorities-{}-{nonce}", std::process::id()));
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
}

impl Drop for Workspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn imports_local_csv_and_lists_stable_text_and_json() {
    let work = Workspace::new();
    work.success(&["init-db", "--db", "local.db"]);
    fs::write(
        work.0.join("authorities.csv"),
        "slug,name,url\nzeta,Zeta Agency,\nalpha,Alpha Agency,https://example.invalid/body/alpha\n",
    )
    .unwrap();
    assert_eq!(
        work.success(&["import-authorities", "authorities.csv", "--db", "local.db"]),
        "Imported 2 authorities\n"
    );
    assert_eq!(
        work.success(&["list-authorities", "--db", "local.db"]),
        "alpha\tAlpha Agency\thttps://example.invalid/body/alpha\nzeta\tZeta Agency\t\n"
    );
    let output = work.success(&[
        "--output-format",
        "json",
        "list-authorities",
        "--db",
        "local.db",
    ]);
    let rows: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(
        rows,
        json!([
            {"slug": "alpha", "name": "Alpha Agency", "url": "https://example.invalid/body/alpha"},
            {"slug": "zeta", "name": "Zeta Agency", "url": null}
        ])
    );
    work.success(&["import-authorities", "authorities.csv", "--db", "local.db"]);
    assert_eq!(
        work.success(&[
            "--output-format",
            "json",
            "list-authorities",
            "--db",
            "local.db"
        ]),
        output
    );
    fs::write(
        work.0.join("authorities.csv"),
        "authority_slug,public_body_name,request_url\nalpha,Updated Alpha,\n",
    )
    .unwrap();
    work.success(&["import-authorities", "authorities.csv", "--db", "local.db"]);
    assert!(work
        .success(&["list-authorities", "--db", "local.db"])
        .contains("alpha\tUpdated Alpha\t\n"));
}

#[test]
fn reads_do_not_create_databases_and_remote_import_options_are_unavailable() {
    let work = Workspace::new();
    let missing = work.run(&["list-authorities", "--db", "missing.db"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(!work.0.join("missing.db").exists());
    for args in [
        vec!["import-authorities"],
        vec![
            "import-authorities",
            "--source-url",
            "https://example.invalid",
        ],
    ] {
        assert_eq!(work.run(&args).status.code(), Some(2));
    }
    assert!(!work.0.join("fyi_system.db").exists());
}

#[tokio::test]
async fn invalid_late_row_leaves_authorities_unchanged_without_sensitive_diagnostics() {
    let work = Workspace::new();
    work.success(&["init-db", "--db", "local.db"]);
    fs::write(
        work.0.join("authorities.csv"),
        "slug,name,url\nexisting,Original,\n",
    )
    .unwrap();
    work.success(&["import-authorities", "authorities.csv", "--db", "local.db"]);
    fs::write(
        work.0.join("invalid.csv"),
        "slug,name,url\nexisting,Changed,\n,PRIVATE-SENTINEL,\n",
    )
    .unwrap();
    let result = work.run(&["import-authorities", "invalid.csv", "--db", "local.db"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&result.stderr).contains("PRIVATE-SENTINEL"));
    assert_eq!(
        work.success(&["list-authorities", "--db", "local.db"]),
        "existing\tOriginal\t\n"
    );
    let db = DbPool::new(&format!("sqlite://{}", work.0.join("local.db").display()))
        .await
        .unwrap();
    let queued: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_outgoing_queue")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(queued, 0);
    db.pool().close().await;
}
