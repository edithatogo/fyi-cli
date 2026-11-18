use super::*;
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn limits() -> ReadLimits {
    ReadLimits {
        max_bytes: 16 * 1024,
        max_duration: Duration::from_secs(2),
    }
}

async fn client(server: &MockServer) -> SyncClient {
    let identity = crate::agent_runtime::ClientIdentity::default_identity(None).unwrap();
    let http = build_read_http_client(&identity).unwrap();
    let client = SyncClient::with_http_client_and_validation(&server.uri(), http, true).unwrap();
    {
        let mut middleware = client.middleware.lock().await;
        middleware.pacing.current_delay = Duration::ZERO;
        middleware.pacing.policy.baseline_delay = Duration::ZERO;
        middleware.pacing.policy.min_delay = Duration::ZERO;
    }
    client
}

fn assert_safe_kind(error: &anyhow::Error, kind: SourceFailureKind) {
    assert_eq!(source_failure_kind(error), Some(kind));
    let text = format!("{error:#}");
    assert!(!text.contains("synthetic-private"), "{text}");
    assert!(!text.contains("http://"), "{text}");
    assert!(!text.contains("https://"), "{text}");
}

#[test]
fn configured_read_client_validates_source_identity_and_limits() {
    let identity = crate::agent_runtime::ClientIdentity::default_identity(None).unwrap();
    let defaults = crate::agent_runtime::GuardrailConfig::default();
    assert!(
        SyncClient::with_guardrails("https://source.example", &identity, defaults.clone()).is_ok()
    );
    for invalid_url in [
        "http://source.example",
        "https://127.0.0.1",
        "https://localhost",
        "https://localhost.",
        "https://source.local",
        "https://source.local.",
        "https://[::1]",
        "https://[fc00::1]",
        "https://[fe80::1]",
        "https://[::ffff:127.0.0.1]",
        "https://private:synthetic-private@source.example",
        "https://source.example?synthetic-private",
        "https://source.example#synthetic-private",
    ] {
        let error = SyncClient::with_guardrails(invalid_url, &identity, defaults.clone())
            .err()
            .unwrap();
        assert_eq!(
            source_failure_kind(&error),
            Some(SourceFailureKind::Validation)
        );
    }
    for config in [
        crate::agent_runtime::GuardrailConfig {
            max_requests: 0,
            ..defaults.clone()
        },
        crate::agent_runtime::GuardrailConfig {
            max_response_bytes: 0,
            ..defaults.clone()
        },
        crate::agent_runtime::GuardrailConfig {
            max_runtime: Duration::ZERO,
            ..defaults.clone()
        },
        crate::agent_runtime::GuardrailConfig {
            max_concurrency: 0,
            ..defaults
        },
    ] {
        let error = SyncClient::with_guardrails("https://source.example", &identity, config)
            .err()
            .unwrap();
        assert_safe_kind(&error, SourceFailureKind::Validation);
    }
    let mut invalid_identity = identity;
    invalid_identity.product.clear();
    assert!(SyncClient::with_guardrails(
        "https://source.example",
        &invalid_identity,
        Default::default()
    )
    .is_err());
}

