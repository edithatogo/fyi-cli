//! Explicit statutory clock calculations. Never infer law from a language or catalog day count.
//!
//! All inputs are caller-selected and unvalidated: the policy's source reference
//! is provenance, not proof of legal applicability. A calendar must declare its
//! complete coverage and all excluded dates, including observed/regional holidays.
//! Only reviewed data may be used operationally; no default regional calendar exists.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use time::{Date, Duration, Month};

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
    #[serde(with = "crate::time_util::date")]
    pub effective_from: Date,
    #[serde(default, with = "crate::time_util::option_date")]
    pub effective_to: Option<Date>,
    pub period: StatutoryDuration,
    pub receipt_convention: ReceiptConvention,
    pub due_date_adjustment: DeadlineAdjustment,
}

/// An explicit day-granularity exclusion calendar for one jurisdiction or scope.
///
/// Weekend day codes are 0=Monday through 6=Sunday. `excluded_dates` must
/// include any applicable national, state/regional, observed or special dates;
/// callers must check completeness for their selected rule. Embedded national
/// and state bases may need a more local calendar. This engine does not invent
/// or fetch holidays and cannot represent a part-day closure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HolidayCalendar {
    pub jurisdiction: String,
    pub revision: String,
    #[serde(with = "crate::time_util::date")]
    pub valid_from: Date,
    #[serde(with = "crate::time_util::date")]
    pub valid_to: Date,
    #[serde(deserialize_with = "deserialize_unique_set")]
    pub weekend_days: BTreeSet<u8>,
    #[serde(
        serialize_with = "crate::time_util::date_btree_set::serialize",
        deserialize_with = "deserialize_unique_date_set"
    )]
    pub excluded_dates: BTreeSet<Date>,
}

fn deserialize_unique_date_set<'de, D>(deserializer: D) -> Result<BTreeSet<Date>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    let mut unique = BTreeSet::new();
    for raw in values {
        let date = Date::parse(
            &raw,
            time::macros::format_description!("[year]-[month]-[day]"),
        )
        .map_err(serde::de::Error::custom)?;
        if !unique.insert(date) {
            return Err(serde::de::Error::custom("duplicate calendar value"));
        }
    }
    Ok(unique)
}

