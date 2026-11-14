//! Owned, bounded data for the read-only terminal presentation.
//!
//! The adapter supplies facts; these types do not open a database, query a service,
//! infer deadlines, or turn an update timestamp into a history. Text sanitization
//! protects terminal output, not confidentiality: adapters must omit credentials,
//! requester identities, correspondence bodies and raw backend errors entirely.

use std::{collections::BTreeSet, fmt};

/// Maximum request rows accepted by one view snapshot.
pub const MAX_REQUESTS: usize = 500;
/// Maximum timeline events across all requests in one view snapshot.
pub const MAX_TIMELINE_EVENTS: usize = 1000;
/// Maximum Unicode scalar values retained per text field, including an ellipsis.
pub const MAX_TEXT_CHARS: usize = 512;

/// A display category, not a replacement for the backend's status contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestStatus {
    Draft,
    AwaitingResponse,
    AwaitingClarification,
    Successful,
    PartiallySuccessful,
    Refused,
    Withdrawn,
    Overdue,
    Other,
    Unknown,
}

impl RequestStatus {
    /// Map known status labels without guessing an outcome for other values.
    ///
    /// For example, `completed` does not establish a successful disclosure and
    /// `closed` does not establish withdrawal. Unknown text is never echoed.
    pub fn from_backend(status: Option<&str>) -> Self {
        match status.map(str::trim) {
            None | Some("" | "unknown") => Self::Unknown,
            Some("draft") => Self::Draft,
            Some("waiting_response" | "awaiting_response") => Self::AwaitingResponse,
            Some("waiting_clarification" | "awaiting_clarification") => Self::AwaitingClarification,
            Some("successful") => Self::Successful,
            Some("partially_successful") => Self::PartiallySuccessful,
            Some("rejected" | "refused") => Self::Refused,
            Some("withdrawn") => Self::Withdrawn,
            Some("overdue") => Self::Overdue,
            Some(_) => Self::Other,
        }
    }

    /// Static text safe to render even when the original status was untrusted.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::AwaitingResponse => "Awaiting response",
            Self::AwaitingClarification => "Awaiting clarification",
            Self::Successful => "Successful",
            Self::PartiallySuccessful => "Partially successful",
            Self::Refused => "Refused",
            Self::Withdrawn => "Withdrawn",
            Self::Overdue => "Overdue",
            Self::Other => "Other",
            Self::Unknown => "Unknown",
        }
    }
}

/// Minimal request metadata; an authority must never be a requester identity.
#[derive(Clone, PartialEq, Eq)]
pub struct RequestItem {
    pub id: i64,
    pub authority: Option<String>,
    pub title: String,
    pub status: RequestStatus,
    pub needs_attention: Option<bool>,
    pub updated_at: Option<String>,
    pub timeline: TimelineState,
}

impl fmt::Debug for RequestItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestItem")
            .field("status", &self.status)
            .field("needs_attention", &self.needs_attention)
            .field("timeline", &self.timeline)
            .finish_non_exhaustive()
    }
}

/// Missing history and an explicitly loaded empty history are distinct states.
#[derive(Clone, PartialEq, Eq)]
pub enum TimelineState {
    Unavailable,
    Loading,
    Ready(Vec<TimelineItem>),
    Failed(PresentationError),
}

impl fmt::Debug for TimelineState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => f.write_str("Unavailable"),
            Self::Loading => f.write_str("Loading"),
            Self::Ready(events) => f
                .debug_struct("Ready")
                .field("events", &events.len())
                .finish(),
            Self::Failed(error) => f.debug_tuple("Failed").field(error).finish(),
        }
    }
}

/// A caller-supplied historical event, without raw correspondence or error text.
#[derive(Clone, PartialEq, Eq)]
pub struct TimelineItem {
    pub occurred_at: Option<String>,
    pub kind: TimelineKind,
    pub summary: String,
}

impl fmt::Debug for TimelineItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TimelineItem")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

/// Static event categories for supplied history only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineKind {
    Created,
    StatusChanged,
    Updated,
    Note,
    Other,
}

impl TimelineKind {
    /// Display text independent of any backend-provided free-form label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::StatusChanged => "Status changed",
            Self::Updated => "Updated",
            Self::Note => "Note",
            Self::Other => "Other event",
        }
    }
}

/// Whether the adapter supplied its entire collection or a bounded subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SnapshotScope {
    #[default]
    Complete,
    Partial {
        /// Authoritative total if known; never inferred from the displayed rows.
        total: Option<usize>,
    },
}

