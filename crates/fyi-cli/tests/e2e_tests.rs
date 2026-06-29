use fyi_core::api::{AlaveteliRequest, CreateRequestPayload, CreateRequestResponse};
use fyi_core::db::DbPool;
use std::fs;
use std::path::Path;
use std::process::Command;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_e2e_cli_and_api_flow() {
    let db_path = "e2e_temp_fyi_system.db";
    let db_path_abs = std::env::current_dir()
        .expect("Failed to resolve current directory")
        .join(db_path);
    let db_url = format!("sqlite://{}", db_path_abs.display());

    // Clean up any existing database file
    if Path::new(db_path).exists() {
        let _ = fs::remove_file(db_path);
    }

    // 1. Invoke Binary to initialize database
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fyi-cli"));
    cmd.args(["--db", db_path, "init-db", "--db", db_path]);

    let output = cmd.output().expect("Failed to execute fyi-cli init-db");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Initialized SQLite database"));

    // 2. Open DB and run migrations (simulating CLI DB integration)
    let pool = DbPool::new(&db_url)
        .await
        .expect("Failed to connect to CLI-created database");
    pool.run_migrations()
        .await
        .expect("Failed to run migrations on CLI-created database");
    assert!(
        Path::new(db_path).exists(),
        "Database file should be created by database initialization"
    );

    // Insert dummy request via db pool
    let request = AlaveteliRequest {
        id: 42,
        title: "E2E Test Request".to_string(),
        body: "I require access to public documents under the OIA.".to_string(),
        user_name: Some("Test User".to_string()),
        status: Some("awaiting_triage".to_string()),
        created_at: Some("2026-06-15T00:00:00Z".to_string()),
        updated_at: Some("2026-06-15T00:00:00Z".to_string()),
        url: Some("https://fyi.org.nz/request/42".to_string()),
        tags: Some(vec!["e2e".to_string()]),
    };
    pool.insert_request(&request)
        .await
        .expect("Failed to insert request into DB");

    // Query request from DB and assert consistency
    let retrieved = pool
        .get_request(42)
        .await
        .expect("Failed to query request")
        .expect("Request not found");
    assert_eq!(retrieved.id, 42);
    assert_eq!(retrieved.title, "E2E Test Request");

    // 3. Set up mock API using wiremock to simulate server interaction
    let mock_server = MockServer::start().await;

    let payload = CreateRequestPayload {
        title: "E2E Test Request".to_string(),
        body: "I require access to public documents under the OIA.".to_string(),
        external_user_name: "Test User".to_string(),
        external_url: "https://fyi.org.nz/request/42".to_string(),
        tags: Some("e2e".to_string()),
    };

    let expected_response = CreateRequestResponse {
        id: 10042,
        url: format!("{}/request/10042", mock_server.uri()),
    };

    Mock::given(method("POST"))
        .and(path("/api/v2/request"))
        .and(body_json(&payload))
        .respond_with(ResponseTemplate::new(201).set_body_json(&expected_response))
        .mount(&mock_server)
        .await;

    // Simulate API client loop sending request payload to mock server
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/v2/request", mock_server.uri()))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request to mock API server");

    assert_eq!(response.status(), 201);
    let api_resp: CreateRequestResponse =
        response.json().await.expect("Failed to parse API response");
    assert_eq!(api_resp.id, 10042);
    assert!(api_resp.url.contains("/request/10042"));

    // 4. Update local DB with new ID returned from simulated API post
    let mut updated_request = request.clone();
    updated_request.id = api_resp.id;
    updated_request.url = Some(api_resp.url);
    updated_request.status = Some("sent".to_string());

    pool.insert_request(&updated_request)
        .await
        .expect("Failed to update request with API result");

    let retrieved_updated = pool
        .get_request(10042)
        .await
        .expect("Failed to query updated request")
        .expect("Updated request not found");
    assert_eq!(retrieved_updated.id, 10042);
    assert_eq!(retrieved_updated.status, Some("sent".to_string()));

    // 5. Invoke another CLI subcommand to verify arguments parse correctly
    let mut cmd2 = Command::new(env!("CARGO_BIN_EXE_fyi-cli"));
    cmd2.args([
        "--db",
        db_path,
        "register-request",
        "slug-example",
        "My CLI Req",
        "Body from CLI",
        "--status",
        "draft",
        "--db",
        db_path,
    ]);
    let output2 = cmd2
        .output()
        .expect("Failed to execute register-request subcommand");
    assert!(output2.status.success());
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(stdout2.contains("Registering request: 'My CLI Req'"));

    // Clean up temporary database file
    let _ = fs::remove_file(db_path);
}
