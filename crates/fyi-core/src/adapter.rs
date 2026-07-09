//! Community jurisdiction adapter SDK.
//!
//! Third-party contributors can implement [`CommunityJurisdictionAdapter`] to
//! register FOI/OIA sites that are not yet first-party supported. Adapters
//! describe law metadata, capabilities, and optional draft scaffolding without
//! requiring network access at registration time.

use crate::jurisdiction::{Capabilities, FoiLaw, Instance, InstanceStatus};
use serde::{Deserialize, Serialize};

/// Metadata a community adapter must provide for catalog registration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterDescriptor {
    /// Stable adapter id (becomes instance id when registered).
    pub id: String,
    /// Human-readable adapter name.
    pub name: String,
    /// Adapter author or maintainer.
    pub author: String,
    /// Semver-ish version string for the adapter package.
    pub version: String,
    /// Short description of the jurisdiction / site.
    pub description: String,
}

/// Result of validating adapter configuration before registration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterValidation {
    pub ok: bool,
    pub messages: Vec<String>,
}

/// Community-contributed jurisdiction adapter.
///
/// Implementors map a remote Alaveteli (or compatible) site into the local
/// [`Instance`] catalog model. Network I/O is intentionally out of scope for
/// this trait; use [`crate::jurisdiction::FoiProvider`] for live API calls.
///
/// # Example
///
/// ```
/// use fyi_core::adapter::{
///     AdapterDescriptor, AdapterValidation, CommunityJurisdictionAdapter, StubCommunityAdapter,
/// };
/// use fyi_core::jurisdiction::InstanceStatus;
///
/// let adapter = StubCommunityAdapter::example_pacific();
/// assert_eq!(adapter.descriptor().id, "example-pacific-foi");
/// let instance = adapter.to_instance();
/// assert_eq!(instance.status, InstanceStatus::Community);
/// assert!(adapter.validate().ok);
/// ```
pub trait CommunityJurisdictionAdapter: Send + Sync {
    /// Static metadata about this adapter package.
    fn descriptor(&self) -> AdapterDescriptor;

    /// Build the catalog [`Instance`] this adapter contributes.
    fn to_instance(&self) -> Instance;

    /// Optional jurisdiction-specific draft opening paragraph.
    fn draft_scaffold(&self, authority_name: &str) -> String {
        let instance = self.to_instance();
        format!(
            "Dear {},\n\nUnder {} I request the following information.",
            authority_name, instance.foi_law.law_name
        )
    }

    /// Validate required fields before the instance is registered.
    fn validate(&self) -> AdapterValidation {
        let instance = self.to_instance();
        let mut messages = Vec::new();
        if instance.id.trim().is_empty() {
            messages.push("instance id must not be empty".into());
        }
        if instance.base_url.trim().is_empty() {
            messages.push("base_url must not be empty".into());
        }
        if !instance.base_url.starts_with("http://") && !instance.base_url.starts_with("https://") {
            messages.push("base_url must be an http(s) URL".into());
        }
        if instance.foi_law.law_name.trim().is_empty() {
            messages.push("foi_law.law_name must not be empty".into());
        }
        if instance.country.trim().is_empty() {
            messages.push("country must not be empty".into());
        }
        AdapterValidation {
            ok: messages.is_empty(),
            messages,
        }
    }
}

/// Register a validated community adapter into an in-memory instance list.
pub fn register_adapter_instance(
    adapters: &[Box<dyn CommunityJurisdictionAdapter>],
) -> Result<Vec<Instance>, Vec<String>> {
    let mut instances = Vec::new();
    let mut errors = Vec::new();
    for adapter in adapters {
        let validation = adapter.validate();
        if validation.ok {
            let mut instance = adapter.to_instance();
            instance.status = InstanceStatus::Community;
            instances.push(instance);
        } else {
            let desc = adapter.descriptor();
            for msg in validation.messages {
                errors.push(format!("{}: {}", desc.id, msg));
            }
        }
    }
    if errors.is_empty() {
        Ok(instances)
    } else {
        Err(errors)
    }
}

