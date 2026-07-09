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

/// Heuristic quality dimensions for a draft prompt or letter body (0.0–1.0).
#[derive(Debug, Clone, PartialEq)]
pub struct DraftQualityScore {
    /// Plain-language clarity (length balance, sentence structure cues).
    pub clarity: f64,
    /// How specific the request appears (dates, document types, named entities).
    pub specificity: f64,
    /// Presence of legal basis / statute / FOI terminology references.
    pub legal_refs: f64,
    /// Weighted overall score in 0.0–1.0.
    pub overall: f64,
}

impl DraftQualityScore {
    fn clamp_unit(v: f64) -> f64 {
        v.clamp(0.0, 1.0)
    }

    /// Combine dimension scores with fixed weights.
    pub fn from_dimensions(clarity: f64, specificity: f64, legal_refs: f64) -> Self {
        let clarity = Self::clamp_unit(clarity);
        let specificity = Self::clamp_unit(specificity);
        let legal_refs = Self::clamp_unit(legal_refs);
        let overall = Self::clamp_unit(0.35 * clarity + 0.40 * specificity + 0.25 * legal_refs);
        Self {
            clarity,
            specificity,
            legal_refs,
            overall,
        }
    }
}

fn count_matches(haystack_lower: &str, needles: &[&str]) -> usize {
    needles
        .iter()
        .filter(|n| haystack_lower.contains(&n.to_ascii_lowercase()))
        .count()
}

/// Heuristic quality score for a [`DraftPrompt`] (title + body).
///
/// This is intentionally cheap and deterministic — not an LLM judge. Scores
/// guide multi-turn refinement UIs and tests, not compliance decisions.
pub fn score_draft(prompt: &DraftPrompt) -> DraftQualityScore {
    let combined = format!("{}\n{}", prompt.title, prompt.body);
    let lower = combined.to_ascii_lowercase();
    let len = combined.chars().count();
    let word_count = combined.split_whitespace().count();

    // Clarity: prefer mid-length drafts with punctuation and paragraph breaks.
    let length_score = if len < 80 {
        (len as f64) / 80.0 * 0.4
    } else if len <= 2_500 {
        0.7 + ((len.min(1_200) as f64) / 1_200.0) * 0.3
    } else {
        0.55
    };
    let structure_bonus = if lower.contains('\n') { 0.1 } else { 0.0 }
        + if combined.contains('.') || combined.contains('?') {
            0.1
        } else {
            0.0
        };
    let clarity = DraftQualityScore::clamp_unit(length_score + structure_bonus);

    // Specificity: dates, document nouns, concrete request language.
    let specific_cues = [
        "date",
        "between",
        "from ",
        "until",
        "document",
        "email",
        "report",
        "minutes",
        "contract",
        "policy",
        "copy",
        "copies",
        "including",
        "specifically",
        "period",
        "202",
        "19",
        "20",
        "requested information",
        "subject:",
    ];
    let cue_hits = count_matches(&lower, &specific_cues);
    let word_factor = (word_count as f64 / 80.0).min(1.0);
    let specificity =
        DraftQualityScore::clamp_unit((cue_hits as f64 / 6.0).min(1.0) * 0.75 + word_factor * 0.25);

    // Legal refs: FOI/OIA terminology and statute-like phrases.
    let legal_cues = [
        "official information",
        "freedom of information",
        "foi",
        "oia",
        "foia",
        "act",
        "section",
        "statutory",
        "information commissioner",
        "right to information",
        "public interest",
        "legal basis",
        "review",
        "appeal",
    ];
    let legal_hits = count_matches(&lower, &legal_cues);
    let legal_refs = DraftQualityScore::clamp_unit((legal_hits as f64 / 4.0).min(1.0));

    DraftQualityScore::from_dimensions(clarity, specificity, legal_refs)
}