#[tokio::test]
async fn bounded_reads_keep_success_variants_and_matching_provenance() {
    let server = MockServer::start().await;
    let client = client(&server).await;
    let request = json!({"id": 7, "title": "synthetic", "body": "fixture"});
    for (route, body) in [
        ("/api/v2/request.json", json!({"requests": []})),
        ("/api/v2/version.json", json!({"version": "fixture-v1"})),
        ("/api/v2/request/7.json", request.clone()),
        (
            "/api/v2/authority.json",
            json!({"authorities": [{"id": "3", "name": "Synthetic Authority"}]}),
        ),
    ] {
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/search.json"))
        .and(query_param("q", "fixture & query"))
        .and(query_param("page", "2"))
        .and(query_param("per_page", "10"))
        .and(query_param("sort", "newest"))
        .and(query_param("filter", "successful"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results": [request]})))
        .expect(1)
        .mount(&server)
        .await;
    let health = client.health_check_read(&limits()).await.unwrap();
    assert!(health.value.network_reachable && health.value.api_reachable);
    let version = client.get_api_version_read(&limits()).await.unwrap();
    assert_eq!(version.value, "fixture-v1");
    assert_eq!(version.provenance.cache_status, CacheStatus::Miss);
    assert_eq!(version.provenance.source_status, 200);
    assert_eq!(
        version.provenance.response_bytes,
        json!({"version": "fixture-v1"}).to_string().len() as u64
    );
    assert!(crate::time_util::parse_strict_rfc3339(&version.provenance.retrieved_at).is_ok());
    assert_eq!(
        client
            .fetch_request_read(7, &limits())
            .await
            .unwrap()
            .value
            .id,
        7
    );
    assert_eq!(
        client.list_authorities_read(&limits()).await.unwrap().value[0]
            .id
            .as_deref(),
        Some("3")
    );
    let options = SearchOptions {
        query: Some("fixture & query".into()),
        page: Some(2),
        per_page: Some(10),
        sort: Some("newest".into()),
        filter: Some("successful".into()),
    };
    assert_eq!(
        client
            .search_requests_read(&options, &limits())
            .await
            .unwrap()
            .value
            .len(),
        1
    );
    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 5);
    assert!(received.iter().all(|request| request.method == "GET"));
    let user_agent = received[0]
        .headers
        .get("user-agent")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(user_agent.starts_with("fyi-cli/") && user_agent.contains("fp:"));
}

#[tokio::test]
async fn all_bounded_reads_reject_malformed_payloads_without_private_text() {
    let server = MockServer::start().await;
    let client = client(&server).await;
    for (route, body) in [
        (
            "/api/v2/version.json",
            "{\"version\": [\"synthetic-private\"]}",
        ),
        ("/search.json", "{\"results\": \"synthetic-private\"}"),
        (
            "/api/v2/request/7.json",
            "{\"id\":\"synthetic-private\",\"title\":\"x\",\"body\":\"x\"}",
        ),
        ("/api/v2/authority.json", "synthetic-private invalid JSON"),
    ] {
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
    }
    for error in [
        client.get_api_version_read(&limits()).await.unwrap_err(),
        client
            .search_requests_read(&SearchOptions::default(), &limits())
            .await
            .unwrap_err(),
        client.fetch_request_read(7, &limits()).await.unwrap_err(),
        client.list_authorities_read(&limits()).await.unwrap_err(),
    ] {
        assert_safe_kind(&error, SourceFailureKind::MalformedPayload);
    }
}

#[tokio::test]
async fn request_id_mismatch_is_a_malformed_source_result() {
    let server = MockServer::start().await;
    let client = client(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/v2/request/7.json"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"id":8,"title":"synthetic-private","body":"fixture"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    assert_safe_kind(
        &client.fetch_request_read(7, &limits()).await.unwrap_err(),
        SourceFailureKind::MalformedPayload,
    );
}

#[tokio::test]
async fn bounded_read_status_matrix_keeps_only_observed_status() {
    for status in [401, 403, 404, 429, 500, 502, 503] {
        let server = MockServer::start().await;
        let client = client(&server).await;
        Mock::given(method("GET"))
            .and(path("/api/v2/version.json"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("Retry-After", "synthetic-private-401")
                    .set_body_string("synthetic-private-503-body"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = client.get_api_version_read(&limits()).await.unwrap_err();
        assert_safe_kind(&error, SourceFailureKind::HttpStatus(status));
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn health_read_retains_http_failure_while_legacy_health_remains_boolean() {
    let server = MockServer::start().await;
    let client = client(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/v2/request.json"))
        .respond_with(ResponseTemplate::new(403).set_body_string("synthetic-private"))
        .expect(2)
        .mount(&server)
        .await;
    assert_safe_kind(
        &client.health_check_read(&limits()).await.unwrap_err(),
        SourceFailureKind::HttpStatus(403),
    );
    let legacy = client.health_check().await;
    assert!(legacy.network_reachable && !legacy.api_reachable);
}

#[tokio::test]
async fn authority_feed_discovery_validates_slug_without_source_io() {
    let server = MockServer::start().await;
    let client = client(&server).await;
    assert_eq!(
        client.authority_feed_url("health_A-7").unwrap(),
        format!("{}/feed/body/health_A-7", server.uri())
    );
    for slug in [
        "",
        "../other",
        "//host",
        "https://host",
        "a?x=y",
        "a#x",
        "a%2fb",
        "a b",
        "é",
    ] {
        assert_safe_kind(
            &client.authority_feed_url(slug).unwrap_err(),
            SourceFailureKind::Validation,
        );
    }
    assert!(client.authority_feed_url(&"a".repeat(129)).is_err());
    assert!(client.authority_feed_url(&"a".repeat(128)).is_ok());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[test]
fn authority_feed_discovery_preserves_configured_subpath() {
    for base in [
        "https://source.example/portal",
        "https://source.example/portal/",
    ] {
        let client = SyncClient::new(base).unwrap();
        assert_eq!(
            client.authority_feed_url("health").unwrap(),
            "https://source.example/portal/feed/body/health"
        );
    }
}

fn request_reference(path: &str, request_id: Option<i64>) -> AuthorityFeedReference {
    AuthorityFeedReference {
        kind: AuthorityFeedReferenceKind::Request,
        url: format!("https://source.example{path}"),
        request_id,
    }
}

fn event_reference(path: &str) -> AuthorityFeedReference {
    AuthorityFeedReference {
        kind: AuthorityFeedReferenceKind::RequestEvent,
        url: format!("https://source.example{path}"),
        request_id: None,
    }
}

fn parse_fixture_references(
    feed: &str,
    limit: usize,
) -> Result<(Vec<AuthorityFeedReference>, bool)> {
    feed_references(
        feed,
        &Url::parse("https://source.example/").unwrap(),
        &Url::parse("https://source.example/feed/body/health").unwrap(),
        limit,
    )
}

#[tokio::test]
async fn rss_and_atom_feed_plans_use_one_get_and_never_pull_records() {
    for (feed, expected_ids, reference_count) in [
        (
            r#"<rss version="2.0"><channel><title>Synthetic authority feed</title>
              <item><link>https://source.example/api/v2/request/1001.json</link></item>
              <item><link>https://source.example/api/v2/request/1002.json</link></item>
              <item><link>https://source.example/api/v2/request/1001.json#incoming-42</link></item>
            </channel></rss>"#,
            vec![1001, 1002],
            2,
        ),
        (
            r#"<feed xmlns="http://www.w3.org/2005/Atom">
              <entry><id>https://source.example/api/v2/request/2001.json</id>
                <link rel="alternate" href="https://source.example/request/synthetic_one"/></entry>
              <entry><id>https://source.example/api/v2/request/2002.json</id>
                <link href="https://source.example/request/synthetic_two"/></entry>
            </feed>"#,
            vec![2001, 2002],
            4,
        ),
    ] {
        let server = MockServer::start().await;
        let client = client(&server).await;
        let feed = feed.replace("https://source.example", &server.uri());
        Mock::given(method("GET"))
            .and(path("/feed/body/health"))
            .respond_with(ResponseTemplate::new(200).set_body_string(feed.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let result = client
            .plan_authority_feed("health", 200, &limits())
            .await
            .unwrap();
        assert_eq!(result.value.request_ids, expected_ids);
        assert_eq!(result.value.references.len(), reference_count);
        assert!(!result.value.truncated);
        assert_eq!(
            result.value.feed_url,
            format!("{}/feed/body/health", server.uri())
        );
        assert_eq!(result.provenance.response_bytes, feed.len() as u64);
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[test]
fn feed_plan_filters_foreign_invalid_and_unrelated_links_and_deduplicates() {
    let feed = r#"<feed><entry><link href="https://foreign.example/request/2"/>
      <link href="https://synthetic-private@source.example/request/3"/>
      <link href="https://source.example/request/4?secret=synthetic-private"/>
      <link href="https://source.example/request/5#fragment"/>
      <link href="https://source.example/request/5#other-fragment"/>
      <link href="https://source.example/api/v2/request/0.json"/>
      <link href="https://source.example/request/6oops"/>
      <link href="https://source.example/api/v2/request/9223372036854775808.json"/>
      <title>Unrelated /request/8 must not be admitted</title>
      <link href="/request/9/fixture/request/42"/>
      <id>https://source.example/api/v2/request/9.json</id>
      <link href="/body/authority"/></entry></feed>"#;
    assert_eq!(
        parse_fixture_references(feed, 200).unwrap(),
        (
            vec![
                request_reference("/api/v2/request/9.json", Some(9)),
                request_reference("/request/5", None),
                request_reference("/request/6oops", None),
            ],
            false
        )
    );
}

#[test]
fn realistic_atom_keeps_request_slugs_and_unresolved_event_references() {
    // Alaveteli Atom IDs identify InfoRequestEvent objects, while alternate
    // links point to event redirect routes or request URL titles.
    let feed = r#"<?xml version="1.0" encoding="UTF-8"?>
      <feed xmlns="http://www.w3.org/2005/Atom">
        <id>tag:source.example,2005:/track/body/health</id>
        <title>Synthetic authority changes</title>
        <link rel="self" href="/request/9991"/>
        <entry><id>tag:source.example,2005:InfoRequestEvent/3001</id>
          <title>Request event 3001, not request 3001</title>
          <link href="../../request_event/3001" rel="alternate"/>
          <link href="/request/9992" rel="self"/>
          <link href="/request/9993" rel="enclosure"/>
          <link href="/request/9994" rel="related"/>
        </entry>
        <entry><id>tag:source.example,2005:InfoRequestEvent/3002</id>
          <link rel="alternate" href="/request/synthetic_statistics#incoming-42"/>
          <link href="/request/synthetic_statistics#incoming-43"/>
          <summary>Metadata references /request/9995 without a link.</summary>
          <link rel="alternate" href="request/9996"/>
          <link rel="alternate" href="/request/321"/>
          <link rel="alternate" href="/api/v2/request/4001.json"/>
        </entry>
      </feed>"#;
    let expected = vec![
        request_reference("/api/v2/request/4001.json", Some(4001)),
        request_reference("/request/321", None),
        request_reference("/request/synthetic_statistics", None),
        event_reference("/request_event/3001"),
    ];
    assert_eq!(
        parse_fixture_references(feed, 4).unwrap(),
        (expected.clone(), false)
    );
    assert_eq!(
        parse_fixture_references(feed, 1).unwrap(),
        (vec![expected[0].clone()], true)
    );
}

#[test]
fn numeric_public_url_titles_are_unresolved_and_never_request_ids() {
    let feed = r#"<feed>
      <entry><link href="/request/123#incoming-9"/></entry>
      <entry><link href="/request/0"/></entry>
      <entry><link href="/request/9223372036854775808"/></entry>
      <entry><link href="/request_event/456"/></entry>
      <entry><id>tag:source.example,2005:InfoRequestEvent/789</id></entry>
      <entry><link href="/requests/321"/></entry>
      </feed>"#;
    let (references, truncated) = parse_fixture_references(feed, 10).unwrap();
    assert_eq!(
        references,
        vec![
            request_reference("/request/0", None),
            request_reference("/request/123", None),
            request_reference("/request/9223372036854775808", None),
            event_reference("/request_event/456"),
        ]
    );
    assert!(references
        .iter()
        .all(|reference| reference.request_id.is_none()));
    assert!(!truncated);
}

#[test]
fn rss_guid_and_atom_ids_require_canonical_reference_urls() {
    let rss = r#"<rss><channel>
      <link>https://source.example/request/9991</link>
      <item><guid>3001</guid><title>https://source.example/request/9992</title>
        <description>https://source.example/request/9993</description>
        <guid>https://source.example/request_event/31#event</guid>
        <link>https://source.example/request/synthetic_slug#incoming-7</link>
        <link href="https://source.example/request/9994"/>
      </item></channel></rss>"#;
    assert_eq!(
        parse_fixture_references(rss, 10).unwrap(),
        (
            vec![
                request_reference("/request/synthetic_slug", None),
                event_reference("/request_event/31")
            ],
            false
        )
    );
    let atom = r#"<feed><id>https://source.example/api/v2/request/9991.json</id>
      <entry><id>https://source.example/api/v2/request/71.json#response</id></entry>
      <entry><id>tag:source.example,2005:InfoRequestEvent/request/9992</id></entry>
      <entry><id>3001</id><link>/request/9993</link></entry></feed>"#;
    assert_eq!(
        parse_fixture_references(atom, 10).unwrap(),
        (
            vec![request_reference("/api/v2/request/71.json", Some(71))],
            false
        )
    );
}

#[test]
fn feed_reference_limit_is_deterministic_and_shared_across_route_kinds() {
    let feed = r#"<rss><channel><item>
      <link>/request/9</link><link>/api/v2/request/2.json</link><guid>/request_event/3</guid>
      <link>/request/synthetic_slug</link><link>/request/9#duplicate</link><link>/api/v2/request/1.json</link>
      </item></channel></rss>"#;
    assert_eq!(
        parse_fixture_references(feed, 2).unwrap(),
        (
            vec![
                request_reference("/api/v2/request/1.json", Some(1)),
                request_reference("/api/v2/request/2.json", Some(2)),
            ],
            true
        )
    );
    assert_eq!(
        parse_fixture_references(feed, 5).unwrap(),
        (
            vec![
                request_reference("/api/v2/request/1.json", Some(1)),
                request_reference("/api/v2/request/2.json", Some(2)),
                request_reference("/request/9", None),
                request_reference("/request/synthetic_slug", None),
                event_reference("/request_event/3"),
            ],
            false
        )
    );
}

#[tokio::test]
async fn plan_request_ids_are_only_the_projection_of_retained_references() {
    let server = MockServer::start().await;
    let client = client(&server).await;
    let feed = r#"<feed><entry><link href="/api/v2/request/7.json#first"/>
      <link href="/api/v2/request/7.json#second"/><id>/api/v2/request/9.json</id></entry>
      <entry><link href="/request/321"/></entry>
      <entry><id>tag:example,2005:InfoRequestEvent/3</id><link href="/request_event/3"/></entry>
      </feed>"#;
    Mock::given(method("GET"))
        .and(path("/feed/body/health"))
        .respond_with(ResponseTemplate::new(200).set_body_string(feed))
        .expect(1)
        .mount(&server)
        .await;
    let plan = client
        .plan_authority_feed("health", 1, &limits())
        .await
        .unwrap()
        .value;
    assert_eq!(plan.request_ids, vec![7]);
    assert_eq!(plan.references.len(), 1);
    assert_eq!(plan.references[0].request_id, Some(7));
    assert!(plan.truncated);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[test]
fn feed_relative_links_resolve_from_document_and_stay_within_base_path() {
    let feed = r#"<feed><entry><link href="../../request/synthetic_slug#incoming-7"/>
      <link href="../../request_event/8"/><link href="/portal/api/v2/request/9.json"/>
      <link href="../../../request/9991"/><link href="/request/9992"/>
      <link href="request/9993"/><link href="/portal-other/request/9994"/>
      </entry></feed>"#;
    for configured_base in [
        "https://source.example/portal",
        "https://source.example/portal/",
    ] {
        let base = Url::parse(configured_base).unwrap();
        let document = Url::parse("https://source.example/portal/feed/body/health").unwrap();
        assert_eq!(
            feed_references(feed, &base, &document, 10).unwrap(),
            (
                vec![
                    request_reference("/portal/api/v2/request/9.json", Some(9)),
                    request_reference("/portal/request/synthetic_slug", None),
                    event_reference("/portal/request_event/8"),
                ],
                false
            )
        );
    }
}

#[test]
fn feed_references_reject_extra_segments_and_unsafe_encodings() {
    for link in [
        "/request/1001/synthetic-first",
        "/request/slug/",
        "/request_event/1/extra",
        "/requests/123",
        "/requests/slug",
        "/requests/-1",
        "/request_event/0",
        "/request_event/a_slug",
        "/api/v2/request/0.json",
        "/api/v2/request/-1.json",
        "/api/v2/request/+1.json",
        "/api/v2/request/7.json/extra",
        "/api/v2/request/7",
        "/api/v2/request/slug.json",
        "/api/v2/request/9223372036854775808.json",
        "/api/v2/request/00000000000000000001.json",
        "/request_event/9223372036854775808",
        "/request/unsafe%2fslug",
        "/request/%2e%2e/request/4",
        "/request/has%20space",
        "/request/has space",
        "/request/has.dot",
        "/request/é",
        "/request/has\\separator",
        "/request/4?",
        "/request/4?secret=synthetic-private",
        "https://foreign.example/request/4",
        "https://user:synthetic-private@source.example/request/4",
    ] {
        let feed = format!("<feed><entry><link href=\"{link}\"/></entry></feed>");
        assert_eq!(
            parse_fixture_references(&feed, 10).unwrap(),
            (vec![], false),
            "{link}"
        );
    }
    let oversized = "a".repeat(129);
    let feed = format!("<feed><entry><link href=\"/request/{oversized}\"/></entry></feed>");
    assert_eq!(
        parse_fixture_references(&feed, 10).unwrap(),
        (vec![], false)
    );
}

#[test]
fn empty_and_all_foreign_feeds_have_an_empty_untruncated_plan() {
    for feed in [
        "<feed/>",
        "<rss><channel/></rss>",
        "<feed><entry><link href=\"https://foreign.example/request/8\"/></entry></feed>",
    ] {
        assert_eq!(parse_fixture_references(feed, 1).unwrap(), (vec![], false));
    }
}

#[test]
fn feed_parser_rejects_malformed_documents_entities_and_base_overrides() {
    for feed in [
        "<feed><entry></feed>",
        "<feed/><feed/>",
        "<feed>&unknown;</feed>",
        "<!DOCTYPE feed [<!ENTITY ext SYSTEM 'https://foreign.example/'>]><feed>&ext;</feed>",
        "<feed xml:base=\"https://foreign.example/\"><entry><link href=\"/request/a\"/></entry></feed>",
        "<feed><entry><link href=\"/request/a\" href=\"/request/b\"/></entry></feed>",
        "<feed><entry><id>/request/<x>a</x></id></entry></feed>",
        "<feed>&#0;</feed>",
    ] {
        assert!(parse_fixture_references(feed, 10).is_err());
    }
}

#[tokio::test]
async fn malformed_and_entity_feeds_return_private_typed_errors() {
    for feed in [
        "synthetic-private", "<html>synthetic-private</html>", "<rss><channel>",
        "<rss><channel></rss>", "<rss></rss><feed></feed>", "<rss/>synthetic-private",
        "<rss><item>&synthetic-private;</item></rss>",
        "<!DOCTYPE rss [<!ENTITY ext SYSTEM 'https://synthetic-private.invalid/'>]><rss><item>&ext;</item></rss>",
        "<feed><entry><link href=\"/request/1\" href=\"/request/2\"/></entry></feed>",
        "<feed xml:base=\"https://synthetic-private.invalid/\"><entry><link href=\"/request/1\"/></entry></feed>",
        "<feed><entry><id>/request/<x>2</x>1</id></entry></feed>",
        "<feed>&#0;</feed>",
        "<feed>\0synthetic-private</feed>",
    ] {
        let server = MockServer::start().await;
        let client = client(&server).await;
        Mock::given(method("GET")).and(path("/feed/body/health"))
            .respond_with(ResponseTemplate::new(200).set_body_string(feed))
            .expect(1).mount(&server).await;
        let error = client.plan_authority_feed("health", 10, &limits()).await.unwrap_err();
        assert_safe_kind(&error, SourceFailureKind::MalformedPayload);
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[test]
fn feed_parser_accepts_cdata_and_predefined_references_and_bounds_depth() {
    for feed in [
        "<feed><entry><id><![CDATA[https://source.example/request/7]]></id></entry></feed>",
        "<rss><channel><item><link>https://source.example/request/&#55;</link><title>&amp;</title></item></channel></rss>",
    ] { assert_eq!(parse_fixture_references(feed, 10).unwrap(), (vec![request_reference("/request/7", None)],false)); }
    for feed in [
        format!("<rss>{}{}</rss>", "<x>".repeat(65), "</x>".repeat(65)),
        "<?xml version=\"1.1\"?><rss/>".into(),
        "<?xml version=\"1.0\" encoding=\"iso-8859-1\"?><rss/>".into(),
        "<rss><?xml version=\"1.0\"?></rss>".into(),
        "<![CDATA[synthetic-private]]><rss/>".into(),
        "<rss/><![CDATA[synthetic-private]]>".into(),
    ] {
        assert!(parse_fixture_references(&feed, 10).is_err());
    }
}

#[tokio::test]
async fn invalid_read_limits_and_feed_items_fail_before_io() {
    let server = MockServer::start().await;
    let client = client(&server).await;
    for limit in [
        ReadLimits {
            max_bytes: 0,
            ..limits()
        },
        ReadLimits {
            max_duration: Duration::ZERO,
            ..limits()
        },
    ] {
        assert_safe_kind(
            &client.get_api_version_read(&limit).await.unwrap_err(),
            SourceFailureKind::Validation,
        );
    }
    for id in [0, -1] {
        assert_safe_kind(
            &client.fetch_request_read(id, &limits()).await.unwrap_err(),
            SourceFailureKind::Validation,
        );
    }
    for count in [0, 201] {
        assert_safe_kind(
            &client
                .plan_authority_feed("health", count, &limits())
                .await
                .unwrap_err(),
            SourceFailureKind::Validation,
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn revalidated_response_retains_original_retrieval_time_and_counts_no_cached_bytes() {
    let server = MockServer::start().await;
    let client = client(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/v2/version.json"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"v1\"")
                .set_body_json(json!({"version":"fixture"})),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v2/version.json"))
        .and(header("If-None-Match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304))
        .expect(1)
        .mount(&server)
        .await;
    let original = client.get_api_version_read(&limits()).await.unwrap();
    let reused = client
        .clone()
        .get_api_version_read(&limits())
        .await
        .unwrap();
    assert_eq!(reused.value, original.value);
    assert_eq!(
        reused.provenance.retrieved_at,
        original.provenance.retrieved_at
    );
    assert_eq!(reused.provenance.cache_status, CacheStatus::Revalidated);
    assert_eq!(reused.provenance.source_status, 304);
    assert_eq!(reused.provenance.response_bytes, 0);
    assert_eq!(
        client
            .middleware
            .lock()
            .await
            .guardrails
            .snapshot()
            .response_bytes,
        original.provenance.response_bytes
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn cache_revalidation_requires_the_sent_validator_and_per_call_body_limit() {
    let server = MockServer::start().await;
    let client = client(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/v2/version.json"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"v1\"")
                .set_body_json(json!({"version":"fixture"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    client.get_api_version_read(&limits()).await.unwrap();
    let mock = Mock::given(method("GET"))
        .and(path("/api/v2/version.json"))
        .respond_with(ResponseTemplate::new(304))
        .expect(1)
        .mount_as_scoped(&server)
        .await;
    let error = client
        .get_api_version_read(&ReadLimits {
            max_bytes: 1,
            ..limits()
        })
        .await
        .unwrap_err();
    assert_safe_kind(&error, SourceFailureKind::Guardrail);
    drop(mock);
    let mock = Mock::given(method("GET"))
        .and(path("/api/v2/version.json"))
        .respond_with(
            ResponseTemplate::new(304).insert_header("ETag", "\"synthetic-private-changed\""),
        )
        .expect(1)
        .mount_as_scoped(&server)
        .await;
    assert_safe_kind(
        &client.get_api_version_read(&limits()).await.unwrap_err(),
        SourceFailureKind::Cache,
    );
    drop(mock);
    client.validator_cache.lock().await.clear();
    Mock::given(method("GET"))
        .and(path("/api/v2/version.json"))
        .respond_with(ResponseTemplate::new(304))
        .expect(1)
        .mount(&server)
        .await;
    assert_safe_kind(
        &client.get_api_version_read(&limits()).await.unwrap_err(),
        SourceFailureKind::Cache,
    );
}

#[tokio::test]
async fn request_and_byte_guardrails_are_shared_across_client_clones() {
    let server = MockServer::start().await;
    let client = client(&server).await;
    client
        .middleware
        .lock()
        .await
        .guardrails
        .config
        .max_requests = 1;
    Mock::given(method("GET"))
        .and(path("/api/v2/version.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string("\"fixture\""))
        .expect(1)
        .mount(&server)
        .await;
    client.get_api_version_read(&limits()).await.unwrap();
    assert_safe_kind(
        &client
            .clone()
            .get_api_version_read(&limits())
            .await
            .unwrap_err(),
        SourceFailureKind::Guardrail,
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    let server = MockServer::start().await;
    let client = self::client(&server).await;
    client
        .middleware
        .lock()
        .await
        .guardrails
        .config
        .max_response_bytes = 9;
    Mock::given(method("GET"))
        .and(path("/api/v2/version.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string("\"fixture\""))
        .expect(1)
        .mount(&server)
        .await;
    client.get_api_version_read(&limits()).await.unwrap();
    assert_safe_kind(
        &client
            .clone()
            .get_api_version_read(&limits())
            .await
            .unwrap_err(),
        SourceFailureKind::Guardrail,
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn per_operation_timeout_bounds_stalled_source_and_pacing_wait() {
    let server = MockServer::start().await;
    let client = client(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/v2/version.json"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("\"fixture\"")
                .set_delay(Duration::from_secs(1)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let short = ReadLimits {
        max_duration: Duration::from_millis(40),
        ..limits()
    };
    assert_safe_kind(
        &client.get_api_version_read(&short).await.unwrap_err(),
        SourceFailureKind::Timeout,
    );
    client.middleware.lock().await.pacing.current_delay = Duration::from_secs(1);
    assert_safe_kind(
        &client.get_api_version_read(&short).await.unwrap_err(),
        SourceFailureKind::Timeout,
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert_eq!(
        client
            .middleware
            .lock()
            .await
            .guardrails
            .snapshot()
            .requests,
        2
    );
}

#[tokio::test]
async fn redirects_never_escape_one_request_admission() {
    let server = MockServer::start().await;
    let client = client(&server).await;
    client
        .middleware
        .lock()
        .await
        .guardrails
        .config
        .max_requests = 1;
    Mock::given(method("GET"))
        .and(path("/api/v2/version.json"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("Location", "/synthetic-private-redirect"),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/synthetic-private-redirect"))
        .respond_with(ResponseTemplate::new(200).set_body_string("\"must-not-fetch\""))
        .expect(0)
        .mount(&server)
        .await;
    assert_safe_kind(
        &client.get_api_version_read(&limits()).await.unwrap_err(),
        SourceFailureKind::HttpStatus(302),
    );
    assert_safe_kind(
        &client.get_api_version_read(&limits()).await.unwrap_err(),
        SourceFailureKind::Guardrail,
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn stream_guard_stops_after_first_over_budget_chunk() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let client = SyncClient::new("https://source.example/").unwrap();
    client.middleware.lock().await.pacing.current_delay = Duration::ZERO;
    let polled = Arc::new(AtomicUsize::new(0));
    let observed = polled.clone();
    let stream = futures_util::stream::unfold(0, move |index| {
        let observed = observed.clone();
        async move {
            if index >= 3 {
                return None;
            }
            observed.fetch_add(1, Ordering::SeqCst);
            Some((Ok::<_, std::io::Error>(b"12345678".to_vec()), index + 1))
        }
    });
    let request = client
        .http
        .get("https://source.example/api/v2/version.json");
    let error = client
        .send_guarded_with_limits_using(
            request,
            Some(10),
            false,
            "alaveteli_api",
            |request| async move {
                let url = request.url().clone();
                let response = http::Response::builder()
                    .status(200)
                    .body(reqwest::Body::wrap_stream(stream))
                    .unwrap();
                Ok((response.into(), url))
            },
        )
        .await
        .err()
        .unwrap();
    assert_eq!(
        source_failure_kind(&error),
        Some(SourceFailureKind::Guardrail)
    );
    assert_eq!(polled.load(Ordering::SeqCst), 2);
    assert_eq!(
        client
            .middleware
            .lock()
            .await
            .guardrails
            .snapshot()
            .response_bytes,
        16
    );
}

#[tokio::test]
async fn in_flight_304_keeps_the_representation_used_for_its_validator() {
    let client = SyncClient::new("https://source.example/").unwrap();
    client.middleware.lock().await.pacing.current_delay = Duration::ZERO;
    let url = "https://source.example/api/v2/version.json";
    client.validator_cache.lock().await.insert(
        url.into(),
        ValidatorCacheEntry {
            etag: Some("\"v1\"".into()),
            last_modified: None,
            body: b"\"one\"".to_vec(),
            retrieved_at: "2026-01-01T00:00:00.000Z".into(),
        },
    );
    let cache = client.validator_cache.clone();
    let response = client
        .send_guarded_with_limits_using(
            client.http.get(url),
            Some(1024),
            true,
            "alaveteli_api",
            |request| async move {
                assert_eq!(request.headers()[header::IF_NONE_MATCH], "\"v1\"");
                cache.lock().await.insert(
                    url.into(),
                    ValidatorCacheEntry {
                        etag: Some("\"v2\"".into()),
                        last_modified: None,
                        body: b"\"two\"".to_vec(),
                        retrieved_at: "2026-01-02T00:00:00.000Z".into(),
                    },
                );
                let response = http::Response::builder()
                    .status(304)
                    .body(reqwest::Body::from(""))
                    .unwrap();
                Ok((response.into(), request.url().clone()))
            },
        )
        .await
        .unwrap();
    assert_eq!(response.body, b"\"one\"");
    assert_eq!(response.retrieved_at, "2026-01-01T00:00:00.000Z");
}

#[tokio::test]
async fn failed_http_bodies_consume_shared_source_byte_allowance() {
    let server = MockServer::start().await;
    let client = client(&server).await;
    client
        .middleware
        .lock()
        .await
        .guardrails
        .config
        .max_response_bytes = 9;
    Mock::given(method("GET"))
        .and(path("/api/v2/version.json"))
        .respond_with(ResponseTemplate::new(500).set_body_string("123456789"))
        .expect(1)
        .mount(&server)
        .await;
    assert_safe_kind(
        &client.get_api_version_read(&limits()).await.unwrap_err(),
        SourceFailureKind::HttpStatus(500),
    );
    assert_safe_kind(
        &client.get_api_version_read(&limits()).await.unwrap_err(),
        SourceFailureKind::Guardrail,
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
