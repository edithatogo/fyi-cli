//! Read-only providers for non-Alaveteli FOI platforms.

pub mod fragdenstaat;

use crate::jurisdiction::Instance;
use crate::provider_contract::{
    ProviderAuthority, ProviderRequest, ReadCapabilities, ReadOnlyFoiProvider,
};
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::future::Future;
use std::pin::Pin;

/// Read-only MuckRock API v2 adapter.
///
/// Authentication and all POST/PATCH/DELETE operations are deliberately absent.
/// The provider parses only public request, search, and agency responses.
pub struct MuckRockProvider {
    instance: Instance,
    client: Client,
    api_base_url: String,
}

impl MuckRockProvider {
    pub fn new(instance: Instance, client: Client) -> Result<Self> {
        let base = instance.base_url.trim_end_matches('/').to_owned();
        if !base.starts_with("https://") {
            return Err(anyhow!("MuckRock provider requires an HTTPS base URL"));
        }
        Ok(Self {
            instance,
            client,
            api_base_url: format!("{base}/api_v2"),
        })
    }

    #[cfg(test)]
    fn with_api_base(instance: Instance, client: Client, api_base_url: String) -> Self {
        Self {
            instance,
            client,
            api_base_url,
        }
    }

    async fn json_get(&self, path: &str, query: &[(&str, &str)]) -> Result<Value> {
        let url = format!(
            "{}/{}",
            self.api_base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let response = self
            .client
            .get(url)
            .query(query)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .context("MuckRock request failed")?;
        let status = response.status();
        let body = response
            .text()
            .await
            .context("MuckRock response read failed")?;
        if !status.is_success() {
            return Err(anyhow!("MuckRock returned HTTP {status}"));
        }
        serde_json::from_str(&body).context("MuckRock returned invalid JSON")
    }
}

impl ReadOnlyFoiProvider for MuckRockProvider {
    fn instance(&self) -> &Instance {
        &self.instance
    }

    fn capabilities(&self) -> ReadCapabilities {
        ReadCapabilities {
            request: true,
            search: true,
            authorities: true,
            ..Default::default()
        }
    }

    fn get_request(
        &self,
        provider_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ProviderRequest>> + Send + '_>> {
        let provider_id = provider_id.to_owned();
        Box::pin(async move {
            provider_id
                .parse::<u64>()
                .context("MuckRock request id must be numeric")?;
            let raw = self
                .json_get(&format!("requests/{provider_id}/"), &[("format", "json")])
                .await?;
            normalize_request(&raw)
        })
    }

    fn search_requests(
        &self,
        query: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ProviderRequest>>> + Send + '_>> {
        let query = query.to_owned();
        Box::pin(async move {
            let raw = self
                .json_get(
                    "requests/",
                    &[("search", query.as_str()), ("format", "json")],
                )
                .await?;
            let results = raw
                .get("results")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            results.iter().map(normalize_request).collect()
        })
    }

    fn list_authorities(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ProviderAuthority>>> + Send + '_>> {
        Box::pin(async move {
            let raw = self.json_get("agencies/", &[("format", "json")]).await?;
            let results = raw
                .get("results")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            results.iter().map(normalize_authority).collect()
        })
    }
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|field| {
            field
                .as_str()
                .map(str::to_owned)
                .or_else(|| field.as_u64().map(|number| number.to_string()))
                .or_else(|| field.as_i64().map(|number| number.to_string()))
        })
    })
}

fn required_string(value: &Value, keys: &[&str], label: &str) -> Result<String> {
    string_field(value, keys)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| anyhow!("MuckRock response missing {label}"))
}

