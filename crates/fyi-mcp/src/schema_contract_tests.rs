//! Synthetic executable contracts for the catalogue, validation and safe errors.

use super::*;
use policy::{RemoteBudget, RemoteInstancePolicy};
use std::collections::{BTreeMap, BTreeSet};

async fn database() -> DbPool {
    let db = DbPool::new_in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    ensure_authorities_table(db.pool()).await.unwrap();
    db
}

fn rpc(method: &str, params: Value) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(17)),
        method: method.into(),
        params: Some(params),
    }
}

async fn call(db: &DbPool, name: &str, arguments: Value) -> JsonRpcResponse {
    handle_jsonrpc_request(
        db,
        rpc("tools/call", json!({"name":name, "arguments":arguments})),
    )
    .await
    .unwrap()
}

fn definition(name: &str) -> &Value {
    all_tool_catalogue()["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == name)
        .unwrap()
}

fn assert_success_contract(name: &str, response: JsonRpcResponse) -> Value {
    assert!(response.error.is_none(), "{name}: {:?}", response.error);
    let result = response.result.unwrap();
    assert_ne!(
        result.get("isError"),
        Some(&json!(true)),
        "{name}: {result}"
    );
    let payload = result.get("structuredContent").unwrap().clone();
    let mut validated = payload.clone();
    assert_eq!(
        arguments::validate_arguments(&definition(name)["outputSchema"], &mut validated),
        Ok(()),
        "{name} returned a result outside its advertised output schema: {payload}"
    );
    assert_eq!(validated, payload, "output must not need coercion: {name}");
    let text = result["content"][0]["text"].as_str().unwrap();
    assert_eq!(serde_json::from_str::<Value>(text).unwrap(), payload);
    payload
}

fn assert_failed(response: &JsonRpcResponse) {
    assert!(
        response.error.is_some()
            || response
                .result
                .as_ref()
                .is_some_and(|value| value["isError"] == true)
    );
}

#[test]
fn every_tool_has_supported_input_output_schema_and_complete_annotations() {
    let tools = all_tool_catalogue()["tools"].as_array().unwrap();
    let mut names = BTreeSet::new();
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        assert!(names.insert(name), "duplicate tool: {name}");
        assert!(!tool["description"].as_str().unwrap().is_empty());
        for key in ["inputSchema", "outputSchema"] {
            assert_eq!(tool[key]["type"], "object", "{name}: {key}");
            assert_eq!(
                arguments::validate_schema(&tool[key]),
                Ok(()),
                "{name}: {key}"
            );
        }
        for hint in [
            "readOnlyHint",
            "destructiveHint",
            "idempotentHint",
            "openWorldHint",
        ] {
            assert!(tool["annotations"][hint].is_boolean(), "{name}: {hint}");
        }
        if is_remote_write(name) {
            assert_eq!(tool["annotations"]["readOnlyHint"], false, "{name}");
        }
    }
}

#[test]
fn full_catalogue_matches_versioned_executable_snapshot() {
    let snapshot: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/remote_mcp/tools-v1.json"
    ))
    .unwrap();
    assert_eq!(snapshot["schema_version"], 1);
    fn by_name(tools: &Value) -> BTreeMap<String, Value> {
        let rows = tools.as_array().unwrap();
        let indexed: BTreeMap<_, _> = rows
            .iter()
            .map(|tool| (tool["name"].as_str().unwrap().to_string(), tool.clone()))
            .collect();
        assert_eq!(
            indexed.len(),
            rows.len(),
            "duplicate tool names in snapshot"
        );
        indexed
    }
    assert_eq!(
        by_name(&all_tool_catalogue()["tools"]),
        by_name(&snapshot["tools"])
    );
}

