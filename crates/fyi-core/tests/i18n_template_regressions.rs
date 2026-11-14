use fyi_core::i18n::LocalizationEngine;
use fyi_core::jurisdiction::InstanceRegistry;

#[test]
fn french_legacy_scaffold_has_a_french_body_and_review_paragraph() {
    let registry = InstanceRegistry::embedded().unwrap();
    let engine = LocalizationEngine::new("fr-FR");
    let generic = engine.render_request_template("CADA", "Loi exemple");
    assert!(generic.contains("demande des informations"));
    assert!(!generic.contains("Under"));
    let instance = registry.get("fr-cada").unwrap();
    let letter = engine.render_request_template_with_instance("CADA", instance);
    assert!(letter.contains("je présente cette"));
    assert!(!letter.contains("I am making"));
}
