//! Experimental feature flags for bleeding-edge capabilities.
//!
//! Flags gate unfinished or optional behaviour without requiring compile-time
//! Cargo features. Defaults keep experimental work **off** so stable code paths
//! stay unchanged unless an operator explicitly enables them.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

/// Named experimental / optional capabilities.
///
/// String form uses `snake_case` identifiers (see [`FeatureFlag::as_str`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureFlag {
    /// Multi-turn drafting refinement and quality scoring.
    AiDraftingRefinement,
    /// Community jurisdiction adapter registration path.
    CommunityAdapters,
    /// Deadline reminder / webhook payload generation (no delivery).
    DeadlineNotifications,
    /// Offline dashboard PWA shell (service worker / manifest).
    OfflinePwa,
    /// MCP resource catalog exposure of local corpus.
    McpResources,
    /// Cross-jurisdiction federation view aggregation.
    FederationView,
    /// Provenance hash-chain archive integrity.
    ProvenanceChain,
    /// Hybrid / FTS search over local requests.
    AdvancedSearch,
}

impl FeatureFlag {
    /// All known flags (stable order).
    pub fn all() -> &'static [FeatureFlag] {
        &[
            FeatureFlag::AiDraftingRefinement,
            FeatureFlag::CommunityAdapters,
            FeatureFlag::DeadlineNotifications,
            FeatureFlag::OfflinePwa,
            FeatureFlag::McpResources,
            FeatureFlag::FederationView,
            FeatureFlag::ProvenanceChain,
            FeatureFlag::AdvancedSearch,
        ]
    }

    /// Snake-case identifier used in config and docs.
    pub fn as_str(self) -> &'static str {
        match self {
            FeatureFlag::AiDraftingRefinement => "ai_drafting_refinement",
            FeatureFlag::CommunityAdapters => "community_adapters",
            FeatureFlag::DeadlineNotifications => "deadline_notifications",
            FeatureFlag::OfflinePwa => "offline_pwa",
            FeatureFlag::McpResources => "mcp_resources",
            FeatureFlag::FederationView => "federation_view",
            FeatureFlag::ProvenanceChain => "provenance_chain",
            FeatureFlag::AdvancedSearch => "advanced_search",
        }
    }

    /// Short human description.
    pub fn description(self) -> &'static str {
        match self {
            FeatureFlag::AiDraftingRefinement => {
                "Multi-turn drafting refinement and heuristic quality scoring"
            }
            FeatureFlag::CommunityAdapters => {
                "CommunityJurisdictionAdapter registration for third-party FOI sites"
            }
            FeatureFlag::DeadlineNotifications => {
                "Deadline reminder schedules and webhook payload builders"
            }
            FeatureFlag::OfflinePwa => "Dashboard offline PWA shell (manifest + service worker)",
            FeatureFlag::McpResources => "MCP resources for local authorities/requests corpus",
            FeatureFlag::FederationView => "Cross-jurisdiction federation summary views",
            FeatureFlag::ProvenanceChain => "SHA-256 provenance hash chain for archives",
            FeatureFlag::AdvancedSearch => "In-memory hybrid full-text search index",
        }
    }
}

impl fmt::Display for FeatureFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FeatureFlag {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_ascii_lowercase().replace('-', "_");
        FeatureFlag::all()
            .iter()
            .copied()
            .find(|flag| flag.as_str() == normalized)
            .ok_or_else(|| format!("unknown feature flag: {s}"))
    }
}

/// Mutable set of enabled feature flags.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FeatureSet {
    enabled: BTreeSet<FeatureFlag>,
}

impl FeatureSet {
    /// Empty set — all experimental features disabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable every known experimental flag (opt-in “bleeding edge” profile).
    pub fn all_experimental() -> Self {
        let mut set = Self::new();
        for flag in FeatureFlag::all() {
            set.enable(*flag);
        }
        set
    }

    /// Default production-safe set: no experimental flags enabled.
    pub fn default_experimental() -> Self {
        Self::new()
    }

    /// Enable a single flag.
    pub fn enable(&mut self, flag: FeatureFlag) {
        self.enabled.insert(flag);
    }

    /// Disable a single flag.
    pub fn disable(&mut self, flag: FeatureFlag) {
        self.enabled.remove(&flag);
    }

    /// Whether `flag` is currently enabled.
    pub fn is_enabled(&self, flag: FeatureFlag) -> bool {
        self.enabled.contains(&flag)
    }

    /// Iterator over enabled flags in sorted order.
    pub fn enabled_flags(&self) -> impl Iterator<Item = FeatureFlag> + '_ {
        self.enabled.iter().copied()
    }

    /// Number of enabled flags.
    pub fn len(&self) -> usize {
        self.enabled.len()
    }

    /// True when no flags are enabled.
    pub fn is_empty(&self) -> bool {
        self.enabled.is_empty()
    }

    /// Enable a flag from its string name. Returns an error for unknown names.
    pub fn enable_str(&mut self, name: &str) -> Result<(), String> {
        let flag = FeatureFlag::from_str(name)?;
        self.enable(flag);
        Ok(())
    }

    /// Disable a flag from its string name. Unknown names are ignored (no-op success).
    pub fn disable_str(&mut self, name: &str) -> Result<(), String> {
        let flag = FeatureFlag::from_str(name)?;
        self.disable(flag);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_experimental_flags() {
        let set = FeatureSet::default_experimental();
        assert!(set.is_empty());
        for flag in FeatureFlag::all() {
            assert!(!set.is_enabled(*flag), "{flag} should be off by default");
        }
    }

    #[test]
    fn enable_disable_roundtrip() {
        let mut set = FeatureSet::new();
        set.enable(FeatureFlag::OfflinePwa);
        assert!(set.is_enabled(FeatureFlag::OfflinePwa));
        assert!(!set.is_enabled(FeatureFlag::McpResources));
        set.disable(FeatureFlag::OfflinePwa);
        assert!(!set.is_enabled(FeatureFlag::OfflinePwa));
    }

    #[test]
    fn all_experimental_enables_everything() {
        let set = FeatureSet::all_experimental();
        assert_eq!(set.len(), FeatureFlag::all().len());
        for flag in FeatureFlag::all() {
            assert!(set.is_enabled(*flag));
        }
    }

    #[test]
    fn from_str_accepts_snake_and_kebab() {
        assert_eq!(
            FeatureFlag::from_str("ai_drafting_refinement").unwrap(),
            FeatureFlag::AiDraftingRefinement
        );
        assert_eq!(
            FeatureFlag::from_str("offline-pwa").unwrap(),
            FeatureFlag::OfflinePwa
        );
        assert!(FeatureFlag::from_str("not_a_real_flag").is_err());
    }

    #[test]
    fn enable_str_and_serde() {
        let mut set = FeatureSet::new();
        set.enable_str("deadline_notifications").unwrap();
        assert!(set.is_enabled(FeatureFlag::DeadlineNotifications));

        let json = serde_json::to_string(&set).unwrap();
        let roundtrip: FeatureSet = serde_json::from_str(&json).unwrap();
        assert_eq!(set, roundtrip);
    }

    #[test]
    fn display_matches_as_str() {
        for flag in FeatureFlag::all() {
            assert_eq!(flag.to_string(), flag.as_str());
            assert!(!flag.description().is_empty());
        }
    }
}
