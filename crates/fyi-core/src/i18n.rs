//! Deterministic, data-only localization. Rendering never validates a legal request.
//!
//! Locale identifiers are normalized separately from user text. Interpolated text
//! is preserved as supplied: there is no NFC/NFKC normalization, recursive
//! evaluation, filesystem access, or registered Fluent callback. Raw messages are
//! plain text; typed letter output can explicitly request HTML text escaping.

use crate::jurisdiction::Instance;
use chrono::{Days, NaiveDate};

mod templates;
pub mod terminology;
use fluent_bundle::{concurrent::FluentBundle, FluentArgs, FluentResource};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;
pub use templates::{
    LegalCitation, LetterRegister, OutputFormat, RenderedRequest, RequestTemplateInput,
    ReviewInformation, TemplateVariant, ValidationStatus,
};
use thiserror::Error;
use unic_langid::LanguageIdentifier;

/// The locale coverage promised by the embedded resources, in stable order.
pub const SUPPORTED_LOCALES: [&str; 6] = ["en-NZ", "en-AU", "en-GB", "de-DE", "fr-FR", "es-ES"];

/// Version of the resource contract and independently maintained message content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleVersion {
    pub format_version: u32,
    pub revision: String,
}

impl BundleVersion {
    /// Metadata shipped alongside the embedded resources, independent of crate version.
    pub fn embedded() -> Self {
        serde_json::from_str(include_str!("../locales/v1/version.json"))
            .expect("embedded bundle version is validated by localization tests")
    }
}

/// How the requested identifier selected a message language, never a jurisdiction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocaleMatch {
    Exact,
    LanguageFallback,
    BaseFallback,
    InvalidFallback,
}

/// Inspectable, deterministic locale negotiation; no process locale is consulted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocaleResolution {
    pub requested: String,
    pub normalized: Option<String>,
    pub resolved: String,
    pub chain: Vec<String>,
    pub matched: LocaleMatch,
}

/// Errors omit variable values and resource text, which may contain private data.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum I18nError {
    #[error("malformed locale identifier")]
    InvalidLocale,
    #[error("locale is not explicitly supported")]
    UnsupportedLocale,
    #[error("unsupported message bundle format version: {0}")]
    UnsupportedBundleVersion(u32),
    #[error("message bundle revision is empty")]
    EmptyBundleRevision,
    #[error("invalid Fluent resource")]
    InvalidResource,
    #[error("duplicate Fluent entry")]
    DuplicateMessage,
    #[error("missing message: {key}")]
    MissingMessage { key: String },
    #[error("message formatting failed (missing variable or invalid expression): {key}")]
    Formatting { key: String },
    #[error("required template field is empty: {field}")]
    MissingField { field: &'static str },
    #[error("template field contains a disallowed control character: {field}")]
    InvalidField { field: &'static str },
    #[error("deadline exceeds the representable date or day-count range")]
    DeadlineRange,
}

/// Parsed Fluent resource with explicit schema/content versioning.
///
/// No functions are registered. Unknown functions and missing variables are errors;
/// partial output from Fluent is never returned as a successful rendering.
pub struct MessageBundle {
    version: BundleVersion,
    bundle: FluentBundle<FluentResource>,
}

impl MessageBundle {
    /// Parse a complete resource, rejecting syntax errors and duplicate entries.
    pub fn from_ftl(locale: &str, version: BundleVersion, source: &str) -> Result<Self, I18nError> {
        if version.format_version != 1 {
            return Err(I18nError::UnsupportedBundleVersion(version.format_version));
        }
        if version.revision.trim().is_empty() {
            return Err(I18nError::EmptyBundleRevision);
        }
        let locale: LanguageIdentifier = locale.parse().map_err(|_| I18nError::InvalidLocale)?;
        let resource =
            FluentResource::try_new(source.to_owned()).map_err(|_| I18nError::InvalidResource)?;
        let mut bundle = FluentBundle::new_concurrent(vec![locale]);
        // Preserve legacy plain-text output and argument bytes. RTL layout is not
        // part of the supported locale contract; consumers own their display context.
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .map_err(|_| I18nError::DuplicateMessage)?;
        Ok(Self { version, bundle })
    }

    fn contains(&self, key: &str) -> bool {
        self.bundle
            .get_message(key)
            .and_then(|message| message.value())
            .is_some()
    }

    /// Verify that required identifiers have message values (attributes alone do not count).
    pub fn require_messages(&self, keys: &[&str]) -> Result<(), I18nError> {
        for key in keys {
            if !self.contains(key) {
                return Err(I18nError::MissingMessage { key: (*key).into() });
            }
        }
        Ok(())
    }

    /// Format once. Variable values are strings, never Fluent source or executable code.
    /// Additional variables are ignored by Fluent; required variables must be present.
    pub fn render(
        &self,
        key: &str,
        variables: &BTreeMap<String, String>,
    ) -> Result<String, I18nError> {
        let message = self
            .bundle
            .get_message(key)
            .ok_or_else(|| I18nError::MissingMessage { key: key.into() })?;
        let pattern = message
            .value()
            .ok_or_else(|| I18nError::MissingMessage { key: key.into() })?;
        let mut args = FluentArgs::new();
        for (name, value) in variables {
            args.set(name.as_str(), value.as_str());
        }
        let mut errors = Vec::new();
        let output = self
            .bundle
            .format_pattern(pattern, Some(&args), &mut errors);
        if !errors.is_empty() {
            return Err(I18nError::Formatting { key: key.into() });
        }
        Ok(output.into_owned())
    }
}

/// Rendered text and the resource layer that actually supplied it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderedMessage {
    pub text: String,
    pub source_locale: String,
    pub bundle_version: BundleVersion,
}