fn normalize_request(raw: &Value) -> Result<ProviderRequest> {
    let id = required_string(raw, &["id", "pk"], "request id")?;
    let title = required_string(raw, &["title", "subject"], "request title")?;
    let body = required_string(
        raw,
        &["requested_docs", "body", "description"],
        "request body",
    )?;
    let source_url = string_field(raw, &["url", "absolute_url"]);
    let raw_bytes = serde_json::to_vec(raw).context("MuckRock response serialization failed")?;
    let hash = format!("sha256:{}", hex_digest(&raw_bytes));
    Ok(ProviderRequest {
        provider_id: format!("muckrock-{id}"),
        title,
        body,
        authority: None,
        status: string_field(raw, &["status", "status_display"]),
        source_url,
        created_at: parse_timestamp(raw, &["date_created", "created_at"]),
        updated_at: parse_timestamp(raw, &["date_updated", "updated_at"]),
        tags: raw
            .get("tags")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        messages: Vec::new(),
        raw_source_hash: Some(hash),
    })
}

fn normalize_authority(raw: &Value) -> Result<ProviderAuthority> {
    let id = required_string(raw, &["id", "pk"], "agency id")?;
    let name = required_string(raw, &["name", "name_display"], "agency name")?;
    Ok(ProviderAuthority {
        provider_id: format!("muckrock-agency-{id}"),
        name,
        url: string_field(raw, &["url", "absolute_url"]),
    })
}

