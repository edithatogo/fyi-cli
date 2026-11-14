//! Explicit jurisdiction terminology; language selection never selects a law.
//!
//! Catalogs and Fluent resources are data only. Lookup performs no I/O and does
//! not infer legal applicability, available remedies or statutory deadlines.

use super::{
    BundleVersion, I18nError, LocaleResolution, LocalizationEngine, MessageBundle, ValidationStatus,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use thiserror::Error;

const EMBEDDED: &str = include_str!("../../locales/terminology/v1/catalog.json");
const MAX_CATALOG_BYTES: usize = 256 * 1024;

/// Stable concepts shared by jurisdiction glossaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Term {
    LawName,
    LawAbbreviation,
    Request,
    Authority,
    Deadline,
    Department,
    Ministry,
    Agency,
    FollowUp,
    InternalReview,
    Appeal,
}

impl Term {
    /// Complete concept inventory in stable display order.
    pub const ALL: [Self; 11] = [
        Self::LawName,
        Self::LawAbbreviation,
        Self::Request,
        Self::Authority,
        Self::Deadline,
        Self::Department,
        Self::Ministry,
        Self::Agency,
        Self::FollowUp,
        Self::InternalReview,
        Self::Appeal,
    ];
}

/// The glossary's preferred generic recipient noun, not an entity classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityTerm {
    Authority,
    Agency,
    Department,
    Ministry,
}

impl AuthorityTerm {
    fn key(self) -> &'static str {
        match self {
            Self::Authority => "authority",
            Self::Agency => "agency",
            Self::Department => "department",
            Self::Ministry => "ministry",
        }
    }
}

/// A source-qualified regime selected explicitly by the caller.
///
/// Country is descriptive metadata. It is never a lookup alias, and no instance,
/// locale, authority or network endpoint is automatically assigned to a profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JurisdictionTerminology {
    pub id: String,
    pub country: String,
    pub law_name: String,
    pub law_abbreviation: String,
    pub request_abbreviation: String,
    pub authority_term: AuthorityTerm,
    pub source_url: String,
    pub source_checked_at: NaiveDate,
}

/// A displayed term with independently inspectable catalog and language provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalizedTerm {
    pub concept: Term,
    pub text: String,
    pub jurisdiction_id: String,
    pub source_url: String,
    pub source_checked_at: NaiveDate,
    pub locale: LocaleResolution,
    /// None for untranslated legal names and abbreviations.
    pub source_locale: Option<String>,
    pub catalog_version: BundleVersion,
    /// None when no Fluent message was used.
    pub message_bundle_version: Option<BundleVersion>,
    pub validation: ValidationStatus,
}

/// Errors deliberately omit supplied catalogs, identifiers, values and source URLs.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TerminologyError {
    #[error("invalid terminology catalog")]
    InvalidCatalog,
    #[error("unsupported terminology catalog format version: {0}")]
    UnsupportedVersion(u32),
    #[error("invalid terminology profile")]
    InvalidProfile,
    #[error("duplicate terminology jurisdiction")]
    DuplicateJurisdiction,
    #[error("unknown terminology jurisdiction; select an explicit supported profile")]
    UnknownJurisdiction,
    #[error(transparent)]
    Localization(#[from] I18nError),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    version: BundleVersion,
    profiles: Vec<JurisdictionTerminology>,
}

/// Immutable, deterministically ordered terminology profiles.
#[derive(Debug, Clone)]
pub struct TerminologyRegistry {
    version: BundleVersion,
    profiles: BTreeMap<String, JurisdictionTerminology>,
}

impl TerminologyRegistry {
    /// Load and validate the versioned resources compiled into the binary.
    pub fn embedded() -> Result<Self, TerminologyError> {
        Self::from_json(EMBEDDED)
    }

    /// Parse caller-supplied data without fetching sources or loading any file.
    ///
    /// Unknown fields, duplicate profiles, malformed references and unsupported
    /// versions fail closed. The size limit applies before parsing. A custom
    /// catalog changes profiles only; it does not load code or Fluent overrides.
    pub fn from_json(source: &str) -> Result<Self, TerminologyError> {
        if source.len() > MAX_CATALOG_BYTES {
            return Err(TerminologyError::InvalidCatalog);
        }
        let catalog: Catalog =
            serde_json::from_str(source).map_err(|_| TerminologyError::InvalidCatalog)?;
        if catalog.version.format_version != 1 {
            return Err(TerminologyError::UnsupportedVersion(
                catalog.version.format_version,
            ));
        }
        if !valid_text(&catalog.version.revision, 64) || catalog.profiles.is_empty() {
            return Err(TerminologyError::InvalidCatalog);
        }
        let mut profiles = BTreeMap::new();
        for profile in catalog.profiles {
            validate_profile(&profile)?;
            if profiles.insert(profile.id.clone(), profile).is_some() {
                return Err(TerminologyError::DuplicateJurisdiction);
            }
        }
        Ok(Self {
            version: catalog.version,
            profiles,
        })
    }

    /// Catalog identity, independent of the crate and message-bundle versions.
    pub fn version(&self) -> &BundleVersion {
        &self.version
    }