/// The only data input required by the presentation; no service handles are held.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct PresentationSnapshot {
    pub requests: Vec<RequestItem>,
    pub scope: SnapshotScope,
}

impl fmt::Debug for PresentationSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PresentationSnapshot")
            .field("requests", &self.requests.len())
            .field("scope", &self.scope)
            .finish()
    }
}

impl PresentationSnapshot {
    /// Validate identity/count bounds and sanitize every free-form display field.
    ///
    /// Event order is preserved exactly as supplied by the adapter. Optional blank
    /// labels become unavailable. Validation never infers authority, attention,
    /// history, or a global count from the request rows.
    pub fn validate(mut self) -> Result<Self, PresentationError> {
        if self.requests.len() > MAX_REQUESTS {
            return Err(PresentationError::TooLarge);
        }
        if let SnapshotScope::Partial { total: Some(total) } = self.scope {
            if total < self.requests.len() {
                return Err(PresentationError::InvalidSnapshot);
            }
        }
        let mut ids = BTreeSet::new();
        let mut remaining_events = MAX_TIMELINE_EVENTS;
        for request in &mut self.requests {
            if !ids.insert(request.id) {
                return Err(PresentationError::InvalidSnapshot);
            }
            request.title = safe_text(&request.title);
            request.authority = safe_optional(request.authority.take());
            request.updated_at = safe_optional(request.updated_at.take());
            if let TimelineState::Ready(events) = &mut request.timeline {
                if events.len() > remaining_events {
                    return Err(PresentationError::TooLarge);
                }
                remaining_events -= events.len();
                for event in events {
                    event.occurred_at = safe_optional(event.occurred_at.take());
                    event.summary = safe_text(&event.summary);
                }
            }
        }
        Ok(self)
    }

    /// Summarize only the supplied rows, keeping unknown attention separate.
    ///
    /// `known_authorities` counts distinct supplied, sanitized authority labels;
    /// it is not an authoritative count of backend authority identities.
    pub fn summary(&self) -> SummaryMetrics {
        SummaryMetrics {
            loaded: self.requests.len(),
            drafts: self
                .requests
                .iter()
                .filter(|request| request.status == RequestStatus::Draft)
                .count(),
            needs_attention: self
                .requests
                .iter()
                .filter(|request| request.needs_attention == Some(true))
                .count(),
            unknown_attention: self
                .requests
                .iter()
                .filter(|request| request.needs_attention.is_none())
                .count(),
            known_authorities: self
                .requests
                .iter()
                .filter_map(|request| request.authority.as_deref())
                .collect::<BTreeSet<_>>()
                .len(),
        }
    }
}

/// Aggregate facts about loaded rows, never fabricated backend-wide metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SummaryMetrics {
    pub loaded: usize,
    pub drafts: usize,
    pub needs_attention: usize,
    pub unknown_attention: usize,
    pub known_authorities: usize,
}

/// Closed error categories ensure no database path, SQL, token or URL is echoed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationError {
    Unavailable,
    InvalidSnapshot,
    TooLarge,
}

impl PresentationError {
    /// Generic operator-facing explanation without backend diagnostic content.
    pub fn message(&self) -> &'static str {
        match self {
            Self::Unavailable => "Local data is unavailable.",
            Self::InvalidSnapshot => "The supplied view data is invalid.",
            Self::TooLarge => "The supplied view data exceeds the display limit.",
        }
    }
}

impl fmt::Display for PresentationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for PresentationError {}

fn safe_optional(value: Option<String>) -> Option<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| safe_text(value.trim()))
}

/// Replace terminal controls, bidi overrides and invisible format controls with `�`.
///
/// Ordinary Unicode, including combining accents, is retained. Input is read only
/// up to the 513th Unicode scalar. Output contains at most 512 scalars; if there
/// is more input, the 512th output scalar is replaced by `…`. This is a scalar
/// limit, not a terminal-column or grapheme-cluster limit.
pub fn safe_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len().min(MAX_TEXT_CHARS * 4));
    let mut chars = text.chars();
    for _ in 0..MAX_TEXT_CHARS {
        let Some(ch) = chars.next() else {
            return result;
        };
        result.push(if unsafe_character(ch) { '\u{fffd}' } else { ch });
    }
    if chars.next().is_some() {
        result.pop();
        result.push('…');
    }
    result
}