fn embedded_bundles() -> &'static BTreeMap<&'static str, MessageBundle> {
    static BUNDLES: OnceLock<BTreeMap<&'static str, MessageBundle>> = OnceLock::new();
    BUNDLES.get_or_init(|| {
        let english = include_str!("../locales/v1/en/common.ftl");
        [
            ("base", "en", english),
            ("en", "en", english),
            (
                "en-NZ",
                "en-NZ",
                include_str!("../locales/v1/en-NZ/common.ftl"),
            ),
            (
                "en-AU",
                "en-AU",
                include_str!("../locales/v1/en-AU/common.ftl"),
            ),
            (
                "en-GB",
                "en-GB",
                include_str!("../locales/v1/en-GB/common.ftl"),
            ),
            (
                "de-DE",
                "de-DE",
                include_str!("../locales/v1/de-DE/common.ftl"),
            ),
            (
                "fr-FR",
                "fr-FR",
                include_str!("../locales/v1/fr-FR/common.ftl"),
            ),
            (
                "es-ES",
                "es-ES",
                include_str!("../locales/v1/es-ES/common.ftl"),
            ),
        ]
        .into_iter()
        .map(|(layer, locale, source)| {
            (
                layer,
                MessageBundle::from_ftl(locale, BundleVersion::embedded(), source)
                    .expect("embedded resources are validated by localization tests"),
            )
        })
        .collect()
    })
}

fn resolve_locale(requested: &str) -> LocaleResolution {
    let parsed = requested.parse::<LanguageIdentifier>().ok();
    let normalized = parsed.as_ref().map(ToString::to_string);
    let (resolved, matched) = match normalized.as_deref() {
        Some(locale) if SUPPORTED_LOCALES.contains(&locale) => {
            (locale.to_owned(), LocaleMatch::Exact)
        }
        Some(_) => {
            let language = parsed
                .as_ref()
                .map(|id| id.language.as_str())
                .unwrap_or_default();
            match language {
                "en" => ("en".into(), LocaleMatch::LanguageFallback),
                "de" => ("de-DE".into(), LocaleMatch::LanguageFallback),
                "fr" => ("fr-FR".into(), LocaleMatch::LanguageFallback),
                "es" => ("es-ES".into(), LocaleMatch::LanguageFallback),
                _ => ("en".into(), LocaleMatch::BaseFallback),
            }
        }
        None => ("en".into(), LocaleMatch::InvalidFallback),
    };
    let mut chain = vec![resolved.clone()];
    if resolved.starts_with("en-") {
        chain.push("en".into());
    }
    chain.push("base".into());
    LocaleResolution {
        requested: requested.into(),
        normalized,
        resolved,
        chain,
        matched,
    }
}

