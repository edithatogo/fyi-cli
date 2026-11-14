//! Hand-authored rendering fixtures preserve existing translations, not legal approval.

use fyi_core::i18n::{LocalizationEngine, SUPPORTED_LOCALES};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenMessages {
    format_version: u32,
    bundle_revision: String,
    authority: String,
    locales: Vec<GoldenLocale>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenLocale {
    locale: String,
    salutation: String,
    closing: String,
    request_term: String,
    shared_message_source_locale: String,
    deadline_labels: Vec<GoldenDeadlineLabel>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenDeadlineLabel {
    days: usize,
    label: String,
}

fn goldens() -> GoldenMessages {
    serde_json::from_str(include_str!("fixtures/i18n/messages.json")).unwrap()
}

#[test]
fn locale_goldens_cover_supported_locales_in_contract_order() {
    let fixtures = goldens();
    let expected = ["en-NZ", "en-AU", "en-GB", "de-DE", "fr-FR", "es-ES"];
    assert_eq!(SUPPORTED_LOCALES, expected);
    assert_eq!(
        fixtures
            .locales
            .iter()
            .map(|fixture| fixture.locale.as_str())
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(fixtures.format_version, 1);
    assert_eq!(fixtures.bundle_revision, "2026-11-14.1");
    assert_eq!(fixtures.authority, "Te Whānau — Café");
    for fixture in fixtures.locales {
        assert_eq!(
            fixture
                .deadline_labels
                .iter()
                .map(|deadline| deadline.days)
                .collect::<Vec<_>>(),
            [0, 1, 2],
            "{} must cover zero, singular and plural labels",
            fixture.locale
        );
    }
}

#[test]
fn supported_messages_and_deadline_labels_match_hand_authored_goldens() {
    let fixtures = goldens();
    let variables = BTreeMap::from([("authority".into(), fixtures.authority.clone())]);
    for fixture in fixtures.locales {
        let engine = LocalizationEngine::try_new(&fixture.locale).unwrap();
        for (key, expected, source_locale) in [
            ("salutation", &fixture.salutation, &fixture.locale),
            (
                "closing",
                &fixture.closing,
                &fixture.shared_message_source_locale,
            ),
            (
                "request_term",
                &fixture.request_term,
                &fixture.shared_message_source_locale,
            ),
        ] {
            let rendered = engine.render_message(key, &variables).unwrap();
            assert_eq!(rendered.text, *expected, "{}: {key}", fixture.locale);
            assert_eq!(
                rendered.source_locale, *source_locale,
                "{}: {key} source",
                fixture.locale
            );
            assert_eq!(
                rendered.bundle_version.format_version,
                fixtures.format_version
            );
            assert_eq!(rendered.bundle_version.revision, fixtures.bundle_revision);
            assert_eq!(
                engine.render_message(key, &variables).unwrap(),
                rendered,
                "{}: {key} repeated rendering",
                fixture.locale
            );
        }
        for deadline in fixture.deadline_labels {
            assert_eq!(
                engine.deadline_label(deadline.days),
                deadline.label,
                "{}: {} days",
                fixture.locale,
                deadline.days
            );
        }
    }
}

#[test]
fn english_regions_inherit_shared_messages_without_losing_their_salutations() {
    let fixtures = goldens();
    let variables = BTreeMap::from([("authority".into(), fixtures.authority)]);
    for locale in ["en-NZ", "en-AU", "en-GB"] {
        let engine = LocalizationEngine::new(locale);
        assert_eq!(engine.resolution().chain, [locale, "en", "base"]);
        let salutation = engine.render_message("salutation", &variables).unwrap();
        assert_eq!(salutation.source_locale, locale);
        let closing = engine.render_message("closing", &BTreeMap::new()).unwrap();
        assert_eq!(closing.source_locale, "en");
        assert_eq!(closing.text, "Yours sincerely");
    }
}

#[test]
fn german_language_fallback_uses_the_german_golden_messages() {
    let fixtures = goldens();
    let german = fixtures
        .locales
        .iter()
        .find(|fixture| fixture.locale == "de-DE")
        .unwrap();
    let variables = BTreeMap::from([("authority".into(), fixtures.authority)]);
    let engine = LocalizationEngine::new("de-AT");
    assert_eq!(engine.resolution().chain, ["de-DE", "base"]);
    for (key, expected) in [
        ("salutation", &german.salutation),
        ("closing", &german.closing),
        ("request_term", &german.request_term),
    ] {
        let rendered = engine.render_message(key, &variables).unwrap();
        assert_eq!(rendered.text, *expected, "de-AT fallback: {key}");
        assert_eq!(rendered.source_locale, "de-DE");
    }
}
