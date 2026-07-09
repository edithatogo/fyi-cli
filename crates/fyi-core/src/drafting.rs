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

/// Polished letter returned by an LLM (or mock) drafting path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftedLetter {
    pub title: String,
    pub body: String,
    /// True when an LLM client produced the body; false for template-only.
    pub llm_assisted: bool,
}

/// Provider-agnostic LLM client for AI-assisted drafting.
///
/// Implementations may call remote APIs; tests should use [`MockLlmClient`].
/// The trait is synchronous so unit tests need no async runtime; async wrappers
/// can call these methods from `spawn_blocking` or own async traits later.
pub trait LlmClient: Send + Sync {
    /// Complete a drafting prompt into letter body text.
    fn complete(&self, system: &str, user_prompt: &str) -> Result<String, String>;
}

/// Deterministic mock LLM used in unit tests (no network).
#[derive(Debug, Clone, Default)]
pub struct MockLlmClient {
    /// Optional fixed response; when `None`, echoes a polished wrapper around the prompt.
    pub fixed_response: Option<String>,
    /// When true, `complete` returns an error.
    pub fail: bool,
}

impl MockLlmClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_response(response: impl Into<String>) -> Self {
        Self {
            fixed_response: Some(response.into()),
            fail: false,
        }
    }

    pub fn always_fail() -> Self {
        Self {
            fixed_response: None,
            fail: true,
        }
    }
}

impl LlmClient for MockLlmClient {
    fn complete(&self, system: &str, user_prompt: &str) -> Result<String, String> {
        if self.fail {
            return Err("mock LLM failure".into());
        }
        if let Some(fixed) = &self.fixed_response {
            return Ok(fixed.clone());
        }
        Ok(format!(
            "[Mock LLM draft]\nSystem: {}\n---\n{}",
            system.lines().next().unwrap_or_default(),
            user_prompt
        ))
    }
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

    /// Build a jurisdiction-templated letter without calling an LLM.
    pub fn draft_from_template(
        &self,
        request: &DraftRequest,
        instance: &Instance,
        locale: &str,
    ) -> DraftedLetter {
        let localization = LocalizationEngine::new(locale);
        let opening =
            localization.render_request_template_with_instance(&request.authority_name, instance);
        let request_term = instance.foi_law.request_term.as_str();
        let body = format!(
            "{}\n\nSubject: {}\n\nBackground:\n{}\n\nRequested information:\n{}\n\n{}",
            opening,
            request.subject,
            request.details,
            request.requested_information,
            localization
                .render_request_template(&request.authority_name, &instance.foi_law.law_name)
                .lines()
                .last()
                .unwrap_or("Yours sincerely")
        );
        DraftedLetter {
            title: format!("{} draft for {}", request_term, request.authority_name),
            body,
            llm_assisted: false,
        }
    }
}

const DRAFT_SYSTEM_PROMPT: &str = "You are an assistant that drafts clear, polite freedom-of-information request letters. Use the jurisdiction scaffold and legal basis provided. Do not invent legal citations.";

/// Draft a request letter using jurisdiction templates and an optional LLM client.
///
/// When `llm` is `Some`, the template prompt is sent to the client and the
/// completion becomes the letter body. When `None`, a pure template draft is
/// returned. LLM errors fall back to the template draft with `llm_assisted = false`.
pub fn draft_request_with_llm(
    engine: &DraftingEngine,
    request: &DraftRequest,
    instance: &Instance,
    locale: &str,
    llm: Option<&dyn LlmClient>,
) -> DraftedLetter {
    let template = engine.draft_from_template(request, instance, locale);
    let Some(client) = llm else {
        return template;
    };

    let prompt = engine.build_prompt(request, instance, locale);
    match client.complete(DRAFT_SYSTEM_PROMPT, &prompt.body) {
        Ok(body) => DraftedLetter {
            title: prompt.title,
            body,
            llm_assisted: true,
        },
        Err(_) => template,
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

    #[test]
    fn draft_without_llm_uses_template() {
        let registry = InstanceRegistry::embedded().unwrap();
        let instance = registry.get("nz-fyi").unwrap();
        let engine = DraftingEngine;
        let request = DraftRequest::new(
            "Ministry of Health",
            "Waiting lists",
            "Background on elective surgery.",
            "Monthly statistics for 2025.",
        );

        let letter = draft_request_with_llm(&engine, &request, instance, "en-NZ", None);
        assert!(!letter.llm_assisted);
        assert!(letter.body.contains("Official Information Act"));
        assert!(letter.body.contains("Waiting lists"));
        assert!(letter.body.contains("Monthly statistics"));
    }

    #[test]
    fn draft_with_mock_llm_sets_assisted_flag() {
        let registry = InstanceRegistry::embedded().unwrap();
        let instance = registry.get("nz-fyi").unwrap();
        let engine = DraftingEngine;
        let request = DraftRequest::new(
            "Ministry of Health",
            "Waiting lists",
            "Background on elective surgery.",
            "Monthly statistics for 2025.",
        );
        let mock = MockLlmClient::with_response("Polished OIA letter body.");

        let letter = draft_request_with_llm(&engine, &request, instance, "en-NZ", Some(&mock));
        assert!(letter.llm_assisted);
        assert_eq!(letter.body, "Polished OIA letter body.");
    }

    #[test]
    fn draft_falls_back_when_llm_fails() {
        let registry = InstanceRegistry::embedded().unwrap();
        let instance = registry.get("uk-wdtk").unwrap();
        let engine = DraftingEngine;
        let request = DraftRequest::new("MoJ", "Subject", "Details", "Info");
        let mock = MockLlmClient::always_fail();

        let letter = draft_request_with_llm(&engine, &request, instance, "en-GB", Some(&mock));
        assert!(!letter.llm_assisted);
        assert!(letter.body.contains("Freedom of Information Act"));
    }
}
