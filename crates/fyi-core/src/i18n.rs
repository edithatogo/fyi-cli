use crate::jurisdiction::Instance;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocaleBundle {
    pub locale: String,
    translations: BTreeMap<String, String>,
}

impl LocaleBundle {
    pub fn for_locale(locale: &str) -> Self {
        let locale = locale.to_ascii_lowercase();
        let mut translations = BTreeMap::new();
        if locale.starts_with("en") {
            translations.insert("salutation".to_string(), "Dear {authority}".to_string());
            translations.insert("request_term".to_string(), "request".to_string());
            translations.insert("closing".to_string(), "Yours sincerely".to_string());
            translations.insert("working_days".to_string(), "working days".to_string());
            translations.insert("working_day".to_string(), "working day".to_string());
            translations.insert("deadline".to_string(), "Statutory deadline".to_string());
        } else {
            translations.insert(
                "salutation".to_string(),
                "Estimado/a {authority}".to_string(),
            );
            translations.insert("request_term".to_string(), "solicitud".to_string());
            translations.insert("closing".to_string(), "Atentamente".to_string());
            translations.insert("working_days".to_string(), "días hábiles".to_string());
            translations.insert("working_day".to_string(), "día hábil".to_string());
            translations.insert("deadline".to_string(), "Plazo legal".to_string());
        }

        Self {
            locale,
            translations,
        }
    }

    pub fn translate(&self, key: &str, fallback: &str) -> String {
        self.translations
            .get(key)
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct LocalizationEngine {
    locale: String,
    bundle: LocaleBundle,
}

impl LocalizationEngine {
    pub fn new(locale: &str) -> Self {
        let bundle = LocaleBundle::for_locale(locale);
        Self {
            locale: locale.to_string(),
            bundle,
        }
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn render_request_template(&self, authority_name: &str, law_name: &str) -> String {
        let salutation = self
            .bundle
            .translate("salutation", "Dear {authority}")
            .replace("{authority}", authority_name);
        format!(
            "{}\n\nUnder {} I am requesting information for this {}.\n\n{}",
            salutation,
            law_name,
            self.bundle.translate("request_term", "request"),
            self.bundle.translate("closing", "Yours sincerely")
        )
    }

    pub fn deadline_label(&self, days: usize) -> String {
        let singular = self.bundle.translate("working_day", "working day");
        let plural = self.bundle.translate("working_days", "working days");
        let noun = if days == 1 { singular } else { plural };
        format!("{} {}", days, noun)
    }

    pub fn render_request_template_with_instance(
        &self,
        authority_name: &str,
        instance: &Instance,
    ) -> String {
        let salutation = self
            .bundle
            .translate("salutation", "Dear {authority}")
            .replace("{authority}", authority_name);
        let law_name = instance.foi_law.law_name.as_str();
        let citation = instance.foi_law.citation.as_deref().unwrap_or(law_name);
        let request_term = instance.foi_law.request_term.as_str();
        let appeal_body = instance
            .foi_law
            .appeal_body
            .as_deref()
            .map(|body| format!("If this request is refused, I may seek review by {}.", body))
            .unwrap_or_default();
        format!(
            "{}\n\nUnder {} ({}) I am making this {}.{}",
            salutation,
            law_name,
            citation,
            request_term,
            if appeal_body.is_empty() {
                String::new()
            } else {
                format!("\n\n{}", appeal_body)
            }
        )
    }

    pub fn add_working_days(&self, start: NaiveDate, days: usize) -> NaiveDate {
        let mut current = start;
        let mut remaining = days;
        while remaining > 0 {
            current = current.succ_opt().unwrap_or(current);
            if is_weekday(current) {
                remaining -= 1;
            }
        }
        current
    }
}

fn is_weekday(day: NaiveDate) -> bool {
    matches!(day.weekday().num_days_from_monday(), 0..=4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jurisdiction::InstanceRegistry;

    #[test]
    fn english_locale_uses_english_terms() {
        let engine = LocalizationEngine::new("en-NZ");
        assert_eq!(engine.deadline_label(2), "2 working days");
        assert!(engine
            .render_request_template("Ministry", "OIA")
            .contains("Dear Ministry"));
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
}
