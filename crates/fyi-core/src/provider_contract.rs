//! Provider-neutral read-only contracts for non-Alaveteli FOI systems.
//!
//! The existing [`crate::jurisdiction::FoiProvider`] is intentionally retained
//! for Alaveteli compatibility. This contract prevents future providers from
//! being forced into Alaveteli-specific request and response types.

use crate::jurisdiction::Instance;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReadCapabilities {
    pub request: bool,
    pub search: bool,
    pub authorities: bool,
    pub correspondence: bool,
    pub attachments: bool,
    pub health: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderAuthority {
    pub provider_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderMessage {
    pub direction: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<String>,
    #[serde(default)]
    pub attachment_urls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub provider_id: String,
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority: Option<ProviderAuthority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub messages: Vec<ProviderMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_source_hash: Option<String>,
}

pub trait ReadOnlyFoiProvider: Send + Sync {
    fn instance(&self) -> &Instance;
    fn capabilities(&self) -> ReadCapabilities;

    fn get_request(
        &self,
        provider_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ProviderRequest>> + Send + '_>>;

    fn search_requests(
        &self,
        query: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ProviderRequest>>> + Send + '_>>;

    fn list_authorities(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ProviderAuthority>>> + Send + '_>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_capabilities_default_to_fail_closed() {
        assert_eq!(
            ReadCapabilities::default(),
            ReadCapabilities {
                request: false,
                search: false,
                authorities: false,
                correspondence: false,
                attachments: false,
                health: false,
            }
        );
    }

    #[test]
    fn provider_request_preserves_provenance_and_messages() {
        let request = ProviderRequest {
            provider_id: "muckrock-123".into(),
            title: "Records request".into(),
            body: "Please provide the records".into(),
            authority: None,
            status: Some("processed".into()),
            source_url: Some("https://example.test/requests/123".into()),
            created_at: None,
            updated_at: None,
            tags: vec!["public".into()],
            messages: vec![ProviderMessage {
                direction: "response".into(),
                body: "Attached".into(),
                sent_at: None,
                attachment_urls: vec!["https://example.test/file.pdf".into()],
            }],
            raw_source_hash: Some("sha256:abc".into()),
        };

        let encoded = serde_json::to_string(&request).unwrap();
        let decoded: ProviderRequest = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, request);
        assert_eq!(decoded.raw_source_hash.as_deref(), Some("sha256:abc"));
    }
}