/// Example stub adapter for documentation and unit tests.
///
/// Represents a fictional Pacific FOI portal; not a live endpoint.
#[derive(Debug, Clone)]
pub struct StubCommunityAdapter {
    pub descriptor: AdapterDescriptor,
    pub base_url: String,
    pub country: String,
    pub locale: String,
    pub foi_law: FoiLaw,
    pub capabilities: Capabilities,
}

impl StubCommunityAdapter {
    /// Built-in example used in docs and tests.
    pub fn example_pacific() -> Self {
        Self {
            descriptor: AdapterDescriptor {
                id: "example-pacific-foi".into(),
                name: "Example Pacific FOI".into(),
                author: "fyi-cli community".into(),
                version: "0.1.0".into(),
                description: "Stub adapter demonstrating the community jurisdiction SDK.".into(),
            },
            base_url: "https://example.pacific-foi.invalid".into(),
            country: "PC".into(),
            locale: "en-PC".into(),
            foi_law: FoiLaw {
                law_name: "Pacific Access to Information Act".into(),
                citation: Some("PAIA 2020".into()),
                request_term: "information request".into(),
                statutory_deadline_days: Some(20),
                appeal_body: Some("Pacific Information Commissioner".into()),
            },
            capabilities: Capabilities {
                read: true,
                write: false,
                attachments: false,
                batch: false,
                feeds: true,
                search: true,
                prefilled_url: false,
                health: true,
            },
        }
    }
}

impl CommunityJurisdictionAdapter for StubCommunityAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        self.descriptor.clone()
    }

    fn to_instance(&self) -> Instance {
        Instance {
            id: self.descriptor.id.clone(),
            base_url: self.base_url.clone(),
            country: self.country.clone(),
            locale: self.locale.clone(),
            foi_law: self.foi_law.clone(),
            capabilities: self.capabilities.clone(),
            status: InstanceStatus::Community,
        }
    }

    fn draft_scaffold(&self, authority_name: &str) -> String {
        format!(
            "Dear {},\n\nUnder the {} ({}) I make this {}.",
            authority_name,
            self.foi_law.law_name,
            self.foi_law
                .citation
                .as_deref()
                .unwrap_or(self.foi_law.law_name.as_str()),
            self.foi_law.request_term
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_adapter_validates_and_builds_instance() {
        let adapter = StubCommunityAdapter::example_pacific();
        let validation = adapter.validate();
        assert!(validation.ok, "{:?}", validation.messages);
        let instance = adapter.to_instance();
        assert_eq!(instance.id, "example-pacific-foi");
        assert_eq!(instance.status, InstanceStatus::Community);
        assert!(instance.capabilities.read);
        assert!(!instance.capabilities.write);
    }

    #[test]
    fn draft_scaffold_mentions_law() {
        let adapter = StubCommunityAdapter::example_pacific();
        let text = adapter.draft_scaffold("Ministry of Example");
        assert!(text.contains("Ministry of Example"));
        assert!(text.contains("Pacific Access to Information Act"));
        assert!(text.contains("PAIA 2020"));
    }

    #[test]
    fn invalid_base_url_fails_validation() {
        let mut adapter = StubCommunityAdapter::example_pacific();
        adapter.base_url = "not-a-url".into();
        let validation = adapter.validate();
        assert!(!validation.ok);
        assert!(validation.messages.iter().any(|m| m.contains("base_url")));
    }

    #[test]
    fn register_adapter_instance_collects_valid() {
        let good: Box<dyn CommunityJurisdictionAdapter> =
            Box::new(StubCommunityAdapter::example_pacific());
        let mut bad = StubCommunityAdapter::example_pacific();
        bad.descriptor.id = String::new();
        bad.base_url = String::new();
        let bad: Box<dyn CommunityJurisdictionAdapter> = Box::new(bad);

        let ok = register_adapter_instance(&[good]).unwrap();
        assert_eq!(ok.len(), 1);

        let err = register_adapter_instance(&[bad]).unwrap_err();
        assert!(!err.is_empty());
    }
}