#[tokio::test]
async fn local_success_results_match_the_advertised_schemas() {
    let db = database().await;
    for (name, arguments) in [
        ("list_requests", json!({})),
        ("list_authorities", json!({})),
        ("check_status", json!({})),
        ("sync_status", json!({})),
        ("sync_status", json!({"request_id":999})),
        ("sync_monitor", json!({})),
        ("sync_conflicts", json!({"limit":1})),
        (
            "compute_deadline",
            json!({"start_date":"2026-11-14", "days":2, "calendar":true}),
        ),
        ("search_corpus", json!({"query":"synthetic", "limit":1})),
        (
            "import_authorities",
            json!({"authorities":[{"slug":"synthetic", "name":"Synthetic authority", "url":null}]}),
        ),
    ] {
        assert_success_contract(name, call(&db, name, arguments).await);
    }
    let created = assert_success_contract(
        "create_request",
        call(
            &db,
            "create_request",
            json!({"title":"Synthetic request", "body":"Synthetic body"}),
        )
        .await,
    );
    let id = created["request"]["id"].clone();
    for (name, arguments) in [
        ("retrieve_request", json!({"id":id})),
        ("sync_status", json!({"request_id":id})),
        (
            "update_request",
            json!({"id":id, "title":"Synthetic updated", "body":"Synthetic updated body"}),
        ),
        (
            "sync_resolve_conflict",
            json!({"request_id":id, "mark_clean":true}),
        ),
        ("list_requests", json!({"limit":1})),
        ("list_authorities", json!({})),
        ("delete_request", json!({"id":id})),
    ] {
        assert_success_contract(name, call(&db, name, arguments).await);
    }
    let capabilities: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/endorsed-client-route/enabled.json"
    ))
    .unwrap();
    assert_success_contract("endorsed_route_status", call(&db, "endorsed_route_status", json!({"capabilities":capabilities, "client_id":"fyi-cli-prod", "scopes":["read"], "now_epoch":1_700_000_000})).await);
}

#[test]
fn deadline_count_is_bounded_before_any_calculation() {
    let schema = &definition("compute_deadline")["inputSchema"];
    for days in [
        json!(u32::MAX),
        json!(u64::from(u32::MAX) + 1),
        json!(u64::MAX),
        json!(36_501),
        json!(-1),
        json!(1.5),
    ] {
        let mut input = json!({"start_date":"2026-11-14", "days":days});
        assert!(
            arguments::validate_arguments(schema, &mut input).is_err(),
            "unbounded deadline accepted: {days}"
        );
    }
    for days in [0, 1, 36_500] {
        let mut input = json!({"start_date":"2026-11-14", "days":days});
        assert!(arguments::validate_arguments(schema, &mut input).is_ok());
    }
}

#[tokio::test]
async fn invalid_import_is_rejected_before_any_authority_write() {
    let db = database().await;
    for invalid in [
        json!({"slug":" ", "name":"Synthetic invalid authority"}),
        json!({"slug":"synthetic-invalid", "name":"\t\n"}),
        json!({"slug":false, "name":"Synthetic invalid authority"}),
        json!({"slug":"synthetic-invalid", "name":"Synthetic invalid authority", "unknown":true}),
    ] {
        let response = call(&db, "import_authorities", json!({"authorities":[{"slug":"synthetic-first", "name":"Synthetic first authority"},invalid]})).await;
        assert_failed(&response);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM authorities")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(count, 0, "invalid import partly wrote its first row");
    }
}

#[tokio::test]
async fn invalid_write_arguments_are_rejected_before_any_database_access() {
    let db = database().await;
    db.pool().close().await;
    for (name, arguments) in [
        (
            "create_request",
            json!({"title":false, "body":"Synthetic body"}),
        ),
        (
            "create_request",
            json!({"title":"Synthetic", "body":"Synthetic", "synthetic-private-key":true}),
        ),
        ("delete_request", json!({"id":1.5})),
        (
            "update_request",
            json!({"id":true, "title":"Synthetic", "body":"Synthetic"}),
        ),
        (
            "import_authorities",
            json!({"authorities":[{"slug":"synthetic", "name":false}]}),
        ),
    ] {
        let response = call(&db, name, arguments).await;
        let error = response.error.unwrap();
        assert_eq!(error.code, -32602);
        assert_eq!(
            error.message,
            "Tool arguments do not match the advertised schema"
        );
        assert!(response.result.is_none());
    }
}

#[tokio::test]
async fn caller_text_is_not_reflected_in_prompt_resource_or_date_errors() {
    let db = database().await;
    for request in [
        rpc("prompts/get", json!({"name":"synthetic-private-marker"})),
        rpc(
            "resources/read",
            json!({"uri":"fyi://requests/synthetic-private-marker"}),
        ),
        rpc(
            "resources/read",
            json!({"uri":"https://synthetic-private-marker.example/path?secret=value"}),
        ),
        rpc(
            "tools/call",
            json!({"name":"compute_deadline", "arguments":{"start_date":"synthetic-private-marker", "days":1}}),
        ),
    ] {
        let response = handle_jsonrpc_request(&db, request).await.unwrap();
        assert_failed(&response);
        assert!(!serde_json::to_string(&response)
            .unwrap()
            .contains("synthetic-private-marker"));
    }
}