fn unsafe_character(ch: char) -> bool {
    ch.is_control()
        || matches!(
            ch,
            '\u{00ad}'
                | '\u{034f}'
                | '\u{0600}'..='\u{0605}'
                | '\u{061c}'
                | '\u{06dd}'
                | '\u{070f}'
                | '\u{0890}'..='\u{0891}'
                | '\u{08e2}'
                | '\u{180e}'
                | '\u{200b}'..='\u{200f}'
                | '\u{2028}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
                | '\u{fff9}'..='\u{fffb}'
                | '\u{110bd}'
                | '\u{110cd}'
                | '\u{13430}'..='\u{1343f}'
                | '\u{1bca0}'..='\u{1bca3}'
                | '\u{1d173}'..='\u{1d17a}'
                | '\u{e0001}'
                | '\u{e0020}'..='\u{e007f}'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: i64) -> RequestItem {
        RequestItem {
            id,
            authority: None,
            title: "Synthetic request".to_string(),
            status: RequestStatus::Unknown,
            needs_attention: None,
            updated_at: None,
            timeline: TimelineState::Unavailable,
        }
    }

    fn snapshot(requests: Vec<RequestItem>) -> PresentationSnapshot {
        PresentationSnapshot {
            requests,
            scope: SnapshotScope::Complete,
        }
    }

    fn event() -> TimelineItem {
        TimelineItem {
            occurred_at: None,
            kind: TimelineKind::Note,
            summary: "Synthetic event".to_string(),
        }
    }

    #[test]
    fn defaults_are_empty_and_unknown_values_stay_unknown() {
        let empty = PresentationSnapshot::default().validate().unwrap();
        assert_eq!(empty.scope, SnapshotScope::Complete);
        assert_eq!(empty.summary(), SummaryMetrics::default());
        let state = snapshot(vec![request(1)]).validate().unwrap();
        assert_eq!(state.summary().loaded, 1);
        assert_eq!(state.summary().unknown_attention, 1);
        assert_eq!(state.summary().needs_attention, 0);
        assert_eq!(state.summary().known_authorities, 0);
        assert_eq!(state.requests[0].status, RequestStatus::Unknown);
        assert_eq!(state.requests[0].timeline, TimelineState::Unavailable);
        assert!(state.requests[0].updated_at.is_none());
    }

    #[test]
    fn known_backend_statuses_have_fixed_presentation_labels() {
        let cases = [
            ("draft", RequestStatus::Draft, "Draft"),
            (
                "waiting_response",
                RequestStatus::AwaitingResponse,
                "Awaiting response",
            ),
            (
                "awaiting_response",
                RequestStatus::AwaitingResponse,
                "Awaiting response",
            ),
            (
                "waiting_clarification",
                RequestStatus::AwaitingClarification,
                "Awaiting clarification",
            ),
            (
                "awaiting_clarification",
                RequestStatus::AwaitingClarification,
                "Awaiting clarification",
            ),
            ("successful", RequestStatus::Successful, "Successful"),
            (
                "partially_successful",
                RequestStatus::PartiallySuccessful,
                "Partially successful",
            ),
            ("rejected", RequestStatus::Refused, "Refused"),
            ("refused", RequestStatus::Refused, "Refused"),
            ("withdrawn", RequestStatus::Withdrawn, "Withdrawn"),
            ("overdue", RequestStatus::Overdue, "Overdue"),
        ];
        for (backend, expected, label) in cases {
            let actual = RequestStatus::from_backend(Some(backend));
            assert_eq!(actual, expected);
            assert_eq!(actual.label(), label);
        }
    }

    #[test]
    fn status_mapping_does_not_infer_an_outcome_or_echo_unknown_text() {
        for status in [
            "completed",
            "closed",
            "submitted",
            "partial",
            "private-status",
        ] {
            let actual = RequestStatus::from_backend(Some(status));
            assert_eq!(actual, RequestStatus::Other);
            assert_eq!(actual.label(), "Other");
        }
        for status in [None, Some(""), Some("  "), Some("unknown")] {
            assert_eq!(RequestStatus::from_backend(status), RequestStatus::Unknown);
        }
        assert_eq!(RequestStatus::Unknown.label(), "Unknown");
    }

    #[test]
    fn timeline_kind_labels_are_static_and_do_not_imply_unprovided_events() {
        for (kind, expected) in [
            (TimelineKind::Created, "Created"),
            (TimelineKind::StatusChanged, "Status changed"),
            (TimelineKind::Updated, "Updated"),
            (TimelineKind::Note, "Note"),
            (TimelineKind::Other, "Other event"),
        ] {
            assert_eq!(kind.label(), expected);
        }
    }

    #[test]
    fn duplicate_request_ids_are_rejected() {
        assert_eq!(
            snapshot(vec![request(7), request(7)]).validate(),
            Err(PresentationError::InvalidSnapshot)
        );
    }

    #[test]
    fn row_limit_is_inclusive() {
        assert!(snapshot((0..MAX_REQUESTS as i64).map(request).collect())
            .validate()
            .is_ok());
        assert_eq!(
            snapshot((0..=MAX_REQUESTS as i64).map(request).collect()).validate(),
            Err(PresentationError::TooLarge)
        );
    }

    #[test]
    fn event_limit_applies_to_the_whole_snapshot() {
        let mut first = request(1);
        first.timeline = TimelineState::Ready(vec![event(); MAX_TIMELINE_EVENTS / 2]);
        let mut second = request(2);
        second.timeline = TimelineState::Ready(vec![event(); MAX_TIMELINE_EVENTS / 2]);
        assert!(snapshot(vec![first.clone(), second.clone()])
            .validate()
            .is_ok());
        if let TimelineState::Ready(events) = &mut second.timeline {
            events.push(event());
        }
        assert_eq!(
            snapshot(vec![first, second]).validate(),
            Err(PresentationError::TooLarge)
        );
    }

    #[test]
    fn partial_totals_cannot_be_smaller_than_loaded_rows() {
        let mut state = snapshot(vec![request(1), request(2)]);
        state.scope = SnapshotScope::Partial { total: Some(1) };
        assert_eq!(state.validate(), Err(PresentationError::InvalidSnapshot));
        for total in [None, Some(2), Some(20)] {
            let state = PresentationSnapshot {
                requests: vec![request(1), request(2)],
                scope: SnapshotScope::Partial { total },
            };
            assert_eq!(state.clone().validate().unwrap().scope, state.scope);
        }
    }

    #[test]
    fn summary_counts_only_supplied_facts_in_loaded_rows() {
        let mut first = request(1);
        first.status = RequestStatus::Draft;
        first.needs_attention = Some(true);
        first.authority = Some("Synthetic Authority A".to_string());
        let mut second = request(2);
        second.status = RequestStatus::Overdue;
        second.needs_attention = Some(false);
        second.authority = first.authority.clone();
        let mut third = request(3);
        third.authority = Some("Synthetic Authority B".to_string());
        let state = PresentationSnapshot {
            requests: vec![first, second, third],
            scope: SnapshotScope::Partial { total: Some(100) },
        }
        .validate()
        .unwrap();
        assert_eq!(
            state.summary(),
            SummaryMetrics {
                loaded: 3,
                drafts: 1,
                needs_attention: 1,
                unknown_attention: 1,
                known_authorities: 2,
            }
        );
    }

    #[test]
    fn every_supplied_text_field_is_sanitized_without_fabricating_metadata() {
        let mut row = request(1);
        row.authority = Some("Authority\u{202e}".to_string());
        row.title = "Title\x1b\n".to_string();
        row.updated_at = Some("date\tlabel".to_string());
        row.timeline = TimelineState::Ready(vec![TimelineItem {
            occurred_at: Some("event\rdate".to_string()),
            kind: TimelineKind::Updated,
            summary: "detail\u{200b}".to_string(),
        }]);
        let state = snapshot(vec![row]).validate().unwrap();
        let row = &state.requests[0];
        assert_eq!(row.authority.as_deref(), Some("Authority�"));
        assert_eq!(row.title, "Title��");
        assert_eq!(row.updated_at.as_deref(), Some("date�label"));
        let TimelineState::Ready(events) = &row.timeline else {
            panic!("supplied timeline must be preserved");
        };
        assert_eq!(events[0].occurred_at.as_deref(), Some("event�date"));
        assert_eq!(events[0].summary, "detail�");
        assert_eq!(row.needs_attention, None);
    }

    #[test]
    fn blank_optional_metadata_is_unavailable() {
        let mut row = request(1);
        row.authority = Some("   ".to_string());
        row.updated_at = Some(String::new());
        row.timeline = TimelineState::Ready(vec![TimelineItem {
            occurred_at: Some(" ".to_string()),
            ..event()
        }]);
        let state = snapshot(vec![row]).validate().unwrap();
        assert!(state.requests[0].authority.is_none());
        assert!(state.requests[0].updated_at.is_none());
        let TimelineState::Ready(events) = &state.requests[0].timeline else {
            panic!("supplied timeline must be preserved");
        };
        assert!(events[0].occurred_at.is_none());
        assert_eq!(state.summary().known_authorities, 0);
    }

    #[test]
    fn sanitizer_preserves_ordinary_unicode_and_replaces_unsafe_controls() {
        let ordinary = "Café 日本語 Māori Ελληνικά e\u{301} 🚲";
        assert_eq!(safe_text(ordinary), ordinary);
        for ch in (0_u32..=0x9f)
            .filter_map(char::from_u32)
            .filter(|ch| ch.is_control())
            .chain([
                '\u{00ad}',
                '\u{034f}',
                '\u{061c}',
                '\u{180e}',
                '\u{200b}',
                '\u{200c}',
                '\u{200d}',
                '\u{200e}',
                '\u{200f}',
                '\u{2028}',
                '\u{2029}',
                '\u{202a}',
                '\u{202b}',
                '\u{202c}',
                '\u{202d}',
                '\u{202e}',
                '\u{2060}',
                '\u{2066}',
                '\u{2067}',
                '\u{2068}',
                '\u{2069}',
                '\u{feff}',
                '\u{e0001}',
                '\u{e0020}',
                '\u{e007f}',
            ])
        {
            assert_eq!(safe_text(&format!("a{ch}b")), "a�b", "U+{:04X}", ch as u32);
        }
    }

    #[test]
    fn truncation_is_bounded_by_unicode_scalars_including_ellipsis() {
        let exact = "界".repeat(MAX_TEXT_CHARS);
        assert_eq!(safe_text(&exact), exact);
        let long = format!("{exact}x");
        let result = safe_text(&long);
        assert_eq!(result.chars().count(), MAX_TEXT_CHARS);
        assert_eq!(result, format!("{}…", "界".repeat(MAX_TEXT_CHARS - 1)));
        assert_eq!(safe_text(""), "");
    }

    #[test]
    fn timeline_order_and_state_are_owned_by_the_caller() {
        let mut ready = request(1);
        ready.timeline = TimelineState::Ready(vec![
            TimelineItem {
                occurred_at: Some("later".to_string()),
                ..event()
            },
            TimelineItem {
                occurred_at: Some("earlier".to_string()),
                ..event()
            },
        ]);
        let mut loading = request(2);
        loading.timeline = TimelineState::Loading;
        let mut failed = request(3);
        failed.timeline = TimelineState::Failed(PresentationError::Unavailable);
        let original = snapshot(vec![ready, loading, failed]);
        assert_eq!(original.clone().validate().unwrap(), original);
    }

    #[test]
    fn debug_and_errors_never_echo_request_content_or_identifiers() {
        let mut row = request(972431);
        row.title = "SYNTHETIC_PRIVATE_TITLE".to_string();
        row.authority = Some("SYNTHETIC_PRIVATE_AUTHORITY".to_string());
        row.updated_at = Some("SYNTHETIC_PRIVATE_TIME".to_string());
        row.timeline = TimelineState::Ready(vec![TimelineItem {
            occurred_at: Some("SYNTHETIC_PRIVATE_EVENT_TIME".to_string()),
            kind: TimelineKind::Other,
            summary: "SYNTHETIC_PRIVATE_SUMMARY".to_string(),
        }]);
        let state = snapshot(vec![row]);
        let text = format!(
            "{:?} {:?} {:?}",
            state, state.requests[0], state.requests[0].timeline
        );
        assert!(!text.contains("SYNTHETIC_PRIVATE"));
        assert!(!text.contains("972431"));
        if let TimelineState::Ready(events) = &state.requests[0].timeline {
            assert!(!format!("{:?}", events[0]).contains("SYNTHETIC_PRIVATE"));
        }
        for (error, message) in [
            (PresentationError::Unavailable, "Local data is unavailable."),
            (
                PresentationError::InvalidSnapshot,
                "The supplied view data is invalid.",
            ),
            (
                PresentationError::TooLarge,
                "The supplied view data exceeds the display limit.",
            ),
        ] {
            assert_eq!(error.message(), message);
            assert_eq!(error.to_string(), message);
        }
    }
}
