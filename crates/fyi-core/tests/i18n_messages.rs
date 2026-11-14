use fyi_core::i18n::{
    BundleVersion, I18nError, LocaleMatch, LocalizationEngine, MessageBundle, SUPPORTED_LOCALES,
};
use std::collections::BTreeMap;

fn version() -> BundleVersion {
    BundleVersion {
        format_version: 1,
        revision: "test.1".into(),
    }
}

#[test]
fn supported_identifiers_are_explicit_and_normalize_separators_and_case() {
    assert_eq!(
        SUPPORTED_LOCALES,
        ["en-NZ", "en-AU", "en-GB", "de-DE", "fr-FR", "es-ES"]
    );
    for locale in SUPPORTED_LOCALES {
        let engine = LocalizationEngine::try_new(locale).unwrap();
        assert_eq!(engine.resolution().resolved, locale);
        assert_eq!(engine.resolution().matched, LocaleMatch::Exact);
    }
    let engine = LocalizationEngine::try_new("EN_nz").unwrap();
    assert_eq!(engine.locale(), "EN_nz");
    assert_eq!(engine.resolution().normalized.as_deref(), Some("en-NZ"));
    assert_eq!(engine.resolution().chain, ["en-NZ", "en", "base"]);
}

#[test]
fn strict_selection_distinguishes_invalid_and_unsupported_locales() {
    assert!(matches!(
        LocalizationEngine::try_new(""),
        Err(I18nError::InvalidLocale)
    ));
    assert!(matches!(
        LocalizationEngine::try_new("en-💥"),
        Err(I18nError::InvalidLocale)
    ));
    assert!(matches!(
        LocalizationEngine::try_new("ja-JP"),
        Err(I18nError::UnsupportedLocale)
    ));
    let invalid = LocalizationEngine::new("en-💥");
    assert_eq!(invalid.resolution().matched, LocaleMatch::InvalidFallback);
    let unsupported = LocalizationEngine::new("ja-JP");
    assert_eq!(unsupported.resolution().matched, LocaleMatch::BaseFallback);
    assert_eq!(unsupported.resolution().chain, ["en", "base"]);
    for locale in ["en", "en-US", "en-IE"] {
        let engine = LocalizationEngine::new(locale);
        assert_eq!(engine.resolution().matched, LocaleMatch::LanguageFallback);
        assert_eq!(engine.resolution().resolved, "en");
    }
    for (locale, resolved) in [("de-AT", "de-DE"), ("fr-CA", "fr-FR"), ("es-MX", "es-ES")] {
        assert_eq!(
            LocalizationEngine::new(locale).resolution().resolved,
            resolved
        );
    }
}

#[test]
fn message_fallback_records_source_and_bundle_revision() {
    let engine = LocalizationEngine::new("en-NZ");
    let rendered = engine.render_message("closing", &BTreeMap::new()).unwrap();
    assert_eq!(rendered.text, "Yours sincerely");
    assert_eq!(rendered.source_locale, "en");
    assert_eq!(rendered.bundle_version.format_version, 1);
    assert!(!rendered.bundle_version.revision.is_empty());
    assert!(matches!(
        engine.render_message("not-a-message", &BTreeMap::new()),
        Err(I18nError::MissingMessage { .. })
    ));
    assert!(engine
        .render_message("salutation", &BTreeMap::new())
        .is_err());
}

#[test]
fn bundles_reject_invalid_versions_syntax_duplicates_and_missing_messages() {
    let mut bad_version = version();
    bad_version.format_version = 2;
    assert!(matches!(
        MessageBundle::from_ftl("en", bad_version, "hello = Hello"),
        Err(I18nError::UnsupportedBundleVersion(2))
    ));
    let mut empty_revision = version();
    empty_revision.revision = "  ".into();
    assert!(MessageBundle::from_ftl("en", empty_revision, "hello = Hello").is_err());
    assert!(MessageBundle::from_ftl("💥", version(), "hello = Hello").is_err());
    assert!(MessageBundle::from_ftl("en", version(), "hello = {").is_err());
    assert!(MessageBundle::from_ftl("en", version(), "hello = One\nhello = Two").is_err());
    let bundle = MessageBundle::from_ftl(
        "en",
        version(),
        "hello = Hello\nattribute-only =\n    .label = Label",
    )
    .unwrap();
    bundle.require_messages(&["hello"]).unwrap();
    assert!(bundle.require_messages(&["absent"]).is_err());
    assert!(bundle.require_messages(&["attribute-only"]).is_err());
    assert!(bundle.render("attribute-only", &BTreeMap::new()).is_err());
}