/// Legacy serializable translation view; new consumers should use strict message rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocaleBundle {
    pub locale: String,
    translations: BTreeMap<String, String>,
}

impl LocaleBundle {
    /// Preserve the legacy wire shape, while obtaining strings from versioned resources.
    pub fn for_locale(locale: &str) -> Self {
        let engine = LocalizationEngine::new(locale);
        let variables = BTreeMap::from([("authority".into(), "{authority}".into())]);
        let translations = [
            "salutation",
            "request_term",
            "closing",
            "working_days",
            "working_day",
            "deadline",
        ]
        .into_iter()
        .map(|key| (key.into(), engine.embedded_message(key, &variables)))
        .collect();
        Self {
            locale: locale.to_ascii_lowercase(),
            translations,
        }
    }

    /// Legacy explicit caller fallback; use `render_message` to detect missing messages.
    pub fn translate(&self, key: &str, fallback: &str) -> String {
        self.translations
            .get(key)
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    }
}

/// Locale formatting only: legal rules remain in `jurisdiction` and `deadlines`.
#[derive(Debug, Clone)]
pub struct LocalizationEngine {
    locale: String,
    resolution: LocaleResolution,
}

impl LocalizationEngine {
    /// Compatibility constructor with observable, deterministic English/language fallback.
    pub fn new(locale: &str) -> Self {
        Self {
            locale: locale.into(),
            resolution: resolve_locale(locale),
        }
    }

    /// Select only a supported identifier; normalize case and `_` separators first.
    pub fn try_new(locale: &str) -> Result<Self, I18nError> {
        let normalized: LanguageIdentifier =
            locale.parse().map_err(|_| I18nError::InvalidLocale)?;
        if !SUPPORTED_LOCALES.contains(&normalized.to_string().as_str()) {
            return Err(I18nError::UnsupportedLocale);
        }
        Ok(Self::new(locale))
    }

    /// Original caller-supplied identifier, retained for source compatibility.
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Selected locale and ordered message fallback layers.
    pub fn resolution(&self) -> &LocaleResolution {
        &self.resolution
    }

    /// Resolve a message in order. A malformed existing translation is an error, not a fallback.
    pub fn render_message(
        &self,
        key: &str,
        variables: &BTreeMap<String, String>,
    ) -> Result<RenderedMessage, I18nError> {
        for layer in &self.resolution.chain {
            if let Some(bundle) = embedded_bundles().get(layer.as_str()) {
                if bundle.bundle.get_message(key).is_some() {
                    return Ok(RenderedMessage {
                        text: bundle.render(key, variables)?,
                        source_locale: layer.clone(),
                        bundle_version: bundle.version.clone(),
                    });
                }
            }
        }
        Err(I18nError::MissingMessage { key: key.into() })
    }

    fn embedded_message(&self, key: &str, variables: &BTreeMap<String, String>) -> String {
        self.render_message(key, variables)
            .expect("embedded message contract is validated by localization tests")
            .text
    }

    /// Legacy unvalidated plain-text scaffold; its last line remains the sign-off.
    pub fn render_request_template(&self, authority_name: &str, law_name: &str) -> String {
        let mut variables = BTreeMap::from([
            ("authority".into(), authority_name.into()),
            ("law_name".into(), law_name.into()),
        ]);
        variables.insert(
            "request_term".into(),
            self.embedded_message("request_term", &variables),
        );
        format!(
            "{}\n\n{}\n\n{}",
            self.embedded_message("salutation", &variables),
            self.embedded_message("legacy_request_body", &variables),
            self.embedded_message("closing", &variables)
        )
    }

