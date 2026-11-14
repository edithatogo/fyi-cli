//! Synthetic contractual examples: these are not statements of any jurisdiction's law.
use chrono::NaiveDate;
use fyi_core::deadlines::statutory::{
    calculate_explicit_deadline, CalculatedDeadline, DeadlineAdjustment, DeadlineExtension,
    DeadlineRuleError, HolidayCalendar, LegalClockRule, ReceiptConvention, StatutoryDuration,
    StatutoryUnit,
};
use std::collections::BTreeSet;

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn calendar() -> HolidayCalendar {
    HolidayCalendar {
        jurisdiction: "TEST-REGION".into(),
        revision: "synthetic-2026-v1".into(),
        valid_from: day(2026, 1, 1),
        valid_to: day(2027, 12, 31),
        weekend_days: BTreeSet::from([5, 6]),
        excluded_dates: BTreeSet::from([day(2026, 7, 6)]),
    }
}

fn rule(unit: StatutoryUnit, amount: u32) -> LegalClockRule {
    LegalClockRule {
        id: "synthetic-rule".into(),
        jurisdiction: "TEST-REGION".into(),
        source_reference: "synthetic fixture, not legal guidance".into(),
        revision: "test-v1".into(),
        effective_from: day(2026, 1, 1),
        effective_to: None,
        period: StatutoryDuration { amount, unit },
        receipt_convention: ReceiptConvention::ExcludeReceiptDay,
        due_date_adjustment: DeadlineAdjustment::None,
    }
}

#[test]
fn working_day_clock_skips_explicit_regional_exclusion_and_weekend() {
    let result = calculate_explicit_deadline(
        day(2026, 7, 3),
        &rule(StatutoryUnit::WorkingDays, 2),
        &calendar(),
        None,
    )
    .unwrap();
    assert_eq!(result.unadjusted_due_date, day(2026, 7, 8));
    assert_eq!(result.due_date, day(2026, 7, 8));
    assert_eq!(result.calendar_revision, "synthetic-2026-v1");
    assert_eq!(result.validation_status, "unvalidated");
}

#[test]
fn calendar_day_clock_counts_weekends_then_explicitly_rolls_due_date() {
    let mut policy = rule(StatutoryUnit::CalendarDays, 2);
    policy.due_date_adjustment = DeadlineAdjustment::NextWorkingDay;
    let result =
        calculate_explicit_deadline(day(2026, 7, 3), &policy, &calendar(), None).unwrap();
    assert_eq!(result.unadjusted_due_date, day(2026, 7, 5));
    assert_eq!(result.due_date, day(2026, 7, 7));
}

#[test]
fn calendar_month_count_is_not_an_alias_for_thirty_days() {
    let mut policy = rule(StatutoryUnit::CalendarMonths, 1);
    policy.due_date_adjustment = DeadlineAdjustment::NextWorkingDay;
    let result =
        calculate_explicit_deadline(day(2026, 1, 31), &policy, &calendar(), None).unwrap();
    assert_eq!(result.unadjusted_due_date, day(2026, 2, 28));
    assert_eq!(result.due_date, day(2026, 3, 2));
}

#[test]
fn receipt_day_convention_is_explicit_and_not_selected_from_locale() {
    let mut policy = rule(StatutoryUnit::WorkingDays, 1);
    policy.receipt_convention = ReceiptConvention::IncludeReceiptDay;
    let result =
        calculate_explicit_deadline(day(2026, 7, 3), &policy, &calendar(), None).unwrap();
    assert_eq!(result.due_date, day(2026, 7, 3));
    policy.period.unit = StatutoryUnit::CalendarMonths;
    assert_eq!(
        calculate_explicit_deadline(day(2026, 7, 3), &policy, &calendar(), None),
        Err(DeadlineRuleError::UnsupportedCountingConvention)
    );
}

#[test]
fn extension_requires_a_reason_and_authority_and_applies_before_final_roll() {
    let extension = DeadlineExtension {
        authority: "explicit synthetic authorization".into(),
        reason: "synthetic consultation".into(),
        period: StatutoryDuration {
            amount: 2,
            unit: StatutoryUnit::CalendarDays,
        },
    };
    let result = calculate_explicit_deadline(
        day(2026, 7, 3),
        &rule(StatutoryUnit::WorkingDays, 2),
        &calendar(),
        Some(&extension),
    )
    .unwrap();
    assert_eq!(result.due_date, day(2026, 7, 10));
    assert!(result.extension_applied);

    let mut invalid = extension.clone();
    invalid.authority.clear();
    assert_eq!(
        calculate_explicit_deadline(
            day(2026, 7, 3),
            &rule(StatutoryUnit::WorkingDays, 2),
            &calendar(),
            Some(&invalid)
        ),
        Err(DeadlineRuleError::InvalidExtension)
    );
}

#[test]
fn rejects_different_jurisdiction_and_uncovered_dates_without_fallback() {
    let mut other = calendar();
    other.jurisdiction = "OTHER-REGION".into();
    assert_eq!(
        calculate_explicit_deadline(
            day(2026, 7, 3),
            &rule(StatutoryUnit::WorkingDays, 2),
            &other,
            None
        ),
        Err(DeadlineRuleError::JurisdictionMismatch)
    );
    let mut short = calendar();
    short.valid_to = day(2026, 7, 7);
    assert_eq!(
        calculate_explicit_deadline(
            day(2026, 7, 3),
            &rule(StatutoryUnit::WorkingDays, 2),
            &short,
            None
        ),
        Err(DeadlineRuleError::CalendarCoverage)
    );
}

#[test]
fn never_saturates_at_maximum_date_or_accepts_unbounded_periods() {
    let max = NaiveDate::MAX;
    let mut c = calendar();
    c.valid_from = max;
    c.valid_to = max;
    c.excluded_dates.clear();
    let mut p = rule(StatutoryUnit::WorkingDays, 1);
    p.effective_from = day(2026, 1, 1);
    assert_eq!(
        calculate_explicit_deadline(max, &p, &c, None),
        Err(DeadlineRuleError::DateOutOfRange)
    );
    let too_large = rule(StatutoryUnit::WorkingDays, 100_001);
    assert_eq!(
        calculate_explicit_deadline(day(2026, 7, 3), &too_large, &calendar(), None),
        Err(DeadlineRuleError::ExcessivePeriod)
    );
}

#[test]
fn source_and_calendar_metadata_are_required_and_legal_status_is_never_approved() {
    let mut p = rule(StatutoryUnit::WorkingDays, 2);
    p.source_reference.clear();
    assert_eq!(
        calculate_explicit_deadline(day(2026, 7, 3), &p, &calendar(), None),
        Err(DeadlineRuleError::InvalidRule)
    );
    let result = calculate_explicit_deadline(
        day(2026, 7, 3),
        &rule(StatutoryUnit::WorkingDays, 2),
        &calendar(),
        None,
    )
    .unwrap();
    let json = serde_json::to_string(&result).unwrap();
    let restored: CalculatedDeadline = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, result);
    assert!(json.contains("unvalidated"));
    assert!(serde_json::from_str::<CalculatedDeadline>(&json.replace("unvalidated", "approved"))
        .is_err());
}
