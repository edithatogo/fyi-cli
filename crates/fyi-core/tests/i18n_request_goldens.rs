//! Independently authored complete-letter expectations, not legally approved requests.

use fyi_core::i18n::{
    LetterRegister, LocaleMatch, LocalizationEngine, OutputFormat, RequestTemplateInput,
    TemplateVariant, ValidationStatus, SUPPORTED_LOCALES,
};
use fyi_core::jurisdiction::InstanceRegistry;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenRequests {
    format_version: u32,
    bundle_revision: String,
    input: GoldenInput,
    requests: Vec<GoldenRequest>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenInput {
    authority: String,
    requester: String,
    title: String,
    body: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenRequest {
    locale: String,
    jurisdiction_id: String,
    title: String,
    body: String,
    message_sources: BTreeMap<String, String>,
}

fn goldens() -> GoldenRequests {
    serde_json::from_str(include_str!("fixtures/i18n/requests.json")).unwrap()
}

#[test]
fn complete_letter_goldens_cover_the_six_locale_and_instance_pairs_in_order() {
    let fixtures = goldens();
    assert_eq!(
        SUPPORTED_LOCALES,
        ["en-NZ", "en-AU", "en-GB", "de-DE", "fr-FR", "es-ES"]
    );
    assert_eq!(
        fixtures
            .requests
            .iter()
            .map(|request| (request.locale.as_str(), request.jurisdiction_id.as_str()))
            .collect::<Vec<_>>(),
        [
            ("en-NZ", "nz-fyi"),
            ("en-AU", "au-rtk"),
            ("en-GB", "uk-wdtk"),
            ("de-DE", "de-fds"),
            ("fr-FR", "fr-cada"),
            ("es-ES", "es-tdas"),
        ]
    );
    assert_eq!(fixtures.format_version, 1);
    assert_eq!(fixtures.bundle_revision, "2026-11-14.2");
    assert_eq!(fixtures.input.authority, "Example Authority / Te Whānau");
    assert_eq!(fixtures.input.requester, "Anaïs Ngā");
    assert_eq!(fixtures.input.title, "Budget 2026 — Café");
    assert_eq!(
        fixtures.input.body,
        "Please provide the 2026 budget.\nInclude annexes."
    );
    for request in fixtures.requests {
        assert_eq!(
            request
                .message_sources
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "closing",
                "legal_citation",
                "request_body",
                "request_title",
                "salutation",
            ],
            "{}: provenance fixture coverage",
            request.locale
        );
    }
}

#[test]
fn complete_requests_match_hand_authored_text_and_provenance_without_approval() {
    let fixtures = goldens();
    let registry = InstanceRegistry::embedded().unwrap();
    for expected in fixtures.requests {
        let instance = registry.get(&expected.jurisdiction_id).unwrap();
        assert_eq!(instance.locale, expected.locale);
        let input = RequestTemplateInput::new(
            &fixtures.input.authority,
            &fixtures.input.requester,
            &fixtures.input.title,
            &fixtures.input.body,
            instance,
        );
        assert_eq!(input.variant, TemplateVariant::Request);
        assert_eq!(input.register, LetterRegister::Formal);
        assert!(input.date.is_none());
        assert!(input.request_reference.is_none());
        assert!(input.review.is_none());

        let engine = LocalizationEngine::new(&expected.locale);
        let rendered = engine
            .render_request(&input, OutputFormat::PlainText)
            .unwrap();
        assert_eq!(rendered.title, expected.title, "{}: title", expected.locale);
        assert_eq!(
            rendered.body, expected.body,
            "{}: full body",
            expected.locale
        );
        assert_eq!(rendered.jurisdiction_id, expected.jurisdiction_id);
        assert_eq!(rendered.message_sources, expected.message_sources);
        assert_eq!(rendered.locale.requested, expected.locale);
        assert_eq!(rendered.locale.resolved, expected.locale);
        assert_eq!(rendered.locale.matched, LocaleMatch::Exact);
        assert_eq!(
            rendered.bundle_version.format_version,
            fixtures.format_version
        );
        assert_eq!(rendered.bundle_version.revision, fixtures.bundle_revision);
        assert_eq!(rendered.variant, TemplateVariant::Request);
        assert_eq!(rendered.register, LetterRegister::Formal);
        assert_eq!(rendered.format, OutputFormat::PlainText);
        assert_eq!(rendered.validation, ValidationStatus::RenderedUnvalidated);

        let repeated = engine
            .render_request(&input, OutputFormat::PlainText)
            .unwrap();
        assert_eq!(rendered.title.as_bytes(), repeated.title.as_bytes());
        assert_eq!(rendered.body.as_bytes(), repeated.body.as_bytes());
        assert_eq!(
            serde_json::to_vec(&rendered).unwrap(),
            serde_json::to_vec(&repeated).unwrap(),
            "{}: full serialized contract must be deterministic",
            expected.locale
        );
    }
}