    /// Format a supplied count; this label does not decide a statutory counting rule.
    pub fn deadline_label(&self, days: usize) -> String {
        let key = if days == 1 {
            "working_day"
        } else {
            "working_days"
        };
        format!("{} {}", days, self.embedded_message(key, &BTreeMap::new()))
    }

    /// Legacy instance scaffold. Catalog references and output shape remain compatible.
    pub fn render_request_template_with_instance(
        &self,
        authority_name: &str,
        instance: &Instance,
    ) -> String {
        let law_name = instance.foi_law.law_name.as_str();
        let citation = instance.foi_law.citation.as_deref().unwrap_or(law_name);
        let mut variables = BTreeMap::from([
            ("authority".into(), authority_name.into()),
            (
                "legal_citation".into(),
                format!("{} ({})", law_name, citation),
            ),
            ("request_term".into(), instance.foi_law.request_term.clone()),
        ]);
        let mut sections = vec![
            self.embedded_message("salutation", &variables),
            self.embedded_message("legacy_instance_body", &variables),
        ];
        if let Some(body) = instance.foi_law.appeal_body.as_deref() {
            variables.insert("review_authority".into(), body.into());
            sections.push(self.embedded_message("legacy_review", &variables));
        }
        sections.join("\n\n")
    }

    /// Compatibility adapter. Panics for an unrepresentable result; prefer the fallible API.
    /// This is a weekday-only catalog estimate, not a legally verified deadline.
    pub fn deadline_for_instance(&self, start: NaiveDate, instance: &Instance) -> NaiveDate {
        self.try_deadline_for_instance(start, instance)
            .expect("deadline is outside the supported date range")
    }

    /// Apply existing catalog policy and delegate arithmetic to the canonical deadline module.
    pub fn try_deadline_for_instance(
        &self,
        start: NaiveDate,
        instance: &Instance,
    ) -> Result<NaiveDate, I18nError> {
        let input = crate::deadlines::DeadlineInput::from_instance(start, instance);
        check_deadline_capacity(&input)?;
        Ok(crate::deadlines::calculate_deadline_for_instance(start, instance).due_date)
    }

    /// Compatibility weekday adapter. Panics for unrepresentable counts/results.
    pub fn add_working_days(&self, start: NaiveDate, days: usize) -> NaiveDate {
        self.try_add_working_days(start, days)
            .expect("deadline is outside the supported date range")
    }

    /// Check representation bounds, then use the existing weekday arithmetic.
    pub fn try_add_working_days(
        &self,
        start: NaiveDate,
        days: usize,
    ) -> Result<NaiveDate, I18nError> {
        let days = u32::try_from(days).map_err(|_| I18nError::DeadlineRange)?;
        let input = crate::deadlines::DeadlineInput::new(start, days);
        check_deadline_capacity(&input)?;
        Ok(crate::deadlines::add_working_days(
            start,
            days,
            input.working_day_rule,
        ))
    }
}

