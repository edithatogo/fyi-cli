use fyi_core::i18n::{
    I18nError, LetterRegister, LocalizationEngine, OutputFormat, RequestTemplateInput,
    ReviewInformation, TemplateVariant, ValidationStatus, SUPPORTED_LOCALES,
};
use fyi_core::jurisdiction::InstanceRegistry;

#[test]
fn typed_request_carries_rendering_and_legal_provenance_without_approval() {
    let registry = InstanceRegistry::embedded().unwrap();
    let instance = registry.get("nz-fyi").unwrap();
    let mut input = RequestTemplateInput::new("Example Council", "Anaïs Ngā", "Budget & Café", "Please supply the budget.\nInclude annexes.", instance);
    input.date = Some("14 November 2026");
    input.review = ReviewInformation::from_instance(instance);
    let rendered = LocalizationEngine::new("en-NZ").render_request(&input, OutputFormat::PlainText).unwrap();
    assert_eq!(rendered.title, input.title);
    assert!(rendered.body.starts_with("Kia ora Example Council\n\nDate: 14 November 2026\n\n"));
    assert!(rendered.body.contains(input.body));
    assert!(rendered.body.contains(&instance.foi_law.law_name));
    assert!(rendered.body.ends_with("Yours sincerely\nAnaïs Ngā"));
    assert_eq!(rendered.jurisdiction_id, "nz-fyi");
    assert_eq!(rendered.validation, ValidationStatus::RenderedUnvalidated);
    assert_eq!(rendered.message_sources["salutation"], "en-NZ");
    assert_eq!(rendered.message_sources["request_body"], "en");
    assert_eq!(rendered.bundle_version.format_version, 1);
}

#[test]
fn optional_fields_do_not_leave_dangling_sections_or_duplicate_citations() {
    let registry = InstanceRegistry::embedded().unwrap();
    let mut instance = registry.get("nz-fyi").unwrap().clone();
    instance.foi_law.citation = None;
    instance.foi_law.appeal_body = None;
    let input = RequestTemplateInput::new("Council", "Ana", "Budget", "Please supply records.", &instance);
    assert!(ReviewInformation::from_instance(&instance).is_none());
    let letter = LocalizationEngine::new("en-GB").render_request(&input, OutputFormat::PlainText).unwrap();
    assert!(!letter.body.contains("()"));
    assert!(!letter.body.contains("\n\n\n"));
    assert!(!letter.body.contains("Review contact"));
    assert!(!letter.body.contains("Date:"));
    assert_eq!(letter.body.matches(&instance.foi_law.law_name).count(), 1);
}

#[test]
fn missing_required_fields_and_multiline_headers_fail_closed() {
    let registry = InstanceRegistry::embedded().unwrap();
    let instance = registry.get("nz-fyi").unwrap();
    let engine = LocalizationEngine::new("en-NZ");
    for field in ["authority", "requester", "title", "body"] {
        let mut input = RequestTemplateInput::new("Council", "Ana", "Budget", "Records", instance);
        match field { "authority" => input.authority = " ", "requester" => input.requester = "", "title" => input.title = "", _ => input.body = "\n\t" }
        assert!(matches!(engine.render_request(&input, OutputFormat::PlainText), Err(I18nError::MissingField { .. })));
    }
    let mut input = RequestTemplateInput::new("Council", "Ana", "Budget\r\nBcc: someone", "Records", instance);
    assert!(matches!(engine.render_request(&input, OutputFormat::PlainText), Err(I18nError::InvalidField { .. })));
    input.title = "Budget";
    input.date = Some("");
    assert!(engine.render_request(&input, OutputFormat::PlainText).is_err());
    input.date = None;
    input.review = Some(ReviewInformation { authority: "", instructions: None });
    assert!(engine.render_request(&input, OutputFormat::PlainText).is_err());
}

