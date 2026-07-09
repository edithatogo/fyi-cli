//! Statutory deadline engine for multi-jurisdiction FOI/OIA tracking.
//!
//! Calculates working-day deadlines from a start date and jurisdiction
//! `statutory_deadline_days`, and provides overdue helpers with store-friendly
//! serde types.

use crate::jurisdiction::Instance;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

/// Working-day calendar rules applied when counting statutory periods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkingDayRule {
    /// Monday–Friday only (default for most FOI regimes).
    #[default]
    WeekdaysOnly,
    /// Count every calendar day (no weekend skip).
    CalendarDays,
}

/// Parameters used to compute a statutory deadline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeadlineInput {
    /// Inclusive start date of the statutory period (typically submission day).
    pub start_date: NaiveDate,
    /// Number of working (or calendar) days allowed by statute.
    pub statutory_deadline_days: u32,
    /// How non-working days are treated while counting.
    #[serde(default)]
    pub working_day_rule: WorkingDayRule,
}

/// Computed statutory deadline with enough metadata for persistence and UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatutoryDeadline {
    pub start_date: NaiveDate,
    pub due_date: NaiveDate,
    pub statutory_deadline_days: u32,
    pub working_day_rule: WorkingDayRule,
    /// Optional jurisdiction / instance id for store joins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
}

/// Overdue evaluation result relative to an "as of" date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverdueStatus {
    pub due_date: NaiveDate,
    pub as_of: NaiveDate,
    pub is_overdue: bool,
    /// Working days past the due date (0 when not overdue).
    pub days_overdue: u32,
    /// Working days remaining until due (0 when overdue or due today).
    pub days_remaining: u32,
}

impl DeadlineInput {
    pub fn new(start_date: NaiveDate, statutory_deadline_days: u32) -> Self {
        Self {
            start_date,
            statutory_deadline_days,
            working_day_rule: WorkingDayRule::WeekdaysOnly,
        }
    }

    pub fn with_rule(mut self, rule: WorkingDayRule) -> Self {
        self.working_day_rule = rule;
        self
    }

    /// Build input from a catalog [`Instance`], defaulting to 20 days.
    pub fn from_instance(start_date: NaiveDate, instance: &Instance) -> Self {
        let days = instance
            .foi_law
            .statutory_deadline_days
            .unwrap_or(20)
            .max(0) as u32;
        Self::new(start_date, days)
    }
}

/// Returns true when `day` is Monday–Friday.
pub fn is_weekday(day: NaiveDate) -> bool {
    matches!(day.weekday().num_days_from_monday(), 0..=4)
}

/// Advance `start` by `days` according to `rule`.
///
/// Counting begins on the day *after* `start` (standard FOI practice: day 0
/// is the day of receipt/submission).
pub fn add_working_days(start: NaiveDate, days: u32, rule: WorkingDayRule) -> NaiveDate {
    if days == 0 {
        return start;
    }
    let mut current = start;
    let mut remaining = days;
    while remaining > 0 {
        current = current.succ_opt().unwrap_or(current);
        let counts = match rule {
            WorkingDayRule::WeekdaysOnly => is_weekday(current),
            WorkingDayRule::CalendarDays => true,
        };
        if counts {
            remaining -= 1;
        }
    }
    current
}

/// Count working (or calendar) days strictly after `from` up to and including `to`.
/// Returns 0 when `to <= from`.
pub fn working_days_between(from: NaiveDate, to: NaiveDate, rule: WorkingDayRule) -> u32 {
    if to <= from {
        return 0;
    }
    let mut current = from;
    let mut count = 0u32;
    while current < to {
        current = current.succ_opt().unwrap_or(current);
        let counts = match rule {
            WorkingDayRule::WeekdaysOnly => is_weekday(current),
            WorkingDayRule::CalendarDays => true,
        };
        if counts {
            count += 1;
        }
    }
    count
}

/// Calculate a [`StatutoryDeadline`] from input parameters.
pub fn calculate_deadline(input: &DeadlineInput) -> StatutoryDeadline {
    let due_date = add_working_days(
        input.start_date,
        input.statutory_deadline_days,
        input.working_day_rule,
    );
    StatutoryDeadline {
        start_date: input.start_date,
        due_date,
        statutory_deadline_days: input.statutory_deadline_days,
        working_day_rule: input.working_day_rule,
        instance_id: None,
    }
}

