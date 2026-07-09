//! Deadline reminder schedules and webhook payload builders.
//!
//! Pure types only — no email, HTTP, or other delivery. Callers own transport.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Default lead times (calendar days before due) for deadline reminders.
pub const DEFAULT_REMINDER_DAYS_BEFORE: &[u32] = &[7, 3, 1];

/// Schedule of how many calendar days before a due date to notify.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderSchedule {
    /// Calendar days before `due` when a reminder should fire.
    /// Default: 7, 3, 1.
    pub days_before: Vec<u32>,
}

impl Default for ReminderSchedule {
    fn default() -> Self {
        Self {
            days_before: DEFAULT_REMINDER_DAYS_BEFORE.to_vec(),
        }
    }
}

impl ReminderSchedule {
    pub fn new(days_before: impl IntoIterator<Item = u32>) -> Self {
        let mut days: Vec<u32> = days_before.into_iter().collect();
        days.sort_unstable();
        days.dedup();
        // Prefer larger lead times first for display/UX (7, 3, 1).
        days.reverse();
        Self { days_before: days }
    }

    /// Standard FOI operator schedule: 7, 3, and 1 day(s) before due.
    pub fn standard() -> Self {
        Self::default()
    }

    /// True when `days_remaining` matches one of the schedule offsets.
    pub fn should_notify_on_remaining(&self, days_remaining: i64) -> bool {
        if days_remaining < 0 {
            return false;
        }
        self.days_before
            .iter()
            .any(|&d| i64::from(d) == days_remaining)
    }
}

/// Reminder evaluation for a single deadline relative to `as_of`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeadlineReminder {
    pub due: NaiveDate,
    pub as_of: NaiveDate,
    /// Calendar days remaining until due (negative when overdue).
    pub days_remaining: i64,
    /// Whether a notification should fire under the associated schedule.
    pub should_notify: bool,
}

impl DeadlineReminder {
    /// Evaluate reminder state for `due` as of `as_of` using `schedule`.
    pub fn evaluate(due: NaiveDate, as_of: NaiveDate, schedule: &ReminderSchedule) -> Self {
        let days_remaining = (due - as_of).num_days();
        let should_notify = schedule.should_notify_on_remaining(days_remaining);
        Self {
            due,
            as_of,
            days_remaining,
            should_notify,
        }
    }

    /// True when the due date is strictly before `as_of`.
    pub fn is_overdue(&self) -> bool {
        self.days_remaining < 0
    }
}

/// Event types emitted as webhook-shaped JSON (delivery not implemented here).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    DeadlineApproaching,
    DeadlineOverdue,
}

/// Webhook-shaped payload for deadline notifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event: WebhookEventType,
    pub request_id: Option<String>,
    pub due: NaiveDate,
    pub as_of: NaiveDate,
    pub days_remaining: i64,
    pub message: String,
}

/// Builder for [`WebhookPayload`].
#[derive(Debug, Clone, Default)]
pub struct WebhookPayloadBuilder {
    request_id: Option<String>,
    due: Option<NaiveDate>,
    as_of: Option<NaiveDate>,
    days_remaining: Option<i64>,
    message: Option<String>,
}

impl WebhookPayloadBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    pub fn due(mut self, due: NaiveDate) -> Self {
        self.due = Some(due);
        self
    }

    pub fn as_of(mut self, as_of: NaiveDate) -> Self {
        self.as_of = Some(as_of);
        self
    }

    pub fn days_remaining(mut self, days: i64) -> Self {
        self.days_remaining = Some(days);
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Populate from a [`DeadlineReminder`], optionally attaching a request id.
    pub fn from_reminder(reminder: &DeadlineReminder) -> Self {
        Self {
            request_id: None,
            due: Some(reminder.due),
            as_of: Some(reminder.as_of),
            days_remaining: Some(reminder.days_remaining),
            message: None,
        }
    }

    /// Build a **deadline approaching** payload.
    ///
    /// Requires `due`, `as_of`, and `days_remaining`. Generates a default
    /// message when none was set.
    pub fn build_deadline_approaching(self) -> Result<WebhookPayload, String> {
        let due = self.due.ok_or_else(|| "due date is required".to_string())?;
        let as_of = self
            .as_of
            .ok_or_else(|| "as_of date is required".to_string())?;
        let days_remaining = self
            .days_remaining
            .ok_or_else(|| "days_remaining is required".to_string())?;
        let message = self.message.unwrap_or_else(|| {
            format!(
                "Deadline approaching: due {due} ({days_remaining} calendar day(s) remaining as of {as_of})"
            )
        });
        Ok(WebhookPayload {
            event: WebhookEventType::DeadlineApproaching,
            request_id: self.request_id,
            due,
            as_of,
            days_remaining,
            message,
        })
    }
}

