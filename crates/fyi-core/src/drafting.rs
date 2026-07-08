use crate::i18n::LocalizationEngine;
use crate::jurisdiction::Instance;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftRequest {
    pub authority_name: String,
    pub subject: String,
    pub details: String,
    pub requested_information: String,
}

impl DraftRequest {
    pub fn new(
        authority_name: &str,
        subject: &str,
        details: &str,
        requested_information: &str,
    ) -> Self {
        Self {
            authority_name: authority_name.to_string(),
            subject: subject.to_string(),
            details: details.to_string(),
            requested_information: requested_information.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftPrompt {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Default)]
pub struct DraftingEngine;

impl DraftingEngine {
    pub fn build_prompt(
        &self,
        request: &DraftRequest,
        instance: &Instance,
        locale: &str,
    ) -> DraftPrompt {
        let localization = LocalizationEngine::new(locale);
        let base_letter =
            localization.render_request_template_with_instance(&request.authority_name, instance);

        let law_name = instance.foi_law.law_name.as_str();
        let citation = instance.foi_law.citation.as_deref().unwrap_or(law_name);
        let request_term = instance.foi_law.request_term.as_str();
        let deadline_days = instance
            .foi_law
            .statutory_deadline_days
            .unwrap_or(20)
            .to_string();
        let appeal_body = instance
            .foi_law
            .appeal_body
            .as_deref()
            .unwrap_or("the relevant review body");

        DraftPrompt {
            title: format!("{} draft for {}", request_term, request.authority_name),
            body: format!(
                "Draft a polished {} letter for {} in locale {}.\n\nUse the following opening as the scaffold:\n{}\n\nAdditional guidance:\n- Keep the language clear and specific.\n- Reference the legal basis {} ({}) and note a statutory deadline of {} days.\n- If the request is refused, mention review by {}.\n\nSubject: {}\n\nBackground: {}\n\nRequested information: {}",
                request_term,
                request.authority_name,
                locale,
                base_letter,
                law_name,
                citation,
                deadline_days,
                appeal_body,
                request.subject,
                request.details,
                request.requested_information,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jurisdiction::InstanceRegistry;

    #[test]
    fn australia_prompt_includes_foi_act_context_and_appeal_body() {
        let registry = InstanceRegistry::embedded().unwrap();
        let instance = registry.get("au-rtk").unwrap();
        let engine = DraftingEngine;
        let request = DraftRequest::new(
            "Department of Finance",
            "Budget records",
            "A request for procurement records and procurement decisions.",
            "Copies of procurement contracts and corresponding briefing notes.",
        );

        let prompt = engine.build_prompt(&request, instance, "en-AU");

        assert!(prompt.title.contains("FOI request"));
        assert!(prompt.body.contains("Freedom of Information Act"));
        assert!(prompt.body.contains("FOI Act"));
        assert!(prompt
            .body
            .contains("Office of the Australian Information Commissioner (OAIC)"));
        assert!(prompt.body.contains("Budget records"));
    }

    #[test]
    fn uk_prompt_includes_foia_2000_context_and_ico_review_body() {
        let registry = InstanceRegistry::embedded().unwrap();
        let instance = registry.get("uk-wdtk").unwrap();
        let engine = DraftingEngine;
        let request = DraftRequest::new(
            "Ministry of Justice",
            "Policy documents",
            "A request for the policy guidance used in the most recent review.",
            "Copies of the review notes and implementation guidance.",
        );

        let prompt = engine.build_prompt(&request, instance, "en-GB");

        assert!(prompt.title.contains("FOI request"));
        assert!(prompt.body.contains("Freedom of Information Act"));
        assert!(prompt.body.contains("FOIA 2000"));
        assert!(prompt
            .body
            .contains("Information Commissioner's Office (ICO)"));
        assert!(prompt.body.contains("Policy documents"));
    }
}
