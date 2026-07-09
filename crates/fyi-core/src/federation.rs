//! Cross-jurisdiction federation view over the embedded instance catalog.
//!
//! Aggregates Alaveteli-family instances into summary records suitable for a
//! multi-site operator dashboard without requiring live network fan-out.

use crate::jurisdiction::{Instance, InstanceRegistry, InstanceStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Compact summary of one federated FOI instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedInstanceSummary {
    pub id: String,
    pub base_url: String,
    pub country: String,
    pub locale: String,
    pub law_name: String,
    pub statutory_deadline_days: Option<i32>,
    pub status: InstanceStatus,
    pub capabilities_read: bool,
    pub capabilities_write: bool,
    pub capabilities_search: bool,
}

impl From<&Instance> for FederatedInstanceSummary {
    fn from(instance: &Instance) -> Self {
        Self {
            id: instance.id.clone(),
            base_url: instance.base_url.clone(),
            country: instance.country.clone(),
            locale: instance.locale.clone(),
            law_name: instance.foi_law.law_name.clone(),
            statutory_deadline_days: instance.foi_law.statutory_deadline_days,
            status: instance.status.clone(),
            capabilities_read: instance.capabilities.read,
            capabilities_write: instance.capabilities.write,
            capabilities_search: instance.capabilities.search,
        }
    }
}

/// Aggregated cross-jurisdiction view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederationView {
    /// All known instances as summaries.
    pub instances: Vec<FederatedInstanceSummary>,
    /// Count of instances per ISO country code.
    pub by_country: BTreeMap<String, usize>,
    /// Count of instances per [`InstanceStatus`] label.
    pub by_status: BTreeMap<String, usize>,
    pub total: usize,
}

fn status_key(status: &InstanceStatus) -> String {
    match status {
        InstanceStatus::Supported => "supported".into(),
        InstanceStatus::Experimental => "experimental".into(),
        InstanceStatus::Community => "community".into(),
    }
}

/// Build a federation view from an [`InstanceRegistry`].
pub fn federation_view_from_registry(registry: &InstanceRegistry) -> FederationView {
    let mut instances: Vec<FederatedInstanceSummary> = registry
        .list()
        .into_iter()
        .map(FederatedInstanceSummary::from)
        .collect();
    instances.sort_by(|a, b| a.id.cmp(&b.id));

    let mut by_country: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
    for summary in &instances {
        *by_country.entry(summary.country.clone()).or_insert(0) += 1;
        *by_status.entry(status_key(&summary.status)).or_insert(0) += 1;
    }
    let total = instances.len();
    FederationView {
        instances,
        by_country,
        by_status,
        total,
    }
}

/// List federated instance summaries from the embedded catalog.
pub fn list_federated_summaries() -> anyhow::Result<Vec<FederatedInstanceSummary>> {
    let registry = InstanceRegistry::embedded()?;
    Ok(federation_view_from_registry(&registry).instances)
}

/// Full federation view from the embedded catalog.
pub fn embedded_federation_view() -> anyhow::Result<FederationView> {
    let registry = InstanceRegistry::embedded()?;
    Ok(federation_view_from_registry(&registry))
}

/// Filter federation instances by country code (case-insensitive).
pub fn filter_by_country(view: &FederationView, country: &str) -> Vec<FederatedInstanceSummary> {
    let country = country.to_ascii_uppercase();
    view.instances
        .iter()
        .filter(|s| s.country.eq_ignore_ascii_case(&country))
        .cloned()
        .collect()
}

/// Filter federation instances that advertise search capability.
pub fn filter_searchable(view: &FederationView) -> Vec<FederatedInstanceSummary> {
    view.instances
        .iter()
        .filter(|s| s.capabilities_search)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_view_includes_nz_and_uk() {
        let view = embedded_federation_view().unwrap();
        assert!(view.total >= 3);
        let ids: Vec<_> = view.instances.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"nz-fyi"));
        assert!(ids.contains(&"uk-wdtk"));
        assert!(ids.contains(&"au-rtk"));
        assert!(view.by_country.get("NZ").copied().unwrap_or(0) >= 1);
        assert!(
            view.by_status.contains_key("supported") || view.by_status.contains_key("experimental")
        );
    }

    #[test]
    fn list_federated_summaries_sorted() {
        let list = list_federated_summaries().unwrap();
        let mut sorted = list.clone();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(list, sorted);
    }

    #[test]
    fn filter_by_country_nz() {
        let view = embedded_federation_view().unwrap();
        let nz = filter_by_country(&view, "nz");
        assert!(!nz.is_empty());
        assert!(nz.iter().all(|s| s.country == "NZ"));
    }

    #[test]
    fn filter_searchable_returns_capable_only() {
        let view = embedded_federation_view().unwrap();
        let searchable = filter_searchable(&view);
        assert!(!searchable.is_empty());
        assert!(searchable.iter().all(|s| s.capabilities_search));
    }

    #[test]
    fn summary_serde_roundtrip() {
        let view = embedded_federation_view().unwrap();
        let json = serde_json::to_string(&view).unwrap();
        let restored: FederationView = serde_json::from_str(&json).unwrap();
        assert_eq!(view, restored);
    }
}
