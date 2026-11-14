//! Execute the same local registration fixtures as the Python reference tests.

use fyi_core::db::local::requests::NewLocalRequest;
use fyi_core::db::local::LocalAuthority;
use fyi_core::db::DbPool;
use serde_json::Value;

const PYTHON_FIXTURES: &str =
    include_str!("../../../conformance/local-management/requests-v1-python.json");

fn fixture_request(case: &Value) -> NewLocalRequest {
    let arguments: Vec<&str> = case["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(arguments[0], "register-request");
    let mut request = NewLocalRequest {
        authority_slug: arguments[1].to_owned(),
        title: arguments[2].to_owned(),
        body: arguments[3].to_owned(),
        tags: String::new(),
        status: "draft".to_owned(),
        fyi_request_id: None,
    };
    let options = arguments[4..].chunks_exact(2);
    assert!(options.remainder().is_empty());
    for option in options {
        match option[0] {
            "--tags" => request.tags = option[1].to_owned(),
            "--status" => request.status = option[1].to_owned(),
            "--fyi-request-id" => request.fyi_request_id = Some(option[1].parse().unwrap()),
            other => panic!("unsupported option in checked-in fixture: {other}"),
        }
    }
    request
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

#[tokio::test]
async fn python_registration_fixtures_match_native_persisted_fields() {
    let corpus: Value = serde_json::from_str(PYTHON_FIXTURES).unwrap();
    assert_eq!(corpus["schema_version"], 1);
    for case in corpus["valid_cases"].as_array().unwrap() {
        let fixture = case["id"].as_str().unwrap();
        let db = database().await;
        let input = fixture_request(case);
        let id = db.register_local_request(&input).await.unwrap();
        assert_eq!(id, 1, "{fixture}");
        let stored = db.get_local_request(id).await.unwrap().unwrap();
        let mut actual = serde_json::to_value(stored).unwrap();
        let fields = actual.as_object_mut().unwrap();
        let created_at = fields.remove("created_at").unwrap();
        let updated_at = fields.remove("updated_at").unwrap();
        assert_eq!(created_at, updated_at, "{fixture}");
        assert!(
            chrono::DateTime::parse_from_rfc3339(created_at.as_str().unwrap()).is_ok(),
            "{fixture}"
        );
        assert_eq!(actual, case["expected_row_without_timestamps"], "{fixture}");
        db.pool().close().await;
    }
}

#[tokio::test]
async fn explicitly_documented_legacy_invalid_inputs_are_rejected_by_native_registration() {
    let corpus: Value = serde_json::from_str(PYTHON_FIXTURES).unwrap();
    for case in corpus["legacy_registration_differences"].as_array().unwrap() {
        let fixture = case["id"].as_str().unwrap();
        assert!(!case["rust_behavior"].as_str().unwrap().is_empty());
        let db = database().await;
        assert!(
            db.register_local_request(&fixture_request(case))
                .await
                .is_err(),
            "{fixture}"
        );
        let counts: (i64, i64) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM tracked_requests),
                    (SELECT COUNT(*) FROM local_request_events)",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(counts, (0, 0), "{fixture}");
        db.pool().close().await;
    }
}