#[tokio::test]
async fn database_error_content_is_private_in_tool_results() {
    let db = database().await;
    sqlx::query("CREATE TRIGGER fail_synthetic_insert BEFORE INSERT ON requests BEGIN SELECT RAISE(ABORT, 'synthetic-private-database-marker'); END")
        .execute(db.pool()).await.unwrap();
    let response = call(
        &db,
        "create_request",
        json!({"title":"Synthetic", "body":"Synthetic body"}),
    )
    .await;
    assert_failed(&response);
    assert!(!serde_json::to_string(&response)
        .unwrap()
        .contains("synthetic-private-database-marker"));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM requests")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn database_error_content_is_private_in_resource_errors() {
    let db = database().await;
    sqlx::query("DROP TABLE requests")
        .execute(db.pool())
        .await
        .unwrap();
    sqlx::query("CREATE VIEW requests AS SELECT * FROM synthetic_private_database_marker")
        .execute(db.pool())
        .await
        .unwrap();
    let response =
        handle_jsonrpc_request(&db, rpc("resources/read", json!({"uri":"fyi://requests"})))
            .await
            .unwrap();
    assert_failed(&response);
    let encoded = serde_json::to_string(&response).unwrap();
    assert!(!encoded.contains("synthetic_private_database_marker"));
    assert!(!encoded.contains("no such table"));
}

#[tokio::test]
async fn disabled_remote_catalogue_entries_cannot_authorize_a_call() {
    let db = database().await;
    let disabled = RemoteMcpPolicy::new(vec![]).unwrap();
    assert!(registered_tool_catalogue(&disabled)["tools"]
        .as_array()
        .unwrap()
        .iter()
        .all(|tool| !tool["name"].as_str().unwrap().starts_with("remote_")));
    let gate = RemoteWriteGate::default();
    for (name, arguments) in [
        ("remote_health", json!({"instance_id":"synthetic"})),
        ("remote_version", json!({"instance_id":"synthetic"})),
        (
            "remote_search_requests",
            json!({"instance_id":"synthetic", "query":"synthetic", "limit":1}),
        ),
        (
            "remote_get_request",
            json!({"instance_id":"synthetic", "request_id":1}),
        ),
        (
            "remote_list_authorities",
            json!({"instance_id":"synthetic", "limit":1}),
        ),
        (
            "remote_create_request",
            json!({"instance_id":"synthetic", "phase":"prepare", "title":"Synthetic", "body":"Synthetic body", "external_user_name":"Synthetic", "external_url":"https://example.test/synthetic", "idempotency_key":"fixture"}),
        ),
        (
            "remote_add_correspondence",
            json!({"instance_id":"synthetic", "phase":"prepare", "request_id":1, "direction":"request", "body":"Synthetic body", "sent_at":"2026-11-14T00:00:00Z", "idempotency_key":"fixture"}),
        ),
        (
            "remote_update_state",
            json!({"instance_id":"synthetic", "phase":"prepare", "request_id":1, "expected_current_state":"waiting_response", "state":"successful", "idempotency_key":"fixture"}),
        ),
    ] {
        let response = handle_jsonrpc_request_with_policy_and_gate(
            &db,
            rpc("tools/call", json!({"name":name, "arguments":arguments})),
            &disabled,
            &gate,
        )
        .await
        .unwrap();
        assert!(
            response.error.is_none(),
            "known but disabled tool must return its capability error: {name}"
        );
        assert_eq!(response.result.unwrap()["isError"], true, "{name}");
    }
    assert!(disabled.status().instances.is_empty());
    let read_only = RemoteMcpPolicy::new(vec![RemoteInstancePolicy {
        instance_id: "synthetic".into(),
        base_url: "https://foi-repository.example".into(),
        read_enabled: true,
        write_enabled: false,
        budget: RemoteBudget::default(),
    }])
    .unwrap();
    let catalogue = registered_tool_catalogue(&read_only);
    assert!(catalogue["tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool["name"] == "remote_health"));
    assert!(catalogue["tools"]
        .as_array()
        .unwrap()
        .iter()
        .all(|tool| !is_remote_write(tool["name"].as_str().unwrap())));
}

#[tokio::test]
async fn versioned_handler_exchanges_match_actual_wire_dispatch() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/remote_mcp/v2.json")).unwrap();
    assert_eq!(fixture["schema_version"], 2);
    assert_eq!(fixture["protocol_version"], MCP_PROTOCOL_VERSION);
    let db = database().await;
    let mut exercised = 0;
    for case in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["verification"] == "handler")
    {
        let name = case["name"].as_str().unwrap();
        let enabled = name == "remote_confirmation_rejected";
        if enabled {
            // The enabled case must remain an unprepared write commit. These
            // assertions prevent fixture drift from turning an offline test
            // into a network read or a successfully prepared remote operation.
            assert_eq!(case["request"]["method"], "tools/call");
            assert_eq!(case["request"]["params"]["name"], "remote_create_request");
            assert_eq!(case["request"]["params"]["arguments"]["phase"], "commit");
        }
        let policy = RemoteMcpPolicy::new(vec![RemoteInstancePolicy {
            instance_id: "synthetic".into(),
            base_url: "https://foi-repository.example".into(),
            read_enabled: enabled,
            write_enabled: enabled,
            budget: RemoteBudget::default(),
        }])
        .unwrap();
        let gate = RemoteWriteGate::default();
        let bytes = case
            .get("request_text")
            .and_then(Value::as_str)
            .map(|text| text.as_bytes().to_vec())
            .unwrap_or_else(|| serde_json::to_vec(&case["request"]).unwrap());
        let mut version = MCP_PROTOCOL_VERSION;
        let actual = handle_wire_frame(&db, &bytes, &policy, &gate, &mut version)
            .await
            .unwrap();
        assert_eq!(actual, case["response"], "fixture drift in {name}");
        assert_eq!(
            policy.status().instances[0].requests_used,
            0,
            "handler fixture must never admit remote I/O: {name}"
        );
        exercised += 1;
    }
    assert!(
        exercised >= 8,
        "handler fixtures must not silently disappear"
    );
}