/// Convenience: build an approaching payload when the schedule says to notify.
pub fn approaching_payload_if_due(
    due: NaiveDate,
    as_of: NaiveDate,
    schedule: &ReminderSchedule,
    request_id: Option<&str>,
) -> Option<WebhookPayload> {
    let reminder = DeadlineReminder::evaluate(due, as_of, schedule);
    if !reminder.should_notify {
        return None;
    }
    let mut builder = WebhookPayloadBuilder::from_reminder(&reminder);
    if let Some(id) = request_id {
        builder = builder.request_id(id);
    }
    builder.build_deadline_approaching().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_schedule_is_7_3_1() {
        let schedule = ReminderSchedule::standard();
        assert_eq!(schedule.days_before, vec![7, 3, 1]);
    }

    #[test]
    fn notify_exactly_on_schedule_offsets() {
        let schedule = ReminderSchedule::standard();
        let due = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();

        let seven = DeadlineReminder::evaluate(
            due,
            NaiveDate::from_ymd_opt(2026, 7, 13).unwrap(),
            &schedule,
        );
        assert_eq!(seven.days_remaining, 7);
        assert!(seven.should_notify);

        let three = DeadlineReminder::evaluate(
            due,
            NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
            &schedule,
        );
        assert_eq!(three.days_remaining, 3);
        assert!(three.should_notify);

        let one = DeadlineReminder::evaluate(
            due,
            NaiveDate::from_ymd_opt(2026, 7, 19).unwrap(),
            &schedule,
        );
        assert_eq!(one.days_remaining, 1);
        assert!(one.should_notify);

        let other = DeadlineReminder::evaluate(
            due,
            NaiveDate::from_ymd_opt(2026, 7, 15).unwrap(),
            &schedule,
        );
        assert_eq!(other.days_remaining, 5);
        assert!(!other.should_notify);
    }

    #[test]
    fn overdue_does_not_notify_on_schedule() {
        let schedule = ReminderSchedule::standard();
        let due = NaiveDate::from_ymd_opt(2026, 7, 10).unwrap();
        let as_of = NaiveDate::from_ymd_opt(2026, 7, 12).unwrap();
        let reminder = DeadlineReminder::evaluate(due, as_of, &schedule);
        assert!(reminder.is_overdue());
        assert!(!reminder.should_notify);
        assert_eq!(reminder.days_remaining, -2);
    }

    #[test]
    fn webhook_payload_builder_deadline_approaching() {
        let due = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
        let as_of = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();
        let payload = WebhookPayloadBuilder::new()
            .request_id("42")
            .due(due)
            .as_of(as_of)
            .days_remaining(7)
            .build_deadline_approaching()
            .unwrap();

        assert_eq!(payload.event, WebhookEventType::DeadlineApproaching);
        assert_eq!(payload.request_id.as_deref(), Some("42"));
        assert_eq!(payload.days_remaining, 7);
        assert!(payload.message.contains("Deadline approaching"));
        assert!(payload.message.contains("7"));

        let json = serde_json::to_string(&payload).unwrap();
        let roundtrip: WebhookPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(payload, roundtrip);
    }

    #[test]
    fn approaching_payload_if_due_only_on_schedule() {
        let schedule = ReminderSchedule::standard();
        let due = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
        let hit = approaching_payload_if_due(
            due,
            NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
            &schedule,
            Some("req-9"),
        );
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().request_id.as_deref(), Some("req-9"));

        let miss = approaching_payload_if_due(
            due,
            NaiveDate::from_ymd_opt(2026, 7, 14).unwrap(),
            &schedule,
            None,
        );
        assert!(miss.is_none());
    }

    #[test]
    fn builder_requires_fields() {
        let err = WebhookPayloadBuilder::new()
            .build_deadline_approaching()
            .unwrap_err();
        assert!(err.contains("due"));
    }
}
