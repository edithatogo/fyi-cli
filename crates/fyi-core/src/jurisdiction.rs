use crate::api::{
    AddCorrespondencePayload, AlaveteliRequest, CorrespondenceResponse, CreateRequestPayload,
    CreateRequestResponse, UpdateRequestStatePayload, UpdateRequestStateResponse,
};
use crate::sync::{SyncClient, SyncHealth};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoiLaw {
    #[serde(default)]
    pub law_name: String,
    #[serde(default)]
    pub citation: Option<String>,
    #[serde(default)]
    pub request_term: String,
    #[serde(default)]
    pub statutory_deadline_days: Option<i32>,
    #[serde(default)]
    pub appeal_body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Capabilities {
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub write: bool,
    #[serde(default)]
    pub attachments: bool,
    #[serde(default)]
    pub batch: bool,
    #[serde(default)]
    pub feeds: bool,
    #[serde(default)]
    pub search: bool,
    #[serde(default)]
    pub prefilled_url: bool,
    #[serde(default)]
    pub health: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstanceStatus {
    Supported,
    Experimental,
    Community,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub base_url: String,
    pub country: String,
    pub locale: String,
    pub foi_law: FoiLaw,
    #[serde(default)]
    pub capabilities: Capabilities,
    #[serde(default)]
    pub status: InstanceStatus,
}

#[derive(Debug, Clone, Default)]
pub struct InstanceRegistry {
    instances: BTreeMap<String, Instance>,
}

impl InstanceRegistry {
    pub fn embedded() -> Result<Self> {
        Self::from_toml(include_str!("../instances.toml"))
    }

    pub fn from_toml(input: &str) -> Result<Self> {
        #[derive(Deserialize)]
        struct Catalog {
            instances: Vec<Instance>,
        }

        let catalog: Catalog =
            toml::from_str(input).context("failed to parse embedded instance catalog")?;
        Ok(Self::from_instances(catalog.instances))
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read instance catalog from {}", path.display()))?;
        Self::from_toml(&contents)
    }

    pub fn from_instances(instances: impl IntoIterator<Item = Instance>) -> Self {
        let mut registry = Self::default();
        for instance in instances {
            registry.register(instance);
        }
        registry
    }

    pub fn register(&mut self, instance: Instance) {
        self.instances.insert(instance.id.clone(), instance);
    }

    pub fn get(&self, id: &str) -> Option<&Instance> {
        self.instances.get(id)
    }

    pub fn list(&self) -> Vec<&Instance> {
        self.instances.values().collect()
    }
}

pub trait FoiProvider: Send + Sync {
    fn instance(&self) -> &Instance;

    fn get_request(
        &self,
        request_id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<AlaveteliRequest>> + Send + '_>>;

    fn search_requests(
        &self,
        query: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AlaveteliRequest>>> + Send + '_>>;

    fn create_request(
        &self,
        payload: CreateRequestPayload,
    ) -> Pin<Box<dyn Future<Output = Result<CreateRequestResponse>> + Send + '_>>;

    fn add_correspondence(
        &self,
        request_id: i64,
        payload: AddCorrespondencePayload,
    ) -> Pin<Box<dyn Future<Output = Result<CorrespondenceResponse>> + Send + '_>>;

    fn update_request_state(
        &self,
        request_id: i64,
        payload: UpdateRequestStatePayload,
    ) -> Pin<Box<dyn Future<Output = Result<UpdateRequestStateResponse>> + Send + '_>>;

    fn build_prefilled_url(
        &self,
        authority_slug: &str,
        title: &str,
        body: &str,
        tags: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + '_>>;

    fn health(&self) -> Pin<Box<dyn Future<Output = Result<SyncHealth>> + Send + '_>>;
}

pub struct AlaveteliV2Provider {
    instance: Instance,
    client: SyncClient,
}

impl AlaveteliV2Provider {
    pub fn new(instance: Instance, client: SyncClient) -> Self {
        Self { instance, client }
    }
}

impl FoiProvider for AlaveteliV2Provider {
    fn instance(&self) -> &Instance {
        &self.instance
    }

    fn get_request(
        &self,
        request_id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<AlaveteliRequest>> + Send + '_>> {
        let client = self.client.clone();
        Box::pin(async move { client.fetch_request(request_id).await })
    }

    fn search_requests(
        &self,
        query: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AlaveteliRequest>>> + Send + '_>> {
        let client = self.client.clone();
        let query = query.to_string();
        Box::pin(async move { client.search_requests(&query).await })
    }

    fn create_request(
        &self,
        payload: CreateRequestPayload,
    ) -> Pin<Box<dyn Future<Output = Result<CreateRequestResponse>> + Send + '_>> {
        let client = self.client.clone();
        Box::pin(async move { client.create_request(&payload).await })
    }

    fn add_correspondence(
        &self,
        request_id: i64,
        payload: AddCorrespondencePayload,
    ) -> Pin<Box<dyn Future<Output = Result<CorrespondenceResponse>> + Send + '_>> {
        let client = self.client.clone();
        Box::pin(async move { client.add_correspondence(request_id, &payload).await })
    }

    fn update_request_state(
        &self,
        request_id: i64,
        payload: UpdateRequestStatePayload,
    ) -> Pin<Box<dyn Future<Output = Result<UpdateRequestStateResponse>> + Send + '_>> {
        let client = self.client.clone();
        Box::pin(async move { client.update_request_state(request_id, &payload).await })
    }

    fn build_prefilled_url(
        &self,
        authority_slug: &str,
        title: &str,
        body: &str,
        tags: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + '_>> {
        let client = self.client.clone();
        let authority_slug = authority_slug.to_string();
        let title = title.to_string();
        let body = body.to_string();
        let tags = tags.map(str::to_string);
        Box::pin(async move {
            client
                .build_prefilled_url(&authority_slug, &title, &body, tags.as_deref())
                .await
                .map(|url| url.to_string())
        })
    }

    fn health(&self) -> Pin<Box<dyn Future<Output = Result<SyncHealth>> + Send + '_>> {
        let client = self.client.clone();
        Box::pin(async move { Ok(client.health_check().await) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_contains_nz_and_au_instances() {
        let registry = InstanceRegistry::embedded().unwrap();
        assert!(registry.get("nz-fyi").is_some());
        assert!(registry.get("au-rtk").is_some());
    }

    #[test]
    fn registry_can_register_custom_instance() {
        let mut registry = InstanceRegistry::default();
        registry.register(Instance {
            id: "uk-wdtk".to_string(),
            base_url: "https://www.whatdotheyknow.com".to_string(),
            country: "GB".to_string(),
            locale: "en-GB".to_string(),
            foi_law: FoiLaw {
                law_name: "Freedom of Information Act".to_string(),
                citation: Some("FOIA 2000".to_string()),
                request_term: "request".to_string(),
                statutory_deadline_days: Some(20),
                appeal_body: Some("ICO".to_string()),
            },
            capabilities: Capabilities {
                read: true,
                write: true,
                attachments: true,
                batch: true,
                feeds: true,
                search: true,
                prefilled_url: true,
                health: true,
            },
            status: InstanceStatus::Experimental,
        });

        assert_eq!(registry.list().len(), 1);
        assert!(registry.get("uk-wdtk").is_some());
    }
}