/// Apply successive user feedback strings to refine a draft prompt multi-turn.
///
/// Each feedback line is appended under a "Refinement feedback" section so an
/// LLM or template path can incorporate operator corrections. Empty feedback
/// entries are skipped. The title is left unchanged unless feedback mentions
/// a `title:` prefix on a line.
pub fn refine_request_multi_turn(base: &DraftPrompt, feedback: &[String]) -> DraftPrompt {
    let mut body = base.body.clone();
    let mut title = base.title.clone();

    let notes: Vec<&str> = feedback
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if notes.is_empty() {
        return DraftPrompt { title, body };
    }

    for note in &notes {
        if let Some(rest) = note
            .strip_prefix("title:")
            .or_else(|| note.strip_prefix("Title:"))
        {
            let t = rest.trim();
            if !t.is_empty() {
                title = t.to_string();
            }
        }
    }

    if !body.contains("## Refinement feedback") {
        body.push_str("\n\n## Refinement feedback\n");
    } else {
        body.push('\n');
    }

    for (i, note) in notes.iter().enumerate() {
        body.push_str(&format!("{}. {}\n", i + 1, note));
    }

    body.push_str(
        "\nIncorporate the numbered feedback above into a clearer, more specific request letter while preserving the legal basis and jurisdiction scaffold.\n",
    );

    DraftPrompt { title, body }
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

    #[test]
    fn score_draft_rewards_specific_legal_prompts() {
        let thin = DraftPrompt {
            title: "Hi".into(),
            body: "Please send stuff.".into(),
        };
        let rich = DraftPrompt {
            title: "FOI request for procurement contracts".into(),
            body: "Under the Freedom of Information Act (FOIA 2000) I request copies of procurement contracts and briefing notes for the period 2024-01-01 to 2024-12-31, including emails and reports. Legal basis: FOI. Appeal to Information Commissioner's Office.\n\nSubject: Budget records\nRequested information: Copies of contracts.".into(),
        };

        let thin_score = score_draft(&thin);
        let rich_score = score_draft(&rich);

        assert!(rich_score.overall > thin_score.overall);
        assert!(rich_score.legal_refs > thin_score.legal_refs);
        assert!(rich_score.specificity > thin_score.specificity);
        assert!((0.0..=1.0).contains(&rich_score.clarity));
        assert!((0.0..=1.0).contains(&rich_score.overall));
    }

    #[test]
    fn refine_request_multi_turn_appends_feedback() {
        let base = DraftPrompt {
            title: "OIA draft for Ministry of Health".into(),
            body: "Scaffold letter body.".into(),
        };
        let feedback = vec![
            "Narrow the date range to 2025 only.".into(),
            "title: OIA draft — elective surgery waitlists".into(),
            "  ".into(),
            "Ask for monthly statistics in CSV if held.".into(),
        ];

        let refined = refine_request_multi_turn(&base, &feedback);
        assert_eq!(refined.title, "OIA draft — elective surgery waitlists");
        assert!(refined.body.contains("## Refinement feedback"));
        assert!(refined.body.contains("1. Narrow the date range"));
        assert!(refined.body.contains("Ask for monthly statistics"));
        assert!(refined.body.contains("Scaffold letter body."));
        assert!(refined.body.contains("Incorporate the numbered feedback"));
    }

    #[test]
    fn refine_with_empty_feedback_is_identity() {
        let base = DraftPrompt {
            title: "T".into(),
            body: "B".into(),
        };
        let refined = refine_request_multi_turn(&base, &[String::new(), "   ".into()]);
        assert_eq!(refined, base);
    }

    #[test]
    fn quality_score_from_dimensions_weights_overall() {
        let score = DraftQualityScore::from_dimensions(1.0, 0.0, 0.0);
        assert!((score.overall - 0.35).abs() < 1e-9);
        let clamped = DraftQualityScore::from_dimensions(2.0, -1.0, 0.5);
        assert_eq!(clamped.clarity, 1.0);
        assert_eq!(clamped.specificity, 0.0);
    }
}