// Representation guard only, not a calendar engine. Count capacity until MAX in
// full weeks and <=6 tail dates, using the canonical weekday predicate. This
// prevents the legacy engine from repeatedly operating on its terminal date.
fn check_deadline_capacity(input: &crate::deadlines::DeadlineInput) -> Result<(), I18nError> {
    use crate::deadlines::{is_weekday, WorkingDayRule};
    let available = (NaiveDate::MAX - input.start_date).num_days() as u64;
    let capacity = match input.working_day_rule {
        WorkingDayRule::CalendarDays => available,
        WorkingDayRule::WeekdaysOnly => {
            let mut capacity = available / 7 * 5;
            for offset in 1..=available % 7 {
                let day = input
                    .start_date
                    .checked_add_days(Days::new(offset))
                    .ok_or(I18nError::DeadlineRange)?;
                if is_weekday(day) {
                    capacity += 1;
                }
            }
            capacity
        }
    };
    if u64::from(input.statutory_deadline_days) > capacity {
        return Err(I18nError::DeadlineRange);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jurisdiction::InstanceRegistry;

    #[test]
    fn capacity_guard_preserves_canonical_calendar_day_boundaries() {
        use crate::deadlines::{DeadlineInput, WorkingDayRule};
        let start = NaiveDate::MAX.pred_opt().unwrap();
        let valid = DeadlineInput::new(start, 1).with_rule(WorkingDayRule::CalendarDays);
        assert!(check_deadline_capacity(&valid).is_ok());
        let invalid = DeadlineInput::new(start, 2).with_rule(WorkingDayRule::CalendarDays);
        assert_eq!(
            check_deadline_capacity(&invalid),
            Err(I18nError::DeadlineRange)
        );
    }

    #[test]
    fn english_locale_uses_english_terms() {
        let engine = LocalizationEngine::new("en-NZ");
        assert_eq!(engine.deadline_label(2), "2 working days");
        let rendered = engine.render_request_template("Ministry", "OIA");
        assert!(rendered.contains("Kia ora Ministry"));
        assert!(rendered.contains("request"));
    }

    #[test]
    fn add_working_days_skips_weekends() {
        let engine = LocalizationEngine::new("en-AU");
        let start = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let deadline = engine.add_working_days(start, 2);
        assert_eq!(deadline, NaiveDate::from_ymd_opt(2026, 7, 7).unwrap());
    }

    #[test]
    fn au_instance_template_includes_foi_act_citation_and_appeal_body() {
        let engine = LocalizationEngine::new("en-AU");
        let registry = InstanceRegistry::embedded().unwrap();
        let instance = registry.get("au-rtk").unwrap();
        let template = engine.render_request_template_with_instance("Right To Know", instance);

        assert!(template.contains("Freedom of Information Act"));
        assert!(template.contains("FOI Act"));
        assert!(template.contains("FOI request"));
        assert!(template.contains("Australian Information Commissioner"));
    }

    #[test]
    fn nz_locale_uses_kia_ora_salutation_and_deadline_for_instance() {
        let engine = LocalizationEngine::new("en-NZ");
        let registry = InstanceRegistry::embedded().unwrap();
        let instance = registry.get("nz-fyi").unwrap();
        let start = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let deadline = engine.deadline_for_instance(start, instance);

        assert!(engine
            .render_request_template_with_instance("Ministry", instance)
            .contains("Kia ora Ministry"));
        assert_eq!(deadline, NaiveDate::from_ymd_opt(2026, 7, 31).unwrap());
    }

    #[test]
    fn de_locale_uses_german_terms_for_non_english_instances() {
        let engine = LocalizationEngine::new("de-DE");
        let registry = InstanceRegistry::embedded().unwrap();
        let instance = registry.get("de-fds").unwrap();
        let template = engine.render_request_template_with_instance("FragDenStaat", instance);

        assert!(template.contains("Sehr geehrte/r FragDenStaat"));
        assert!(template.contains("Informationsfreiheitsgesetz"));
    }

    #[test]
    fn fr_and_es_locales_render_locale_specific_templates() {
        let fr_engine = LocalizationEngine::new("fr-FR");
        let es_engine = LocalizationEngine::new("es-ES");
        let registry = InstanceRegistry::embedded().unwrap();
        let fr_instance = registry.get("fr-cada").unwrap();
        let es_instance = registry.get("es-tdas").unwrap();

        let fr_template = fr_engine.render_request_template_with_instance("CADA", fr_instance);
        let es_template =
            es_engine.render_request_template_with_instance("Tu Derecho a Saber", es_instance);

        assert!(fr_template.contains("Bonjour CADA"));
        assert!(fr_template.contains("Loi sur l'accès aux documents administratifs"));
        assert!(es_template.contains("Estimado/a Tu Derecho a Saber"));
        assert!(es_template.contains("Ley de Transparencia"));
    }
}