/// Calculate deadline for a catalog instance and attach its id.
pub fn calculate_deadline_for_instance(
    start_date: NaiveDate,
    instance: &Instance,
) -> StatutoryDeadline {
    let input = DeadlineInput::from_instance(start_date, instance);
    let mut deadline = calculate_deadline(&input);
    deadline.instance_id = Some(instance.id.clone());
    deadline
}

/// Evaluate whether a deadline is overdue as of `as_of`.
///
/// A deadline is overdue when `as_of` is strictly after `due_date`.
pub fn evaluate_overdue(deadline: &StatutoryDeadline, as_of: NaiveDate) -> OverdueStatus {
    let is_overdue = as_of > deadline.due_date;
    let days_overdue = if is_overdue {
        working_days_between(deadline.due_date, as_of, deadline.working_day_rule)
    } else {
        0
    };
    let days_remaining = if is_overdue {
        0
    } else {
        working_days_between(as_of, deadline.due_date, deadline.working_day_rule)
    };
    OverdueStatus {
        due_date: deadline.due_date,
        as_of,
        is_overdue,
        days_overdue,
        days_remaining,
    }
}

/// Convenience: true when the statutory deadline is past `as_of`.
pub fn is_overdue(deadline: &StatutoryDeadline, as_of: NaiveDate) -> bool {
    evaluate_overdue(deadline, as_of).is_overdue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jurisdiction::InstanceRegistry;

    #[test]
    fn add_working_days_skips_weekend() {
        // Friday 3 Jul 2026 + 2 working days => Tuesday 7 Jul 2026
        let start = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let due = add_working_days(start, 2, WorkingDayRule::WeekdaysOnly);
        assert_eq!(due, NaiveDate::from_ymd_opt(2026, 7, 7).unwrap());
    }

    #[test]
    fn calendar_days_count_weekends() {
        let start = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let due = add_working_days(start, 2, WorkingDayRule::CalendarDays);
        assert_eq!(due, NaiveDate::from_ymd_opt(2026, 7, 5).unwrap());
    }

    #[test]
    fn zero_days_returns_start() {
        let start = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        assert_eq!(
            add_working_days(start, 0, WorkingDayRule::WeekdaysOnly),
            start
        );
    }

    #[test]
    fn calculate_deadline_serializes() {
        let start = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let deadline = calculate_deadline(&DeadlineInput::new(start, 20));
        let json = serde_json::to_string(&deadline).unwrap();
        let roundtrip: StatutoryDeadline = serde_json::from_str(&json).unwrap();
        assert_eq!(deadline, roundtrip);
        assert_eq!(
            deadline.due_date,
            NaiveDate::from_ymd_opt(2026, 7, 31).unwrap()
        );
    }

    #[test]
    fn nz_instance_deadline_matches_20_working_days() {
        let registry = InstanceRegistry::embedded().unwrap();
        let instance = registry.get("nz-fyi").unwrap();
        let start = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let deadline = calculate_deadline_for_instance(start, instance);
        assert_eq!(deadline.instance_id.as_deref(), Some("nz-fyi"));
        assert_eq!(deadline.statutory_deadline_days, 20);
        assert_eq!(
            deadline.due_date,
            NaiveDate::from_ymd_opt(2026, 7, 31).unwrap()
        );
    }

    #[test]
    fn overdue_detection() {
        let start = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let deadline = calculate_deadline(&DeadlineInput::new(start, 2));
        // due = 2026-07-07
        let on_due = evaluate_overdue(&deadline, deadline.due_date);
        assert!(!on_due.is_overdue);
        assert_eq!(on_due.days_overdue, 0);

        let after = evaluate_overdue(&deadline, NaiveDate::from_ymd_opt(2026, 7, 9).unwrap());
        assert!(after.is_overdue);
        assert!(after.days_overdue >= 1);
        assert!(is_overdue(
            &deadline,
            NaiveDate::from_ymd_opt(2026, 7, 10).unwrap()
        ));
    }

    #[test]
    fn days_remaining_before_due() {
        let start = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let deadline = calculate_deadline(&DeadlineInput::new(start, 5));
        let status = evaluate_overdue(&deadline, start);
        assert!(!status.is_overdue);
        assert_eq!(status.days_remaining, 5);
        assert_eq!(status.days_overdue, 0);
    }
}