// Serde's BTreeSet implementation silently discards duplicates. Contributions
// must instead obey the schema's uniqueItems contract, including when a calendar
// is deserialized directly rather than through the catalog loader.
fn deserialize_unique_set<'de, D, T>(deserializer: D) -> Result<BTreeSet<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Ord,
{
    let values = Vec::<T>::deserialize(deserializer)?;
    let mut unique = BTreeSet::new();
    for value in values {
        if !unique.insert(value) {
            return Err(serde::de::Error::custom("duplicate calendar value"));
        }
    }
    Ok(unique)
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
    #[serde(with = "crate::time_util::date")]
    pub receipt_date: Date,
    #[serde(with = "crate::time_util::date")]
    pub unadjusted_due_date: Date,
    #[serde(with = "crate::time_util::date")]
    pub due_date: Date,
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

fn check_coverage(date: Date, calendar: &HolidayCalendar) -> Result<(), DeadlineRuleError> {
    if date < calendar.valid_from || date > calendar.valid_to {
        return Err(DeadlineRuleError::CalendarCoverage);
    }
    Ok(())
}

fn is_working(date: Date, calendar: &HolidayCalendar) -> Result<bool, DeadlineRuleError> {
    check_coverage(date, calendar)?;
    Ok(!calendar
        .weekend_days
        .contains(&date.weekday().number_days_from_monday())
        && !calendar.excluded_dates.contains(&date))
}

fn add_months(start: Date, months: u32) -> Option<Date> {
    if months == 0 {
        return Some(start);
    }
    let months = i32::try_from(months).ok()?;
    let total_months = start
        .year()
        .checked_mul(12)?
        .checked_add(i32::from(u8::from(start.month())) - 1)?
        .checked_add(months)?;
    let year = total_months.div_euclid(12);
    let month = Month::try_from((total_months.rem_euclid(12) + 1) as u8).ok()?;
    let day = start.day().min(month.length(year));
    Date::from_calendar_date(year, month, day).ok()
}

// Bounded to make hostile counts fail immediately, rather than hang or silently
// saturate. These limits are much wider than ordinary FOI statutory periods.
const MAX_PERIOD_DAYS: u32 = 100_000;
const MAX_PERIOD_MONTHS: u32 = 1_200;

fn advance_period(
    start: Date,
    period: StatutoryDuration,
    convention: ReceiptConvention,
    calendar: &HolidayCalendar,
) -> Result<Date, DeadlineRuleError> {
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
                .checked_add(Duration::days(i64::from(count)))
                .ok_or(DeadlineRuleError::DateOutOfRange)?;
            check_coverage(date, calendar)?;
            Ok(date)
        }
        StatutoryUnit::CalendarMonths => {
            if convention == ReceiptConvention::IncludeReceiptDay {
                return Err(DeadlineRuleError::UnsupportedCountingConvention);
            }
            let date = add_months(start, period.amount).ok_or(DeadlineRuleError::DateOutOfRange)?;
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
                    .next_day()
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
    receipt_date: Date,
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
            date = date.next_day().ok_or(DeadlineRuleError::DateOutOfRange)?;
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

/// Maximum size for an in-memory holiday-calendar JSON payload (256 KiB).
pub const MAX_CALENDAR_CATALOG_BYTES: usize = 256 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHolidayCalendarCatalog {
    schema_version: u32,
    calendars: Vec<HolidayCalendar>,
}

/// Versioned, caller-extensible catalog of [`HolidayCalendar`] definitions.
///
/// Embedded 2026 NZ/AU/UK dates have source evidence in the adjacent
/// `locales/calendars/v1/provenance.json`. Country/state bases and local calendars
/// have different scopes; choose an exact key after checking its limitations.
/// UK calendars use `GB-ENG`, `GB-WLS`, `GB-SCT` and `GB-NIR`, never ambiguous `GB`.
/// DE/FR/ES starters have not received this source review. All entries and
/// caller-supplied catalogs remain unvalidated for statutory applicability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolidayCalendarCatalog {
    calendars: BTreeMap<String, HolidayCalendar>,
}

impl HolidayCalendar {
    /// Parse and validate a single [`HolidayCalendar`] from JSON without I/O.
    pub fn from_json(input: &str) -> Result<Self, DeadlineRuleError> {
        if input.len() > MAX_CALENDAR_CATALOG_BYTES {
            return Err(DeadlineRuleError::InvalidCalendar);
        }
        let calendar: Self =
            serde_json::from_str(input).map_err(|_| DeadlineRuleError::InvalidCalendar)?;
        validate_calendar(&calendar)?;
        validate_calendar_key(&calendar.jurisdiction)?;
        Ok(calendar)
    }
}

impl HolidayCalendarCatalog {
    /// Load the embedded, legally unvalidated 2026 calendar catalog.
    ///
    /// National/state bases do not silently include local holidays or partial
    /// days. The contribution guide documents each scope and its source evidence.
    #[must_use]
    pub fn embedded() -> Self {
        Self::from_json(include_str!("../../locales/calendars/v1/catalog.json"))
            .expect("embedded holiday calendar catalog must be valid")
    }

    /// Parse and validate a version-1 holiday-calendar JSON catalog in memory.
    ///
    /// Rejects payloads larger than 256 KiB, unknown fields, unsupported
    /// `schema_version`, empty catalogs, duplicate keys/dates/weekend codes, and
    /// any invalid [`HolidayCalendar`] entry. Contribution keys start with a
    /// two-letter uppercase country code, optionally followed by hyphen-separated
    /// uppercase alphanumeric scope components of one to eight characters.
    pub fn from_json(input: &str) -> Result<Self, DeadlineRuleError> {
        if input.len() > MAX_CALENDAR_CATALOG_BYTES {
            return Err(DeadlineRuleError::InvalidCalendar);
        }
        let raw: RawHolidayCalendarCatalog =
            serde_json::from_str(input).map_err(|_| DeadlineRuleError::InvalidCalendar)?;
        if raw.schema_version != 1 || raw.calendars.is_empty() {
            return Err(DeadlineRuleError::InvalidCalendar);
        }
        let mut calendars = BTreeMap::new();
        for calendar in raw.calendars {
            validate_calendar(&calendar)?;
            validate_calendar_key(&calendar.jurisdiction)?;
            if calendars
                .insert(calendar.jurisdiction.clone(), calendar)
                .is_some()
            {
                return Err(DeadlineRuleError::InvalidCalendar);
            }
        }
        Ok(Self { calendars })
    }

    /// Look up an exact jurisdiction/scope key without aliases or regional fallback.
    ///
    /// An unknown region, display locale or bare `"GB"` returns `None` for the
    /// embedded catalog. `"NZ"` and `"AU"` explicitly select national bases only.
    #[must_use]
    pub fn get(&self, jurisdiction: &str) -> Option<&HolidayCalendar> {
        self.calendars.get(jurisdiction)
    }

    /// Iterate over all jurisdiction keys in deterministic sorted order.
    pub fn jurisdictions(&self) -> impl Iterator<Item = &str> {
        self.calendars.keys().map(String::as_str)
    }
}

fn validate_calendar_key(jurisdiction: &str) -> Result<(), DeadlineRuleError> {
    let mut parts = jurisdiction.split('-');
    let country = parts.next().unwrap_or_default();
    if country.len() != 2
        || !country.bytes().all(|byte| byte.is_ascii_uppercase())
        || parts.any(|part| {
            part.is_empty()
                || part.len() > 8
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        })
    {
        return Err(DeadlineRuleError::InvalidCalendar);
    }
    Ok(())
}