#[test]
fn synthetic_output_fixtures_match_schemas_and_actual_mcp_envelopes() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/remote_mcp/v2.json")).unwrap();
    let mut tools = BTreeSet::new();
    for case in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["verification"] == "synthetic_output")
    {
        let tool = case["request"]["params"]["name"].as_str().unwrap();
        let id = case["request"].get("id").cloned();
        let expected = &case["response"];
        let response = if case["kind"] == "tool_success" {
            let response = tool_success(id, expected["result"]["structuredContent"].clone());
            assert_success_contract(tool, response.clone());
            tools.insert(tool);
            response
        } else {
            assert_eq!(case["kind"], "tool_error");
            tool_error(
                id,
                expected["result"]["content"][0]["text"].as_str().unwrap(),
            )
        };
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            *expected,
            "fixture envelope drift: {}",
            case["name"]
        );
    }
    assert_eq!(
        tools.len(),
        8,
        "every remote tool needs a synthetic success contract"
    );
}

#[test]
fn endorsed_route_lists_and_identifiers_are_bounded_before_authorization() {
    let capabilities: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/endorsed-client-route/enabled.json"
    ))
    .unwrap();
    let original = json!({
        "capabilities": capabilities,
        "client_id": "fyi-cli-prod",
        "scopes": ["read"],
        "now_epoch": 1_700_000_000
    });
    let schema = &definition("endorsed_route_status")["inputSchema"];
    for pointer in [
        "/scopes",
        "/capabilities/scopes",
        "/capabilities/client_allowlist",
    ] {
        let mut input = original.clone();
        *input.pointer_mut(pointer).unwrap() = json!(vec!["synthetic"; 129]);
        assert!(arguments::validate_arguments(schema, &mut input).is_err());
        *input.pointer_mut(pointer).unwrap() = json!(vec!["synthetic"; 128]);
        assert!(arguments::validate_arguments(schema, &mut input).is_ok());
        *input.pointer_mut(pointer).unwrap() = json!(["x".repeat(257)]);
        assert!(arguments::validate_arguments(schema, &mut input).is_err());
        *input.pointer_mut(pointer).unwrap() = json!(["x".repeat(256)]);
        assert!(arguments::validate_arguments(schema, &mut input).is_ok());
    }
    let mut input = original;
    input["client_id"] = json!("x".repeat(257));
    assert!(arguments::validate_arguments(schema, &mut input).is_err());
    input["client_id"] = json!("x".repeat(256));
    assert!(arguments::validate_arguments(schema, &mut input).is_ok());
}