    /// Supported explicit regimes in deterministic identifier order.
    pub fn profiles(&self) -> impl ExactSizeIterator<Item = &JurisdictionTerminology> {
        self.profiles.values()
    }

    /// Find an exact profile identifier; no country, case or language guessing.
    pub fn profile(&self, id: &str) -> Option<&JurisdictionTerminology> {
        self.profiles.get(id)
    }

    /// Resolve a concept while preserving the caller's explicit legal regime.
    ///
    /// Names and abbreviations remain literal in every language. Other concepts
    /// reuse the existing locale resolution and strict Fluent renderer. Output
    /// is unvalidated plain text; consumers own display-context escaping.
    pub fn lookup(
        &self,
        jurisdiction_id: &str,
        engine: &LocalizationEngine,
        concept: Term,
    ) -> Result<LocalizedTerm, TerminologyError> {
        let profile = self
            .profile(jurisdiction_id)
            .ok_or(TerminologyError::UnknownJurisdiction)?;
        enum Source<'a> {
            Literal(&'a str),
            Message(&'static str),
        }
        let source = match concept {
            Term::LawName => Source::Literal(&profile.law_name),
            Term::LawAbbreviation => Source::Literal(&profile.law_abbreviation),
            Term::Request => Source::Message("request"),
            Term::Authority => Source::Message(profile.authority_term.key()),
            Term::Deadline => Source::Message("deadline"),
            Term::Department => Source::Message("department"),
            Term::Ministry => Source::Message("ministry"),
            Term::Agency => Source::Message("agency"),
            Term::FollowUp => Source::Message("follow_up"),
            Term::InternalReview => Source::Message("internal_review"),
            Term::Appeal => Source::Message("appeal"),
        };
        let (text, source_locale, message_bundle_version) = match source {
            Source::Literal(text) => (text.to_owned(), None, None),
            Source::Message(key) => {
                let variables =
                    BTreeMap::from([("law".into(), profile.request_abbreviation.clone())]);
                let message = render_message(engine, key, &variables)?;
                (
                    message.text,
                    Some(message.source_locale),
                    Some(message.bundle_version),
                )
            }
        };
        Ok(LocalizedTerm {
            concept,
            text,
            jurisdiction_id: profile.id.clone(),
            source_url: profile.source_url.clone(),
            source_checked_at: profile.source_checked_at,
            locale: engine.resolution().clone(),
            source_locale,
            catalog_version: self.version.clone(),
            message_bundle_version,
            validation: ValidationStatus::RenderedUnvalidated,
        })
    }
}

fn valid_text(value: &str, max_bytes: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= max_bytes
        && !value
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
}

fn validate_profile(profile: &JurisdictionTerminology) -> Result<(), TerminologyError> {
    let valid_id = !profile.id.is_empty()
        && profile.id.len() <= 64
        && profile
            .id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    let valid_country =
        profile.country.len() == 2 && profile.country.bytes().all(|b| b.is_ascii_uppercase());
    let valid_labels = valid_text(&profile.law_name, 512)
        && valid_text(&profile.law_abbreviation, 256)
        && valid_text(&profile.request_abbreviation, 256);
    let valid_source = valid_text(&profile.source_url, 2048)
        && profile.source_url.trim() == profile.source_url
        && reqwest::Url::parse(&profile.source_url).is_ok_and(|url| {
            url.scheme() == "https"
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
        });
    if valid_id && valid_country && valid_labels && valid_source {
        Ok(())
    } else {
        Err(TerminologyError::InvalidProfile)
    }
}

fn bundles() -> &'static BTreeMap<&'static str, MessageBundle> {
    static BUNDLES: OnceLock<BTreeMap<&'static str, MessageBundle>> = OnceLock::new();
    BUNDLES.get_or_init(|| {
        let version = TerminologyRegistry::embedded()
            .expect("embedded terminology catalog is contract-tested")
            .version;
        [
            ("en", include_str!("../../locales/terminology/v1/en.ftl")),
            (
                "de-DE",
                include_str!("../../locales/terminology/v1/de-DE.ftl"),
            ),
            (
                "fr-FR",
                include_str!("../../locales/terminology/v1/fr-FR.ftl"),
            ),
            (
                "es-ES",
                include_str!("../../locales/terminology/v1/es-ES.ftl"),
            ),
        ]
        .into_iter()
        .map(|(locale, source)| {
            (
                locale,
                MessageBundle::from_ftl(locale, version.clone(), source)
                    .expect("embedded terminology messages are contract-tested"),
            )
        })
        .collect()
    })
}

fn render_message(
    engine: &LocalizationEngine,
    key: &str,
    variables: &BTreeMap<String, String>,
) -> Result<super::RenderedMessage, I18nError> {
    for layer in &engine.resolution().chain {
        let locale = if layer == "base" { "en" } else { layer };
        if let Some(bundle) = bundles().get(locale) {
            if bundle.bundle.get_message(key).is_some() {
                return Ok(super::RenderedMessage {
                    text: bundle.render(key, variables)?,
                    source_locale: locale.to_owned(),
                    bundle_version: bundle.version.clone(),
                });
            }
        }
    }
    Err(I18nError::MissingMessage { key: key.into() })
}
