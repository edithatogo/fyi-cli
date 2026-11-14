//! Offline terminology contracts; rendered labels do not establish legal applicability.

use fyi_core::i18n::terminology::{Term, TerminologyError, TerminologyRegistry};
use fyi_core::i18n::{LocaleMatch, LocalizationEngine, ValidationStatus, SUPPORTED_LOCALES};
use serde::Deserialize;
use serde_json::{json, Value};

const PRIVATE: &str = "synthetic-private-terminology";

fn synthetic_catalog() -> Value {
    json!({
        "version": {"format_version": 1, "revision": "synthetic.1"},
        "profiles": [{
            "id": "synthetic-law",
            "country": "ZZ",
            "law_name": "Synthetic Information Act",
            "law_abbreviation": "SIA",
            "request_abbreviation": "SIA",
            "authority_term": "agency",
            "source_url": "https://example.invalid/terminology",
            "source_checked_at": "2026-11-14"
        }]
    })
}

#[test]
fn embedded_profiles_cover_explicit_regimes_without_country_inference() {
    let registry = TerminologyRegistry::embedded().unwrap();
    assert_eq!(registry.version().format_version, 1);
    assert_eq!(registry.version().revision, "2026-11-14.1");
    let profiles = [
        ("au-foi", "AU", "FOI", "FOI request"),
        ("au-qld-rti", "AU", "RTI", "RTI request"),
        ("au-tas-rti", "AU", "RTI", "RTI request"),
        ("de-ifg", "DE", "IFG", "IFG request"),
        ("es-ltaibg", "ES", "LTAIBG", "LTAIBG request"),
        ("fr-crpa", "FR", "CRPA", "CRPA request"),
        ("gb-foia", "GB", "FOIA", "FOI request"),
        ("nz-oia", "NZ", "OIA", "OIA request"),
        ("us-foia", "US", "FOIA", "FOIA request"),
    ];
    assert_eq!(
        registry
            .profiles()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>(),
        profiles.map(|p| p.0)
    );
    let english = LocalizationEngine::new("en");
    for (id, country, abbreviation, request) in profiles {
        let profile = registry.profile(id).unwrap();
        assert_eq!(profile.country, country);
        assert_eq!(profile.law_abbreviation, abbreviation);
        assert!(profile.source_url.starts_with("https://"));
        assert_eq!(profile.source_checked_at.to_string(), "2026-11-14");
        let term = registry.lookup(id, &english, Term::Request).unwrap();
        assert_eq!(term.text, request);
        assert_eq!(term.source_locale.as_deref(), Some("en"));
        assert_eq!(term.catalog_version.revision, "2026-11-14.1");
        assert_eq!(
            term.message_bundle_version.unwrap().revision,
            "2026-11-14.1"
        );
    }
    assert!(registry.profile("AU").is_none());
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Golden {
    locale: String,
    jurisdiction_id: String,
    terms: Vec<(Term, String)>,
}

#[test]
fn six_locale_glossaries_match_independent_goldens() {
    let registry = TerminologyRegistry::embedded().unwrap();
    let fixtures: Vec<Golden> =
        serde_json::from_str(include_str!("fixtures/i18n/terminology.json")).unwrap();
    assert_eq!(
        fixtures
            .iter()
            .map(|f| f.locale.as_str())
            .collect::<Vec<_>>(),
        SUPPORTED_LOCALES
    );
    for fixture in fixtures {
        let engine = LocalizationEngine::try_new(&fixture.locale).unwrap();
        assert_eq!(
            fixture
                .terms
                .iter()
                .map(|(term, _)| *term)
                .collect::<Vec<_>>(),
            Term::ALL
        );
        for (concept, text) in fixture.terms {
            let actual = registry
                .lookup(&fixture.jurisdiction_id, &engine, concept)
                .unwrap();
            assert_eq!(actual.text, text, "{}: {concept:?}", fixture.locale);
            assert_eq!(actual.validation, ValidationStatus::RenderedUnvalidated);
        }
    }
}

#[test]
fn changing_display_language_never_changes_jurisdiction_or_legal_names() {
    let registry = TerminologyRegistry::embedded().unwrap();
    for profile in registry.profiles() {
        for locale in SUPPORTED_LOCALES {
            let engine = LocalizationEngine::try_new(locale).unwrap();
            for concept in Term::ALL {
                let term = registry.lookup(&profile.id, &engine, concept).unwrap();
                assert_eq!(term.jurisdiction_id, profile.id);
                assert_eq!(term.source_url, profile.source_url);
                assert_eq!(term.source_checked_at, profile.source_checked_at);
                assert_eq!(term.locale, *engine.resolution());
                assert!(!term.text.trim().is_empty());
                assert_eq!(
                    registry.lookup(&profile.id, &engine, concept).unwrap(),
                    term
                );
                if concept == Term::LawName || concept == Term::LawAbbreviation {
                    assert_eq!(term.source_locale, None);
                    assert_eq!(term.message_bundle_version, None);
                    assert_eq!(
                        term.text.as_str(),
                        if concept == Term::LawName {
                            profile.law_name.as_str()
                        } else {
                            profile.law_abbreviation.as_str()
                        }
                    );
                } else {
                    assert!(term.source_locale.is_some());
                    assert!(term.message_bundle_version.is_some());
                }
            }
        }
    }
}

#[test]
fn locale_fallback_is_observable_and_keeps_the_selected_regime() {
    let registry = TerminologyRegistry::embedded().unwrap();
    for (locale, source, matched, expected) in [
        ("EN_nz", "en", LocaleMatch::Exact, "OIA request"),
        ("en-US", "en", LocaleMatch::LanguageFallback, "OIA request"),
        (
            "fr-CA",
            "fr-FR",
            LocaleMatch::LanguageFallback,
            "demande d’accès à l’information (OIA)",
        ),
        ("ja-JP", "en", LocaleMatch::BaseFallback, "OIA request"),
        ("bad-💥", "en", LocaleMatch::InvalidFallback, "OIA request"),
    ] {
        let engine = LocalizationEngine::new(locale);
        let term = registry.lookup("nz-oia", &engine, Term::Request).unwrap();
        assert_eq!(term.source_locale.as_deref(), Some(source));
        assert_eq!(term.locale.matched, matched);
        assert_eq!(term.jurisdiction_id, "nz-oia");
        assert_eq!(term.text, expected);
    }
}

#[test]
fn unsupported_or_ambiguous_jurisdictions_fail_privately() {
    let registry = TerminologyRegistry::embedded().unwrap();
    for id in ["", "AU", "en-AU", "gb-scotland", "NZ-OIA", PRIVATE] {
        for locale in SUPPORTED_LOCALES {
            let error = registry
                .lookup(id, &LocalizationEngine::new(locale), Term::Request)
                .unwrap_err();
            assert_eq!(error, TerminologyError::UnknownJurisdiction);
            assert!(!format!("{error:?} / {error}").contains(PRIVATE));
        }
    }
}

#[test]
fn profile_authority_terms_do_not_collapse_departments_ministries_and_agencies() {
    let mut catalog = synthetic_catalog();
    let english = LocalizationEngine::new("en-GB");
    for (kind, expected) in [
        ("authority", "public authority"),
        ("agency", "agency"),
        ("department", "department"),
        ("ministry", "ministry"),
    ] {
        catalog["profiles"][0]["authority_term"] = json!(kind);
        let registry = TerminologyRegistry::from_json(&catalog.to_string()).unwrap();
        assert_eq!(
            registry
                .lookup("synthetic-law", &english, Term::Authority)
                .unwrap()
                .text,
            expected
        );
    }
}

#[test]
fn profile_values_are_interpolated_once_as_literal_unicode() {
    let mut catalog = synthetic_catalog();
    let literal = "Café / Cafe\u{301} / { $other } / $(never) / <tag>";
    catalog["profiles"][0]["request_abbreviation"] = json!(literal);
    catalog["profiles"][0]["law_name"] = json!(literal);
    let registry = TerminologyRegistry::from_json(&catalog.to_string()).unwrap();
    let english = LocalizationEngine::new("en-AU");
    assert!(
        registry.profile("nz-oia").is_none(),
        "Custom catalogs must not merge silently"
    );
    let rendered = registry
        .lookup("synthetic-law", &english, Term::Request)
        .unwrap();
    assert_eq!(rendered.catalog_version.revision, "synthetic.1");
    assert_eq!(
        rendered.message_bundle_version.unwrap().revision,
        "2026-11-14.1"
    );
    assert_eq!(rendered.text, format!("{literal} request"));
    assert_eq!(
        registry
            .lookup("synthetic-law", &english, Term::LawName)
            .unwrap()
            .text,
        literal
    );
}

#[test]
fn catalog_text_bounds_count_utf8_bytes_and_accept_the_exact_boundary() {
    for (field, limit) in [
        ("id", 64),
        ("law_name", 512),
        ("law_abbreviation", 256),
        ("request_abbreviation", 256),
    ] {
        let mut catalog = synthetic_catalog();
        let valid = if field == "id" {
            "a".repeat(limit)
        } else {
            "é".repeat(limit / 2)
        };
        catalog["profiles"][0][field] = json!(valid);
        assert!(
            TerminologyRegistry::from_json(&catalog.to_string()).is_ok(),
            "{field}"
        );
        catalog["profiles"][0][field] = json!(format!("{valid}x"));
        assert!(
            TerminologyRegistry::from_json(&catalog.to_string()).is_err(),
            "{field}"
        );
    }
    let mut catalog = synthetic_catalog();
    catalog["version"]["revision"] = json!("r".repeat(64));
    assert!(TerminologyRegistry::from_json(&catalog.to_string()).is_ok());
    catalog["version"]["revision"] = json!("r".repeat(65));
    assert!(TerminologyRegistry::from_json(&catalog.to_string()).is_err());
}

#[test]
fn malformed_unknown_and_empty_catalogs_fail_closed() {
    for text in ["", "{", "[]", PRIVATE] {
        assert_eq!(
            TerminologyRegistry::from_json(text).unwrap_err(),
            TerminologyError::InvalidCatalog
        );
    }
    let base = synthetic_catalog();
    let mut variants = Vec::new();
    let mut unknown = base.clone();
    unknown["unrecognized"] = json!(PRIVATE);
    variants.push(unknown);
    let mut empty = base.clone();
    empty["profiles"] = json!([]);
    variants.push(empty);
    let mut unknown_profile = base.clone();
    unknown_profile["profiles"][0]["unrecognized"] = json!(PRIVATE);
    variants.push(unknown_profile);
    let mut unknown_authority = base.clone();
    unknown_authority["profiles"][0]["authority_term"] = json!(PRIVATE);
    variants.push(unknown_authority);
    for variant in variants {
        let error = TerminologyRegistry::from_json(&variant.to_string()).unwrap_err();
        assert!(!format!("{error:?} / {error}").contains(PRIVATE));
    }
    assert!(TerminologyRegistry::from_json(&" ".repeat(256 * 1024 + 1)).is_err());
}

#[test]
fn catalog_versions_and_duplicate_profile_ids_are_rejected() {
    let mut catalog = synthetic_catalog();
    catalog["version"]["format_version"] = json!(2);
    assert_eq!(
        TerminologyRegistry::from_json(&catalog.to_string()).unwrap_err(),
        TerminologyError::UnsupportedVersion(2)
    );
    catalog["version"]["format_version"] = json!(1);
    for revision in ["", " ", "bad\nrevision"] {
        catalog["version"]["revision"] = json!(revision);
        assert!(TerminologyRegistry::from_json(&catalog.to_string()).is_err());
    }
    catalog = synthetic_catalog();
    let duplicate = catalog["profiles"][0].clone();
    catalog["profiles"].as_array_mut().unwrap().push(duplicate);
    assert_eq!(
        TerminologyRegistry::from_json(&catalog.to_string()).unwrap_err(),
        TerminologyError::DuplicateJurisdiction
    );
}

#[test]
fn malformed_profile_fields_and_source_references_fail_privately() {
    for (field, bad) in [
        ("id", ""),
        ("id", "NZ-OIA"),
        ("id", "../private"),
        ("country", "nz"),
        ("country", "NZL"),
        ("law_name", ""),
        ("law_name", "bad\u{2028}label"),
        ("law_abbreviation", "bad\u{2029}label"),
        ("request_abbreviation", "bad\u{1b}[31mlabel"),
        ("source_url", "file:///synthetic-private-terminology"),
        ("source_url", "http://example.invalid"),
        (
            "source_url",
            "https://synthetic-private-terminology@example.invalid",
        ),
        (
            "source_url",
            "https://example.invalid/?secret=synthetic-private-terminology",
        ),
        (
            "source_url",
            "https://example.invalid/#synthetic-private-terminology",
        ),
        ("source_checked_at", "not-a-date"),
    ] {
        let mut catalog = synthetic_catalog();
        catalog["profiles"][0][field] = json!(bad);
        let error = TerminologyRegistry::from_json(&catalog.to_string()).unwrap_err();
        assert!(!format!("{error:?} / {error}").contains(PRIVATE));
    }
}

#[test]
fn serialized_results_cannot_claim_review_or_legal_validation() {
    let registry = TerminologyRegistry::embedded().unwrap();
    let term = registry
        .lookup("fr-crpa", &LocalizationEngine::new("fr-FR"), Term::Request)
        .unwrap();
    let mut encoded = serde_json::to_value(&term).unwrap();
    assert_eq!(encoded["validation"], "rendered_unvalidated");
    encoded["validation"] = json!("legally_validated");
    assert!(serde_json::from_value::<fyi_core::i18n::terminology::LocalizedTerm>(encoded).is_err());
}

#[test]
fn registry_and_shared_bundles_support_concurrent_offline_lookup() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TerminologyRegistry>();
    let registry = std::sync::Arc::new(TerminologyRegistry::embedded().unwrap());
    let expected = registry
        .lookup("de-ifg", &LocalizationEngine::new("de-DE"), Term::Request)
        .unwrap();
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let registry = registry.clone();
            std::thread::spawn(move || {
                registry
                    .lookup("de-ifg", &LocalizationEngine::new("de-DE"), Term::Request)
                    .unwrap()
            })
        })
        .collect();
    for worker in workers {
        assert_eq!(worker.join().unwrap(), expected);
    }
}
