//! Focused HTTP/JSON-RPC negative cases, complementing executable stdio tests.

use super::*;

async fn state() -> HttpState {
    let db = DbPool::new_in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    HttpState {
        db,
        remote_policy: RemoteMcpPolicy::new(Vec::new()).unwrap(),
        write_gate: RemoteWriteGate::default(),
        bearer_token: Some(Arc::from("synthetic-contract-token")),
    }
}

fn request(body: impl Into<axum::body::Body>) -> Request {
    Request::builder()
        .header(header::AUTHORIZATION, "Bearer synthetic-contract-token")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION)
        .body(body.into())
        .unwrap()
}

async fn post(state: &HttpState, request: Request) -> Response {
    http_mcp(State(state.clone()), request.headers().clone(), request).await
}

async fn body(response: Response) -> Value {
    serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), MAX_MCP_FRAME_BYTES)
            .await
            .unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn http_authentication_precedes_parsing_and_is_private() {
    let state = state().await;
    for authorization in [
        None,
        Some("Bearer invalid"),
        Some("Basic synthetic-contract-token"),
    ] {
        let mut req = request("{private-synthetic-marker");
        req.headers_mut().remove(header::AUTHORIZATION);
        if let Some(value) = authorization {
            req.headers_mut()
                .insert(header::AUTHORIZATION, value.parse().unwrap());
        }
        let response = post(&state, req).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.headers()[header::WWW_AUTHENTICATE],
            "Bearer realm=\"fyi-mcp\""
        );
        assert!(axum::body::to_bytes(response.into_body(), 32)
            .await
            .unwrap()
            .is_empty());
    }
    let mut req = request("{}");
    req.headers_mut().append(
        header::AUTHORIZATION,
        "Bearer synthetic-contract-token".parse().unwrap(),
    );
    assert_eq!(post(&state, req).await.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn http_rejects_browser_origins_even_when_authenticated() {
    let state = state().await;
    for origin in ["https://untrusted.example", "null", "http://127.0.0.1"] {
        let mut req = request("{}");
        req.headers_mut()
            .insert(header::ORIGIN, origin.parse().unwrap());
        let headers = req.headers().clone();
        assert_eq!(post(&state, req).await.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            http_mcp_get(State(state.clone()), headers).await.status(),
            StatusCode::FORBIDDEN
        );
    }
}

#[tokio::test]
async fn http_checks_media_types_versions_and_body_bounds() {
    let state = state().await;
    for accept in [
        "application/json",
        "text/event-stream",
        "application/json, text/event-stream;q=0",
        "application/json, text/event-stream;q=bad",
    ] {
        let mut req = request("{}");
        req.headers_mut()
            .insert(header::ACCEPT, accept.parse().unwrap());
        assert_eq!(post(&state, req).await.status(), StatusCode::BAD_REQUEST);
    }
    let mut req = request("{}");
    req.headers_mut()
        .insert("MCP-Protocol-Version", "2099-01-01".parse().unwrap());
    assert_eq!(
        http_mcp_get(State(state.clone()), req.headers().clone())
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(post(&state, req).await.status(), StatusCode::BAD_REQUEST);
    let mut req = request("{}");
    req.headers_mut().append(
        "MCP-Protocol-Version",
        MCP_FALLBACK_PROTOCOL_VERSION.parse().unwrap(),
    );
    assert_eq!(post(&state, req).await.status(), StatusCode::BAD_REQUEST);
    let mut req = request("{}");
    req.headers_mut().insert(
        "MCP-Protocol-Version",
        axum::http::HeaderValue::from_bytes(&[0xff]).unwrap(),
    );
    assert_eq!(post(&state, req).await.status(), StatusCode::BAD_REQUEST);
    for content_type in [None, Some("text/plain")] {
        let mut req = request("{}");
        req.headers_mut().remove(header::CONTENT_TYPE);
        if let Some(value) = content_type {
            req.headers_mut()
                .insert(header::CONTENT_TYPE, value.parse().unwrap());
        }
        assert_eq!(
            post(&state, req).await.status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
    }
    assert_eq!(
        post(&state, request(vec![b'x'; MAX_MCP_FRAME_BYTES + 1]))
            .await
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[tokio::test]
async fn http_returns_json_rpc_errors_and_empty_notification_responses() {
    let state = state().await;
    let response = post(&state, request("{private-synthetic-marker")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await,
        json!({"jsonrpc":"2.0", "id":null, "error":{"code":-32700, "message":"Parse error"}})
    );
    for message in [
        json!({"jsonrpc":"2.0", "method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0", "id":1, "result":{}}),
        json!({"jsonrpc":"2.0", "id":1, "error":{"code":-32601,"message":"synthetic"}}),
    ] {
        let response = post(&state, request(message.to_string())).await;
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert!(axum::body::to_bytes(response.into_body(), 32)
            .await
            .unwrap()
            .is_empty());
    }
    let response = post(
        &state,
        request(json!({"jsonrpc":"2.0", "id":0, "method":"ping"}).to_string()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await,
        json!({"jsonrpc":"2.0", "id":0, "result":{}})
    );
    let headers = request("").headers().clone();
    let response = http_mcp_get(State(state.clone()), headers).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(response.headers()[header::ALLOW], "POST");
    assert_eq!(
        http_mcp_get(State(state), HeaderMap::new()).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn http_legacy_fallback_accepts_batches_but_current_revision_rejects_them() {
    let state = state().await;
    let batch = json!([
        {"jsonrpc":"2.0", "id":1, "method":"ping"},
        {"jsonrpc":"2.0", "method":"notifications/initialized"}
    ]);
    for version in [
        None,
        Some(MCP_FALLBACK_PROTOCOL_VERSION),
        Some(MCP_PROTOCOL_VERSION),
    ] {
        let mut req = request(batch.to_string());
        req.headers_mut().remove("MCP-Protocol-Version");
        if let Some(value) = version {
            req.headers_mut()
                .insert("MCP-Protocol-Version", value.parse().unwrap());
        }
        let response = body(post(&state, req).await).await;
        if version == Some(MCP_PROTOCOL_VERSION) {
            assert_eq!(response["error"]["code"], -32600);
        } else {
            assert_eq!(response, json!([{"jsonrpc":"2.0", "id":1, "result":{}}]));
        }
    }
}

#[tokio::test]
async fn invalid_initialization_and_ambiguous_envelopes_are_rejected() {
    let state = state().await;
    for params in [
        Value::Null,
        json!({}),
        json!({"protocolVersion":17}),
        json!({"protocolVersion":"", "capabilities":{}, "clientInfo":{"name":"test","version":"1"}}),
    ] {
        let response = body(
            post(
                &state,
                request(
                    json!({"jsonrpc":"2.0", "id":1, "method":"initialize", "params":params})
                        .to_string(),
                ),
            )
            .await,
        )
        .await;
        assert_eq!(response["error"]["code"], -32602);
    }
    for message in [
        json!(true),
        json!({}),
        json!({"jsonrpc":"2.0","id":1,"result":{},"error":{"code":1,"message":"bad"}}),
        json!({"jsonrpc":"2.0","id":1,"method":"ping","result":{}}),
    ] {
        let response = body(post(&state, request(message.to_string())).await).await;
        assert_eq!(response["error"]["code"], -32600);
        assert!(response["id"].is_null());
    }
}

#[tokio::test]
async fn legacy_batch_size_is_bounded_and_notification_only_batch_is_silent() {
    let state = state().await;
    let notification = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
    let mut version = MCP_FALLBACK_PROTOCOL_VERSION;
    for batch in [
        json!([]),
        json!(vec![notification.clone(); MAX_MCP_BATCH_MESSAGES + 1]),
    ] {
        let result = handle_wire_frame(
            &state.db,
            batch.to_string().as_bytes(),
            &state.remote_policy,
            &state.write_gate,
            &mut version,
        )
        .await
        .unwrap();
        assert_eq!(result["error"]["code"], -32600);
    }
    assert!(handle_wire_frame(
        &state.db,
        json!([notification]).to_string().as_bytes(),
        &state.remote_policy,
        &state.write_gate,
        &mut version
    )
    .await
    .is_none());
}

#[tokio::test]
async fn initialization_cannot_be_batched_and_notification_methods_cannot_be_requests() {
    let state = state().await;
    let message = json!({"jsonrpc":"2.0","id":7,"method":"notifications/initialized"});
    let response = body(post(&state, request(message.to_string())).await).await;
    assert_eq!(response["id"], 7);
    assert_eq!(response["error"]["code"], -32600);
    let mut req = request(json!([
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"synthetic","version":"1"}}},
        {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_request","arguments":{"title":"must not persist","body":"synthetic"}}}
    ]).to_string());
    req.headers_mut().remove("MCP-Protocol-Version");
    assert_eq!(body(post(&state, req).await).await["error"]["code"], -32600);
    assert!(state.db.list_requests(1).await.unwrap().is_empty());
}
