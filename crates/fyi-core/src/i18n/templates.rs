//! Typed, offline letter rendering. The result is always legally unvalidated.

use super::{BundleVersion, I18nError, LocaleResolution, LocalizationEngine};
use crate::jurisdiction::{FoiLaw, Instance};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A reference to the existing jurisdiction catalog, independent of message locale.
#[derive(Debug, Clone, Copy)]
pub struct LegalCitation<'a> {
    pub jurisdiction_id: &'a str,
    pub law: &'a FoiLaw,
}

impl<'a> LegalCitation<'a> {
    /// Borrow existing catalog fields without maintaining another legal registry.
    pub fn from_instance(instance: &'a Instance) -> Self {
        Self {
            jurisdiction_id: &instance.id,
            law: &instance.foi_law,
        }
    }
}

/// Supplied review contact/instructions, not a determination of appeal rights.
#[derive(Debug, Clone, Copy)]
pub struct ReviewInformation<'a> {
    pub authority: &'a str,
    pub instructions: Option<&'a str>,
}

impl<'a> ReviewInformation<'a> {
    /// Reuse the catalog's optional review body; callers must check applicability.
    pub fn from_instance(instance: &'a Instance) -> Option<Self> {
        instance
            .foi_law
            .appeal_body
            .as_deref()
            .map(|authority| Self {
                authority,
                instructions: None,
            })
    }
}

/// Letter intent. Selecting a variant never establishes a legal entitlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateVariant {
    Request,
    FollowUp,
    InternalReview,
    Appeal,
}

impl TemplateVariant {
    fn keys(self) -> (&'static str, &'static str) {
        match self {
            Self::Request => ("request_title", "request_body"),
            Self::FollowUp => ("follow_up_title", "follow_up_body"),
            Self::InternalReview => ("internal_review_title", "internal_review_body"),
            Self::Appeal => ("appeal_title", "appeal_body"),
        }
    }
}

/// Greeting/sign-off style. Neither register is human-approved by this API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LetterRegister {
    Formal,
    Informal,
}

/// Explicit output context. HtmlText is escaped text, not an HTML document/header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    PlainText,
    HtmlText,
}

/// Only the unvalidated rendering state can be produced by this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    RenderedUnvalidated,
}

/// Complete letter input with supplied body/title and catalog-derived legal references.
#[derive(Debug, Clone)]
pub struct RequestTemplateInput<'a> {
    pub authority: &'a str,
    pub requester: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub legal: LegalCitation<'a>,
    pub variant: TemplateVariant,
    pub register: LetterRegister,
    /// Caller-formatted date text; this field does not calculate a deadline.
    pub date: Option<&'a str>,
    /// Required for follow-up, internal-review and appeal letters.
    pub request_reference: Option<&'a str>,
    /// Optional except for an appeal, which requires an explicitly supplied contact.
    pub review: Option<ReviewInformation<'a>>,
}

impl<'a> RequestTemplateInput<'a> {
    /// Start a formal initial request. Review information is opt-in, never inferred.
    pub fn new(
        authority: &'a str,
        requester: &'a str,
        title: &'a str,
        body: &'a str,
        instance: &'a Instance,
    ) -> Self {
        Self {
            authority,
            requester,
            title,
            body,
            legal: LegalCitation::from_instance(instance),
            variant: TemplateVariant::Request,
            register: LetterRegister::Formal,
            date: None,
            request_reference: None,
            review: None,
        }
    }

    fn validate(&self) -> Result<(), I18nError> {
        for (field, value) in [
            ("authority", self.authority),
            ("requester", self.requester),
            ("title", self.title),
            ("jurisdiction_id", self.legal.jurisdiction_id),
            ("law_name", self.legal.law.law_name.as_str()),
            ("request_term", self.legal.law.request_term.as_str()),
        ] {
            validate_field(field, value, false)?;
        }
        validate_field("body", self.body, true)?;
        for (field, value) in [
            ("citation", self.legal.law.citation.as_deref()),
            ("date", self.date),
            ("request_reference", self.request_reference),
        ] {
            if let Some(value) = value {
                validate_field(field, value, false)?;
            }
        }
        if self.variant != TemplateVariant::Request && self.request_reference.is_none() {
            return Err(I18nError::MissingField {
                field: "request_reference",
            });
        }
        if self.variant == TemplateVariant::Appeal && self.review.is_none() {
            return Err(I18nError::MissingField {
                field: "review_authority",
            });
        }
        if let Some(review) = self.review {
            validate_field("review_authority", review.authority, false)?;
            if let Some(instructions) = review.instructions {
                validate_field("review_instructions", instructions, true)?;
            }
        }
        Ok(())
    }
}

