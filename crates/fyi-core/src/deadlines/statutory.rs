//! Explicit statutory clock calculations. Never infer law from a language or catalog day count.
//!
//! All inputs are caller-selected and unvalidated: the policy's source reference
//! is provenance, not proof of legal applicability. A calendar must declare its
//! complete coverage and all excluded dates, including observed/regional holidays.
//! Only reviewed data may be used operationally; no default regional calendar exists.

use chrono::{Datelike, Days, Months, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

/// Unit of the specified legal period. Months are never approximated as 30 days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatutoryUnit {
    WorkingDays,
    CalendarDays,
    CalendarMonths,
}

/// Explicit handling of the receipt date; this is not inferred from a locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptConvention {
    ExcludeReceiptDay,
    IncludeReceiptDay,
}

/// Whether a non-working terminal date should move to the next working day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeadlineAdjustment {
    None,
    NextWorkingDay,
}

/// A period in a single unambiguous statutory unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatutoryDuration {
    pub amount: u32,
    pub unit: StatutoryUnit,
}

/// A source-qualified but not legally certified, jurisdiction-specific rule.
///
/// A rule only applies to the stated jurisdiction and effective receipt dates.
/// Consumers must supply a legally reviewed version of the rule themselves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegalClockRule {
    pub id: String,
    pub jurisdiction: String,
    pub source_reference: String,
    pub revision: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub period: StatutoryDuration,
    pub receipt_convention: ReceiptConvention,
    pub due_date_adjustment: DeadlineAdjustment,
}

/// The supplied, fully-covered calendar for one jurisdiction or subdivision.
///
/// Weekend day codes are 0=Monday through 6=Sunday. `excluded_dates` must
/// include any applicable national, state/regional, observed or special dates;
/// this engine does not invent or fetch holidays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HolidayCalendar {
    pub jurisdiction: String,
    pub revision: String,
    pub valid_from: NaiveDate,
    pub valid_to: NaiveDate,
    pub weekend_days: BTreeSet<u8>,
    pub excluded_dates: BTreeSet<NaiveDate>,
}

/// An explicitly authorised extension, applied after the primary period.
///
/// The reason and authority are recorded by the caller. Their truth and legal
/// applicability cannot be validated by the arithmetic engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeadlineExtension {
    pub authority: String,
    pub reason: String,
    pub period: StatutoryDuration,
}

/// No calculation may be deserialized as legally reviewed or approved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Unvalidated,
}

/// Computed arithmetic result with explicit policy/calendar provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalculatedDeadline {
    pub receipt_date: NaiveDate,
    pub unadjusted_due_date: NaiveDate,
    pub due_date: NaiveDate,
    pub policy_id: String,
    pub policy_revision: String,
    pub jurisdiction: String,
    pub calendar_revision: String,
    pub extension_applied: bool,
    pub validation_status: ValidationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DeadlineRuleError {
    #[error("deadline rule provenance, period or effective range is invalid")]
    InvalidRule,
    #[error("calendar scope, revision, coverage or exclusions are invalid")]
    InvalidCalendar,
    #[error("the rule and calendar jurisdictions differ")]
    JurisdictionMismatch,
    #[error("the receipt date is outside the rule's effective period")]
    RuleNotEffective,
    #[error("the calendar does not cover the entire calculation")]
    CalendarCoverage,
    #[error("calendar months do not support inclusive receipt counting")]
    UnsupportedCountingConvention,
    #[error("the date is outside the representable range")]
    DateOutOfRange,
    #[error("the requested period is outside the supported bounded range")]
    ExcessivePeriod,
    #[error("extension authority or reason is missing")]
    InvalidExtension,
}

fn validate_rule(rule: &LegalClockRule) -> Result<(), DeadlineRuleError> {
    if rule.id.trim().is_empty()
        || rule.jurisdiction.trim().is_empty()
        || rule.revision.trim().is_empty()
        || rule.source_reference.trim().is_empty()
        || rule
            .effective_to
            .is_some_and(|end| end < rule.effective_from)
    {
        return Err(DeadlineRuleError::InvalidRule);
    }
    Ok(())
}

fn validate_calendar(calendar: &HolidayCalendar) -> Result<(), DeadlineRuleError> {
    if calendar.jurisdiction.trim().is_empty()
        || calendar.revision.trim().is_empty()
        || calendar.valid_to < calendar.valid_from
        || calendar.weekend_days.len() == 7
        || calendar.weekend_days.iter().any(|day| *day > 6)
        || calendar
            .excluded_dates
            .iter()
            .any(|day| *day < calendar.valid_from || *day > calendar.valid_to)
    {
        return Err(DeadlineRuleError::InvalidCalendar);
    }
    Ok(())
}

fn check_coverage(date: NaiveDate, calendar: &HolidayCalendar) -> Result<(), DeadlineRuleError> {
    if date < calendar.valid_from || date > calendar.valid_to {
        return Err(DeadlineRuleError::CalendarCoverage);
    }
    Ok(())
}

