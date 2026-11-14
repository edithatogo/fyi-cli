//! Regression tests for initial request routing, not only redirect routing.
use super::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "synthetic-origin-test-only";

async fn guarded_get(client: &SyncClient, url: &str) -> Result<GuardedResponse> {
    tokio::time::timeout(
        Duration::from_secs(5),
        client.send_guarded(client.http.get(url)),
    )
    .await
    .expect("local guarded request must remain bounded")
}

#[tokio::test]
async fn cross_origin_never_receives_bot_header() {
    let approved = MockServer::start().await;
    let other = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("fixture"))
        .mount(&other)
        .await;
    let client = SyncClient::new_for_testing(&approved.uri())
        .unwrap()
        .with_bot_token(Some(TOKEN.to_string()));
    let result = guarded_get(&client, &format!("{}/feed", other.uri())).await;
    assert!(result.is_err(), "cross-origin request must be rejected before HTTP");
    assert!(other.received_requests().await.unwrap().is_empty());
    assert!(approved.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn same_origin_request_preserves_body_and_token() {
    let approved = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/feed"))
        .respond_with(ResponseTemplate::new(200).set_body_string("fixture"))
        .expect(1)
        .mount(&approved)
        .await;
    let client = SyncClient::new_for_testing(&approved.uri())
        .unwrap()
        .with_bot_token(Some(TOKEN.to_string()));
    let response = guarded_get(&client, &format!("{}/feed?cursor=local", approved.uri()))
        .await
        .unwrap();
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body, b"fixture".to_vec());
    let received = approved.received_requests().await.unwrap();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].headers.get("x-fyi-bot-token").unwrap(), TOKEN);
}

#[tokio::test]
async fn origin_rejection_precedes_token_parsing() {
    let approved = MockServer::start().await;
    let other = MockServer::start().await;
    let client = SyncClient::new_for_testing(&approved.uri())
        .unwrap()
        .with_bot_token(Some("invalid\nsynthetic-token".to_string()));
    let error = match guarded_get(&client, &format!("{}/feed", other.uri())).await {
        Err(error) => error,
        Ok(_) => panic!("cross-origin request must fail"),
    };
    assert!(error.to_string().contains("configured source origin"));
    assert!(other.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn origin_denial_does_not_echo_url_query_or_token() {
    let approved = MockServer::start().await;
    let other = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&other)
        .await;
    let client = SyncClient::new_for_testing(&approved.uri())
        .unwrap()
        .with_bot_token(Some(TOKEN.to_string()));
    let url = format!("{}/private-path?key=synthetic-query-value", other.uri());
    let error = match guarded_get(&client, &url).await {
        Err(error) => error,
        Ok(_) => panic!("cross-origin request must fail"),
    };
    let message = format!("{error:#}");
    assert!(message.contains("configured source origin"));
    assert!(!message.contains(&url));
    assert!(!message.contains("synthetic-query-value"));
    assert!(!message.contains(TOKEN));
    assert!(other.received_requests().await.unwrap().is_empty());
}
