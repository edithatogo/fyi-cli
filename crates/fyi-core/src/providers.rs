//! Read-only providers for non-Alaveteli FOI platforms.

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
        let base = instance.base_url.trim_end_matches('/');
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
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str).map(str::to_owned))
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
}