#[test]
fn inserted_values_are_literal_and_unicode_is_preserved() {
    let bundle = MessageBundle::from_ftl(
        "en",
        version(),
        "hello = { $name } / { $name }\nbraces = { \"{\" }literal{ \"}\" }",
    )
    .unwrap();
    for value in [
        "{ $name }",
        "{{code}}",
        "<script>alert('x')</script>&",
        "$(echo never)",
        "Te Whānau — Café / Cafe\u{301} / 東京 🧬",
    ] {
        let args = BTreeMap::from([("name".into(), value.into())]);
        assert_eq!(
            bundle.render("hello", &args).unwrap(),
            format!("{value} / {value}")
        );
    }
    assert_eq!(
        bundle.render("braces", &BTreeMap::new()).unwrap(),
        "{literal}"
    );
    assert!(bundle.render("hello", &BTreeMap::new()).is_err());
}

#[test]
fn fluent_expressions_cannot_call_arbitrary_functions() {
    let bundle = MessageBundle::from_ftl("en", version(), "unsafe = { EXEC(\"never\") }").unwrap();
    assert!(bundle.render("unsafe", &BTreeMap::new()).is_err());
    let cyclic = MessageBundle::from_ftl(
        "en",
        version(),
        "first = { second }\nsecond = { first }\nmissing = { absent }",
    )
    .unwrap();
    assert!(cyclic.render("first", &BTreeMap::new()).is_err());
    assert!(cyclic.render("missing", &BTreeMap::new()).is_err());
}

#[test]
fn rendering_is_deterministic_and_never_reinterprets_arguments() {
    let bundle =
        MessageBundle::from_ftl("fr", version(), "message = { $first } — { $second }").unwrap();
    let first = BTreeMap::from([
        ("first".into(), "{ $second }".into()),
        ("second".into(), "François".into()),
    ]);
    let second = BTreeMap::from([
        ("second".into(), "François".into()),
        ("first".into(), "{ $second }".into()),
    ]);
    assert_eq!(
        bundle.render("message", &first).unwrap(),
        "{ $second } — François"
    );
    assert_eq!(
        bundle.render("message", &first).unwrap(),
        bundle.render("message", &second).unwrap()
    );
}

#[test]
fn legacy_bundle_serialization_and_explicit_fallback_remain_usable() {
    use fyi_core::i18n::LocaleBundle;
    let bundle = LocaleBundle::for_locale("en-NZ");
    assert_eq!(bundle.locale, "en-nz");
    assert_eq!(
        bundle.translate("salutation", "missing"),
        "Kia ora {authority}"
    );
    assert_eq!(
        bundle.translate("absent", "explicit fallback"),
        "explicit fallback"
    );
    let json = serde_json::to_string(&bundle).unwrap();
    let copy: LocaleBundle = serde_json::from_str(&json).unwrap();
    assert_eq!(copy, bundle);
    let historical: LocaleBundle = serde_json::from_str(
        r#"{"locale":"en-nz","translations":{"salutation":"Kia ora {authority}"}}"#,
    )
    .unwrap();
    assert_eq!(
        historical.translate("salutation", "missing"),
        "Kia ora {authority}"
    );
}

#[test]
fn cached_resources_are_safe_for_concurrent_cloned_engines() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MessageBundle>();
    assert_send_sync::<LocalizationEngine>();
    let engine = LocalizationEngine::new("fr-FR");
    let other = engine.clone();
    let handle = std::thread::spawn(move || other.deadline_label(2));
    assert_eq!(engine.deadline_label(2), handle.join().unwrap());
}