fn parse_timestamp(raw: &Value, keys: &[&str]) -> Option<chrono::DateTime<chrono::Utc>> {
    string_field(raw, keys)
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
        .map(|value| value.with_timezone(&chrono::Utc))
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jurisdiction::{Capabilities, FoiLaw, InstanceStatus};
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    fn instance(base_url: String) -> Instance {
        Instance {
            id: "us-muckrock".into(),
            base_url,
            country: "US".into(),
            locale: "en-US".into(),
            foi_law: FoiLaw {
                law_name: "FOIA".into(),
                citation: None,
                request_term: "public records request".into(),
                statutory_deadline_days: None,
                appeal_body: None,
            },
            capabilities: Capabilities {
                read: true,
                search: true,
                ..Default::default()
            },
            status: InstanceStatus::Community,
        }
    }

    #[tokio::test]
    async fn reads_and_normalizes_public_request_without_write_capabilities() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/api_v2/requests/42/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id":42,"title":"Records","requested_docs":"Provide records","status":"processed","url":"https://www.muckrock.com/foi/42/"}))).mount(&server).await;
        let provider = MuckRockProvider::with_api_base(
            instance(server.uri()),
            Client::new(),
            format!("{}/api_v2", server.uri()),
        );
        let request = provider.get_request("42").await.unwrap();
        assert_eq!(request.provider_id, "muckrock-42");
        assert!(request.raw_source_hash.unwrap().starts_with("sha256:"));
        assert!(!provider.capabilities().correspondence);
    }

    #[tokio::test]
    async fn rejects_missing_required_fields() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api_v2/requests/42/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id":42})))
            .mount(&server)
            .await;
        let provider = MuckRockProvider::with_api_base(
            instance(server.uri()),
            Client::new(),
            format!("{}/api_v2", server.uri()),
        );
        assert!(provider.get_request("42").await.is_err());
        assert!(provider.get_request("../secrets").await.is_err());
    }

    #[tokio::test]
    async fn search_maps_alias_fields_tags_and_timestamps() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api_v2/requests/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [{
                    "pk": 7,
                    "subject": "Water quality",
                    "description": "Release inspections",
                    "absolute_url": "https://example.test/records/7",
                    "status_display": "Completed",
                    "created_at": "2026-10-21T10:30:00+10:00",
                    "updated_at": "2026-10-22T00:30:00Z",
                    "tags": ["water", 23, "quality"]
                }]
            })))
            .mount(&server)
            .await;
        let provider = MuckRockProvider::with_api_base(
            instance(server.uri()),
            Client::new(),
            format!("{}/api_v2", server.uri()),
        );
        let records = provider.search_requests("water").await.unwrap();
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.provider_id, "muckrock-7");
        assert_eq!(record.title, "Water quality");
        assert_eq!(record.body, "Release inspections");
        assert_eq!(record.status.as_deref(), Some("Completed"));
        assert_eq!(record.tags, vec!["water", "quality"]);
        assert_eq!(
            record.created_at.unwrap().to_rfc3339(),
            "2026-10-21T00:30:00+00:00"
        );
        assert_eq!(
            record.updated_at.unwrap().to_rfc3339(),
            "2026-10-22T00:30:00+00:00"
        );
        assert_eq!(
            record.source_url.as_deref(),
            Some("https://example.test/records/7")
        );
    }

    #[tokio::test]
    async fn authorities_map_aliases_and_fail_closed_on_missing_name() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api_v2/agencies/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [{"pk": 13, "name_display": "Water agency", "absolute_url": "https://example.test/a/13"}]
            })))
            .mount(&server)
            .await;
        let provider = MuckRockProvider::with_api_base(
            instance(server.uri()),
            Client::new(),
            format!("{}/api_v2", server.uri()),
        );
        let authorities = provider.list_authorities().await.unwrap();
        assert_eq!(authorities.len(), 1);
        assert_eq!(authorities[0].provider_id, "muckrock-agency-13");
        assert_eq!(authorities[0].name, "Water agency");
        assert_eq!(
            authorities[0].url.as_deref(),
            Some("https://example.test/a/13")
        );
        assert!(normalize_authority(&serde_json::json!({"pk": 13})).is_err());
        assert!(normalize_authority(&serde_json::json!({"pk": -3, "name": "Signed"})).is_ok());
    }

    #[tokio::test]
    async fn provider_surfaces_http_and_json_errors_without_leaking_bodies() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api_v2/requests/1/"))
            .respond_with(ResponseTemplate::new(403).set_body_string("SECRET_RESPONSE_BODY"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api_v2/requests/2/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{invalid"))
            .mount(&server)
            .await;
        let provider = MuckRockProvider::with_api_base(
            instance(server.uri()),
            Client::new(),
            format!("{}/api_v2", server.uri()),
        );
        let denied = provider.get_request("1").await.unwrap_err().to_string();
        assert!(denied.contains("403"));
        assert!(!denied.contains("SECRET_RESPONSE_BODY"));
        assert!(provider
            .get_request("2")
            .await
            .unwrap_err()
            .to_string()
            .contains("invalid JSON"));
    }

    #[test]
    fn requires_https_and_preserves_capability_boundary() {
        let rejected = MuckRockProvider::new(instance("http://example.test".into()), Client::new());
        assert!(rejected.is_err());
        let provider =
            MuckRockProvider::new(instance("https://example.test/".into()), Client::new()).unwrap();
        assert_eq!(provider.instance().id, "us-muckrock");
        let caps = provider.capabilities();
        assert!(caps.request && caps.search && caps.authorities);
        assert!(!caps.correspondence && !caps.attachments && !caps.health);
        assert_eq!(provider.api_base_url, "https://example.test/api_v2");
    }

    #[test]
    fn normalization_is_order_invariant_but_detects_content_changes() {
        // Metamorphic contract: JSON map ordering must not change the receipt hash.
        let left: Value = serde_json::from_str(
            r#"{"id":4,"title":"A","body":"B","tags":["c"]}"#,
        )
        .unwrap();
        let reordered: Value = serde_json::from_str(
            r#"{"tags":["c"],"body":"B","title":"A","id":4}"#,
        )
        .unwrap();
        let changed: Value = serde_json::from_str(
            r#"{"id":4,"title":"A","body":"Different","tags":["c"]}"#,
        )
        .unwrap();
        let a = normalize_request(&left).unwrap();
        let b = normalize_request(&reordered).unwrap();
        let c = normalize_request(&changed).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.raw_source_hash, b.raw_source_hash);
        assert_ne!(a.raw_source_hash, c.raw_source_hash);
        assert_ne!(a.body, c.body);
    }

    #[test]
    fn required_fields_reject_missing_empty_or_nonnumeric_objects() {
        for bad in [
            serde_json::json!({"title": "A", "body": "B"}),
            serde_json::json!({"id": 1, "title": "", "body": "B"}),
            serde_json::json!({"id": 1, "title": "A", "body": "  "}),
            serde_json::json!({"id": {"not": "scalar"}, "title": "A", "body": "B"}),
        ] {
            assert!(normalize_request(&bad).is_err());
        }
    }
}
