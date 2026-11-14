use fyi_core::i18n::LocalizationEngine;

#[test]
fn unknown_language_falls_back_to_english_instead_of_spanish() {
    let engine = LocalizationEngine::new("ja-JP");
    assert_eq!(engine.deadline_label(2), "2 working days");
    assert!(engine.render_request_template("Council", "Act").starts_with("Dear Council"));
}

#[test]
fn normalized_new_zealand_identifier_preserves_its_greeting() {
    let engine = LocalizationEngine::new("EN_nz");
    assert!(engine.render_request_template("Council", "Act").starts_with("Kia ora Council"));
}

#[test]
fn unsupported_english_regions_keep_a_neutral_english_greeting() {
    for locale in ["en-IE", "en-US"] {
        let engine = LocalizationEngine::new(locale);
        assert!(engine.render_request_template("Council", "Act").starts_with("Dear Council"));
    }
}