#[test]
fn all_variants_and_registers_render_deterministically_in_every_locale() {
    let registry = InstanceRegistry::embedded().unwrap();
    let instance = registry.get("nz-fyi").unwrap();
    for locale in SUPPORTED_LOCALES {
        for variant in [TemplateVariant::Request, TemplateVariant::FollowUp, TemplateVariant::InternalReview, TemplateVariant::Appeal] {
            for register in [LetterRegister::Formal, LetterRegister::Informal] {
                let mut input = RequestTemplateInput::new("Council — 東京", "Anaïs Ngā", "Café / Cafe\u{301}", "{ $authority } <script> & 🧬", instance);
                input.variant = variant;
                input.register = register;
                input.request_reference = Some("REQ-42");
                input.review = Some(ReviewInformation { authority: "Example reviewer", instructions: Some("Instructions supplied by the requester.") });
                let engine = LocalizationEngine::new(locale);
                let first = engine.render_request(&input, OutputFormat::PlainText).unwrap();
                assert_eq!(engine.render_request(&input, OutputFormat::PlainText).unwrap(), first);
                assert!(first.body.contains(input.body));
                assert!(first.title.contains(input.title));
                assert_eq!(first.validation, ValidationStatus::RenderedUnvalidated);
                assert_eq!(first.jurisdiction_id, instance.id);
                for source in first.message_sources.values() {
                    if !locale.starts_with("en-") { assert_eq!(source, locale, "untranslated message in {locale}"); }
                }
            }
        }
    }
}

#[test]
fn variants_require_a_reference_and_appeals_require_a_supplied_reviewer() {
    let registry = InstanceRegistry::embedded().unwrap();
    let instance = registry.get("nz-fyi").unwrap();
    let engine = LocalizationEngine::new("en-NZ");
    for variant in [TemplateVariant::FollowUp, TemplateVariant::InternalReview, TemplateVariant::Appeal] {
        let mut input = RequestTemplateInput::new("Council", "Ana", "Budget", "Records", instance);
        input.variant = variant;
        assert!(engine.render_request(&input, OutputFormat::PlainText).is_err());
        input.request_reference = Some("REQ-42");
        if variant == TemplateVariant::Appeal {
            assert!(engine.render_request(&input, OutputFormat::PlainText).is_err());
            input.review = Some(ReviewInformation { authority: "Supplied reviewer", instructions: None });
        }
        assert!(engine.render_request(&input, OutputFormat::PlainText).is_ok());
    }
}

#[test]
fn html_text_escapes_all_rendered_content_once_and_plain_text_preserves_unicode() {
    let registry = InstanceRegistry::embedded().unwrap();
    let instance = registry.get("nz-fyi").unwrap();
    let input = RequestTemplateInput::new("Council <x>", "A & B", "\"Café\" & 'Caf\u{e9}'", "<script> &lt;x&gt; { $body } Cafe\u{301}", instance);
    let engine = LocalizationEngine::new("en-NZ");
    let plain = engine.render_request(&input, OutputFormat::PlainText).unwrap();
    let html = engine.render_request(&input, OutputFormat::HtmlText).unwrap();
    assert!(plain.body.contains(input.body));
    assert!(html.body.contains("&lt;script&gt; &amp;lt;x&amp;gt; { $body } Cafe\u{301}"));
    assert!(html.title.contains("&quot;Café&quot; &amp; &#39;Café&#39;"));
    assert!(!html.body.contains("<script>"));
    assert_eq!(html.format, OutputFormat::HtmlText);
}

#[test]
fn fallback_changes_language_without_changing_legal_inputs() {
    let registry = InstanceRegistry::embedded().unwrap();
    let instance = registry.get("de-fds").unwrap();
    let input = RequestTemplateInput::new("Council", "Ana", "Budget", "Records", instance);
    let english = LocalizationEngine::new("en-US").render_request(&input, OutputFormat::PlainText).unwrap();
    let unknown = LocalizationEngine::new("zz-ZZ").render_request(&input, OutputFormat::PlainText).unwrap();
    assert_eq!(english.body, unknown.body);
    assert!(english.body.starts_with("Dear Council"));
    assert!(english.body.contains(&instance.foi_law.law_name));
    assert_eq!(english.jurisdiction_id, instance.id);
}