fn validate_field(field: &'static str, value: &str, multiline: bool) -> Result<(), I18nError> {
    if value.trim().is_empty() {
        return Err(I18nError::MissingField { field });
    }
    if value.chars().any(|c| {
        (c.is_control() && !(multiline && matches!(c, '\n' | '\r' | '\t')))
            || (!multiline && matches!(c, '\u{2028}' | '\u{2029}'))
    }) {
        return Err(I18nError::InvalidField { field });
    }
    Ok(())
}

/// A deterministic draft plus rendering provenance; never a validated/submitted request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderedRequest {
    pub title: String,
    pub body: String,
    pub jurisdiction_id: String,
    pub locale: LocaleResolution,
    pub bundle_version: BundleVersion,
    pub message_sources: BTreeMap<String, String>,
    pub variant: TemplateVariant,
    pub register: LetterRegister,
    pub format: OutputFormat,
    pub validation: ValidationStatus,
}

impl LocalizationEngine {
    /// Render an offline draft. Missing/invalid fields fail before any output is returned.
    /// HTML text is escaped after complete rendering; input is never parsed as markup.
    pub fn render_request(
        &self,
        input: &RequestTemplateInput<'_>,
        format: OutputFormat,
    ) -> Result<RenderedRequest, I18nError> {
        input.validate()?;
        let mut args = BTreeMap::from([
            ("authority".into(), input.authority.into()),
            ("title".into(), input.title.into()),
            ("law_name".into(), input.legal.law.law_name.clone()),
            ("request_term".into(), input.legal.law.request_term.clone()),
        ]);
        let mut sources = BTreeMap::new();
        let citation = if let Some(citation) = input.legal.law.citation.as_deref() {
            args.insert("citation".into(), citation.into());
            self.record_message("legal_citation", &args, &mut sources)?
        } else {
            input.legal.law.law_name.clone()
        };
        args.insert("legal_citation".into(), citation);
        if let Some(reference) = input.request_reference {
            args.insert("request_reference".into(), reference.into());
        }
        if let Some(date) = input.date {
            args.insert("date".into(), date.into());
        }
        if let Some(review) = input.review {
            args.insert("review_authority".into(), review.authority.into());
        }
        let (title_key, body_key) = input.variant.keys();
        let title = self.record_message(title_key, &args, &mut sources)?;
        let (greeting_key, closing_key) = match input.register {
            LetterRegister::Formal => ("salutation", "closing"),
            LetterRegister::Informal => ("informal_salutation", "informal_closing"),
        };
        let mut sections = vec![self.record_message(greeting_key, &args, &mut sources)?];
        if input.date.is_some() {
            sections.push(self.record_message("date_line", &args, &mut sources)?);
        }
        if input.variant == TemplateVariant::Request && input.request_reference.is_some() {
            sections.push(self.record_message("reference_line", &args, &mut sources)?);
        }
        sections.push(self.record_message(body_key, &args, &mut sources)?);
        sections.push(input.body.into());
        if let Some(review) = input.review {
            sections.push(self.record_message("review_contact", &args, &mut sources)?);
            if let Some(instructions) = review.instructions {
                sections.push(instructions.into());
            }
        }
        sections.push(format!(
            "{}\n{}",
            self.record_message(closing_key, &args, &mut sources)?,
            input.requester
        ));
        let body = sections.join("\n\n");
        let (title, body) = match format {
            OutputFormat::PlainText => (title, body),
            OutputFormat::HtmlText => (escape_html_text(&title), escape_html_text(&body)),
        };
        Ok(RenderedRequest {
            title,
            body,
            jurisdiction_id: input.legal.jurisdiction_id.into(),
            locale: self.resolution().clone(),
            bundle_version: BundleVersion::embedded(),
            message_sources: sources,
            variant: input.variant,
            register: input.register,
            format,
            validation: ValidationStatus::RenderedUnvalidated,
        })
    }

    fn record_message(
        &self,
        key: &str,
        args: &BTreeMap<String, String>,
        sources: &mut BTreeMap<String, String>,
    ) -> Result<String, I18nError> {
        let message = self.render_message(key, args)?;
        sources.insert(key.into(), message.source_locale);
        Ok(message.text)
    }
}

fn escape_html_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
