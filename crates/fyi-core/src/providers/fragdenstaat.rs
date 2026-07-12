//! Read-only FragDenStaat API v1 provider.

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

/// Public request, public-body, and search reads only. OAuth and all writes are excluded.
pub struct FragDenStaatProvider {
    instance: Instance,
    client: Client,
    api_base_url: String,
}

impl FragDenStaatProvider {
    pub fn new(instance: Instance, client: Client) -> Result<Self> {
        let base = instance.base_url.trim_end_matches('/').to_owned();
        if !base.starts_with("https://") {
            return Err(anyhow!("FragDenStaat provider requires an HTTPS base URL"));
        }
        Ok(Self {
            instance,
            client,
            api_base_url: format!("{base}/api/v1"),
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
            .context("FragDenStaat request failed")?;
        let status = response.status();
        let body = response
            .text()
            .await
            .context("FragDenStaat response read failed")?;
        if !status.is_success() {
            return Err(anyhow!("FragDenStaat returned HTTP {status}"));
        }
        serde_json::from_str(&body).context("FragDenStaat returned invalid JSON")
    }
}

impl ReadOnlyFoiProvider for FragDenStaatProvider {
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
            validate_path_segment(&provider_id)?;
            let raw = self
                .json_get(&format!("request/{provider_id}/"), &[])
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
                .json_get("request/", &[("search", query.as_str())])
                .await?;
            raw.get("results")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(normalize_request)
                .collect()
        })
    }

    fn list_authorities(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ProviderAuthority>>> + Send + '_>> {
        Box::pin(async move {
            let raw = self.json_get("publicbody/", &[]).await?;
            raw.get("results")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(normalize_authority)
                .collect()
        })
    }
}

fn validate_path_segment(value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
    {
        return Err(anyhow!(
            "FragDenStaat request id contains unsafe path characters"
        ));
    }
    Ok(())
}

fn field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|item| {
            item.as_str()
                .map(str::to_owned)
                .or_else(|| item.as_u64().map(|number| number.to_string()))
        })
    })
}

fn required(value: &Value, keys: &[&str], label: &str) -> Result<String> {
    field(value, keys)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| anyhow!("FragDenStaat response missing {label}"))
}

fn normalize_request(raw: &Value) -> Result<ProviderRequest> {
    let id = required(raw, &["id", "pk", "slug"], "request id")?;
    let authority = raw
        .get("publicbody")
        .and_then(|value| normalize_authority(value).ok());
    let digest = Sha256::digest(
        serde_json::to_vec(raw).context("FragDenStaat response serialization failed")?,
    );
    Ok(ProviderRequest {
        provider_id: format!("fragdenstaat-{id}"),
        title: required(raw, &["title", "subject"], "request title")?,
        body: required(raw, &["summary", "description", "body"], "request body")?,
        authority,
        status: field(raw, &["status", "status_display"]),
        source_url: field(raw, &["url", "absolute_url"]),
        created_at: None,
        updated_at: None,
        tags: Vec::new(),
        messages: Vec::new(),
        raw_source_hash: Some(format!(
            "sha256:{}",
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )),
    })
}

fn normalize_authority(raw: &Value) -> Result<ProviderAuthority> {
    let id = required(raw, &["id", "pk", "slug"], "public body id")?;
    Ok(ProviderAuthority {
        provider_id: format!("fragdenstaat-publicbody-{id}"),
        name: required(raw, &["name", "title"], "public body name")?,
        url: field(raw, &["url", "absolute_url"]),
    })
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
            id: "de-fds".into(),
            base_url,
            country: "DE".into(),
            locale: "de-DE".into(),
            foi_law: FoiLaw {
                law_name: "IFG".into(),
                citation: None,
                request_term: "Anfrage".into(),
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
    async fn normalizes_public_request_and_authority_without_oauth() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/api/v1/request/abc-1/")).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id":"abc-1","subject":"Akten","summary":"Bitte Unterlagen","publicbody":{"id":7,"name":"Testbehörde"}}))).mount(&server).await;
        let provider = FragDenStaatProvider::with_api_base(
            instance(server.uri()),
            Client::new(),
            format!("{}/api/v1", server.uri()),
        );
        let request = provider.get_request("abc-1").await.unwrap();
        assert_eq!(request.provider_id, "fragdenstaat-abc-1");
        assert_eq!(request.authority.unwrap().name, "Testbehörde");
        assert!(!provider.capabilities().correspondence);
        assert!(provider.get_request("../secret").await.is_err());
    }
}