fn is_working(date: NaiveDate, calendar: &HolidayCalendar) -> Result<bool, DeadlineRuleError> {
    check_coverage(date, calendar)?;
    Ok(!calendar
        .weekend_days
        .contains(&(date.weekday().num_days_from_monday() as u8))
        && !calendar.excluded_dates.contains(&date))
}

// Bounded to make hostile counts fail immediately, rather than hang or silently
// saturate. These limits are much wider than ordinary FOI statutory periods.
const MAX_PERIOD_DAYS: u32 = 100_000;
const MAX_PERIOD_MONTHS: u32 = 1_200;

fn advance_period(
    start: NaiveDate,
    period: StatutoryDuration,
    convention: ReceiptConvention,
    calendar: &HolidayCalendar,
) -> Result<NaiveDate, DeadlineRuleError> {
    let maximum = match period.unit {
        StatutoryUnit::CalendarMonths => MAX_PERIOD_MONTHS,
        StatutoryUnit::WorkingDays | StatutoryUnit::CalendarDays => MAX_PERIOD_DAYS,
    };
    if period.amount > maximum {
        return Err(DeadlineRuleError::ExcessivePeriod);
    }
    match period.unit {
        StatutoryUnit::CalendarDays => {
            let count = if convention == ReceiptConvention::IncludeReceiptDay {
                period.amount.saturating_sub(1)
            } else {
                period.amount
            };
            let date = start
                .checked_add_days(Days::new(u64::from(count)))
                .ok_or(DeadlineRuleError::DateOutOfRange)?;
            check_coverage(date, calendar)?;
            Ok(date)
        }
        StatutoryUnit::CalendarMonths => {
            if convention == ReceiptConvention::IncludeReceiptDay {
                return Err(DeadlineRuleError::UnsupportedCountingConvention);
            }
            let date = start
                .checked_add_months(Months::new(period.amount))
                .ok_or(DeadlineRuleError::DateOutOfRange)?;
            check_coverage(date, calendar)?;
            Ok(date)
        }
        StatutoryUnit::WorkingDays => {
            let mut current = start;
            let mut remaining = period.amount;
            if convention == ReceiptConvention::IncludeReceiptDay
                && remaining > 0
                && is_working(current, calendar)?
            {
                remaining -= 1;
            }
            while remaining > 0 {
                current = current
                    .succ_opt()
                    .ok_or(DeadlineRuleError::DateOutOfRange)?;
                if is_working(current, calendar)? {
                    remaining -= 1;
                }
            }
            Ok(current)
        }
    }
}

/// Compute a deterministic, explicitly configured but **unvalidated** due date.
///
/// Receipt date is supplied by the caller: network submission, server receipt,
/// acknowledgment and statutory deemed receipt may differ. An optional extension
/// is applied after the base period and before terminal-day adjustment; the caller
/// must verify that sequence against the particular statute. The engine never
/// substitutes a national calendar for an unrecognized regional one.
pub fn calculate_explicit_deadline(
    receipt_date: NaiveDate,
    rule: &LegalClockRule,
    calendar: &HolidayCalendar,
    extension: Option<&DeadlineExtension>,
) -> Result<CalculatedDeadline, DeadlineRuleError> {
    validate_rule(rule)?;
    validate_calendar(calendar)?;
    if rule.jurisdiction != calendar.jurisdiction {
        return Err(DeadlineRuleError::JurisdictionMismatch);
    }
    if receipt_date < rule.effective_from || rule.effective_to.is_some_and(|end| receipt_date > end)
    {
        return Err(DeadlineRuleError::RuleNotEffective);
    }
    check_coverage(receipt_date, calendar)?;
    if let Some(ext) = extension {
        if ext.authority.trim().is_empty() || ext.reason.trim().is_empty() {
            return Err(DeadlineRuleError::InvalidExtension);
        }
    }
    let primary = advance_period(receipt_date, rule.period, rule.receipt_convention, calendar)?;
    let mut date = if let Some(ext) = extension {
        advance_period(
            primary,
            ext.period,
            ReceiptConvention::ExcludeReceiptDay,
            calendar,
        )?
    } else {
        primary
    };
    let unadjusted_due_date = date;
    if rule.due_date_adjustment == DeadlineAdjustment::NextWorkingDay {
        while !is_working(date, calendar)? {
            date = date.succ_opt().ok_or(DeadlineRuleError::DateOutOfRange)?;
        }
    }
    Ok(CalculatedDeadline {
        receipt_date,
        unadjusted_due_date,
        due_date: date,
        policy_id: rule.id.clone(),
        policy_revision: rule.revision.clone(),
        jurisdiction: rule.jurisdiction.clone(),
        calendar_revision: calendar.revision.clone(),
        extension_applied: extension.is_some(),
        validation_status: ValidationStatus::Unvalidated,
    })
}
