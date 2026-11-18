//! Synthetic contractual examples: these are not statements of any jurisdiction's law.
use fyi_core::deadlines::statutory::{
    calculate_explicit_deadline, CalculatedDeadline, DeadlineAdjustment, DeadlineExtension,
    DeadlineRuleError, HolidayCalendar, HolidayCalendarCatalog, LegalClockRule, ReceiptConvention,
    StatutoryDuration, StatutoryUnit, ValidationStatus, MAX_CALENDAR_CATALOG_BYTES,
};
use std::collections::BTreeSet;
use time::{Date, Month};

fn day(y: i32, m: u32, d: u32) -> Date {
    Date::from_calendar_date(y, Month::try_from(m as u8).unwrap(), d as u8).unwrap()
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
    assert_eq!(result.validation_status, ValidationStatus::Unvalidated);
}

#[test]
fn calendar_day_clock_counts_weekends_then_explicitly_rolls_due_date() {
    let mut policy = rule(StatutoryUnit::CalendarDays, 2);
    policy.due_date_adjustment = DeadlineAdjustment::NextWorkingDay;
    let result = calculate_explicit_deadline(day(2026, 7, 3), &policy, &calendar(), None).unwrap();
    assert_eq!(result.unadjusted_due_date, day(2026, 7, 5));
    assert_eq!(result.due_date, day(2026, 7, 7));
}

#[test]
fn calendar_month_count_is_not_an_alias_for_thirty_days() {
    let mut policy = rule(StatutoryUnit::CalendarMonths, 1);
    policy.due_date_adjustment = DeadlineAdjustment::NextWorkingDay;
    let result = calculate_explicit_deadline(day(2026, 1, 31), &policy, &calendar(), None).unwrap();
    assert_eq!(result.unadjusted_due_date, day(2026, 2, 28));
    assert_eq!(result.due_date, day(2026, 3, 2));
}

#[test]
fn receipt_day_convention_is_explicit_and_not_selected_from_locale() {
    let mut policy = rule(StatutoryUnit::WorkingDays, 1);
    policy.receipt_convention = ReceiptConvention::IncludeReceiptDay;
    let result = calculate_explicit_deadline(day(2026, 7, 3), &policy, &calendar(), None).unwrap();
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
    let max = Date::MAX;
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
    assert!(
        serde_json::from_str::<CalculatedDeadline>(&json.replace("unvalidated", "approved"))
            .is_err()
    );
}

#[test]
fn malformed_calendar_and_unknown_weekend_code_fail_closed() {
    let mut c = calendar();
    c.weekend_days.insert(7);
    assert_eq!(
        calculate_explicit_deadline(
            day(2026, 7, 3),
            &rule(StatutoryUnit::WorkingDays, 1),
            &c,
            None
        ),
        Err(DeadlineRuleError::InvalidCalendar)
    );
    let mut c = calendar();
    c.excluded_dates.insert(day(2028, 1, 1));
    assert_eq!(
        calculate_explicit_deadline(
            day(2026, 7, 3),
            &rule(StatutoryUnit::WorkingDays, 1),
            &c,
            None
        ),
        Err(DeadlineRuleError::InvalidCalendar)
    );
    let mut c = calendar();
    c.weekend_days = BTreeSet::from([0, 1, 2, 3, 4, 5, 6]);
    assert_eq!(
        calculate_explicit_deadline(
            day(2026, 7, 3),
            &rule(StatutoryUnit::WorkingDays, 1),
            &c,
            None
        ),
        Err(DeadlineRuleError::InvalidCalendar)
    );
}

#[test]
fn effective_receipt_window_is_checked_independently_of_calendar_coverage() {
    let mut p = rule(StatutoryUnit::CalendarDays, 1);
    p.effective_from = day(2026, 8, 1);
    assert_eq!(
        calculate_explicit_deadline(day(2026, 7, 3), &p, &calendar(), None),
        Err(DeadlineRuleError::RuleNotEffective)
    );
    p.effective_to = Some(day(2026, 7, 1));
    assert_eq!(
        calculate_explicit_deadline(day(2026, 7, 3), &p, &calendar(), None),
        Err(DeadlineRuleError::InvalidRule)
    );
}

#[test]
fn terminal_adjustment_requires_coverage_beyond_unadjusted_due_date() {
    let mut c = calendar();
    c.valid_to = day(2026, 7, 5);
    c.excluded_dates.clear();
    let mut p = rule(StatutoryUnit::CalendarDays, 2);
    p.due_date_adjustment = DeadlineAdjustment::NextWorkingDay;
    assert_eq!(
        calculate_explicit_deadline(day(2026, 7, 3), &p, &c, None),
        Err(DeadlineRuleError::CalendarCoverage)
    );
}

#[test]
fn alternative_weekends_and_inclusive_calendar_days_are_data_not_locale_rules() {
    let mut c = calendar();
    c.excluded_dates.clear();
    c.weekend_days = BTreeSet::from([4, 5]); // Friday / Saturday
    let r = rule(StatutoryUnit::WorkingDays, 1);
    let result = calculate_explicit_deadline(day(2026, 7, 2), &r, &c, None).unwrap();
    assert_eq!(result.due_date, day(2026, 7, 5)); // Sunday counts

    let mut inclusive = rule(StatutoryUnit::CalendarDays, 2);
    inclusive.receipt_convention = ReceiptConvention::IncludeReceiptDay;
    let result = calculate_explicit_deadline(day(2026, 7, 3), &inclusive, &c, None).unwrap();
    assert_eq!(result.due_date, day(2026, 7, 4));
}

#[test]
fn input_documents_are_closed_to_unknown_fields() {
    let raw = serde_json::to_value(rule(StatutoryUnit::CalendarDays, 2)).unwrap();
    let mut extra = raw.as_object().unwrap().clone();
    extra.insert("legal_approval".into(), serde_json::json!(true));
    assert!(serde_json::from_value::<LegalClockRule>(serde_json::Value::Object(extra)).is_err());
}

#[test]
fn embedded_calendar_catalog_covers_nz_au_gb_and_skips_national_holidays() {
    let catalog = HolidayCalendarCatalog::embedded();
    let keys: Vec<&str> = catalog.jurisdictions().collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
    for required in [
        "AU", "AU-ACT", "AU-NSW", "AU-VIC", "DE", "ES", "FR", "GB-ENG", "GB-WLS", "GB-NIR",
        "GB-SCT", "NZ", "NZ-AUK", "NZ-WGN",
    ] {
        assert!(keys.contains(&required), "missing {required}");
    }

    let nz = catalog.get("NZ").expect("NZ starter calendar");
    assert!(nz.excluded_dates.contains(&day(2026, 2, 6))); // Waitangi Day
    let mut nz_rule = rule(StatutoryUnit::WorkingDays, 1);
    nz_rule.jurisdiction = "NZ".into();
    // Easter weekend 2026: Good Friday (Apr 3) + Sat/Sun + Easter Monday (Apr 6) -> Apr 7.
    let result = calculate_explicit_deadline(day(2026, 4, 2), &nz_rule, nz, None).unwrap();
    assert_eq!(result.due_date, day(2026, 4, 7));
    assert_eq!(result.validation_status, ValidationStatus::Unvalidated);

    let nz_wgn = catalog.get("NZ-WGN").expect("NZ-WGN regional calendar");
    assert!(nz_wgn.excluded_dates.contains(&day(2026, 1, 19)));
    let mut wgn_rule = rule(StatutoryUnit::WorkingDays, 1);
    wgn_rule.jurisdiction = "NZ-WGN".into();
    let wgn_result =
        calculate_explicit_deadline(day(2026, 1, 16), &wgn_rule, nz_wgn, None).unwrap();
    assert_eq!(wgn_result.due_date, day(2026, 1, 20));
    assert_eq!(wgn_result.validation_status, ValidationStatus::Unvalidated);

    let au = catalog.get("AU").expect("AU starter calendar");
    assert!(au.excluded_dates.contains(&day(2026, 1, 26)));
    let au_act = catalog.get("AU-ACT").expect("AU-ACT regional calendar");
    assert!(au_act.excluded_dates.contains(&day(2026, 6, 1)));

    assert!(catalog.get("GB").is_none());
    let gb = catalog
        .get("GB-ENG")
        .expect("England bank holiday calendar");
    assert!(gb.excluded_dates.contains(&day(2026, 8, 31)));
    let gb_sct = catalog.get("GB-SCT").expect("GB-SCT regional calendar");
    assert!(gb_sct.excluded_dates.contains(&day(2026, 11, 30)));
    let gb_nir = catalog.get("GB-NIR").expect("GB-NIR regional calendar");
    assert!(gb_nir.excluded_dates.contains(&day(2026, 3, 17)));

    let de = catalog.get("DE").expect("DE starter calendar");
    assert!(de.excluded_dates.contains(&day(2026, 10, 3)));
    let fr = catalog.get("FR").expect("FR starter calendar");
    assert!(fr.excluded_dates.contains(&day(2026, 7, 14)));
    let es = catalog.get("ES").expect("ES starter calendar");
    assert!(es.excluded_dates.contains(&day(2026, 10, 12)));
}

#[test]
fn holiday_calendar_catalog_from_json_allows_custom_contributions_and_fails_closed() {
    let custom = r#"{
      "schema_version": 1,
      "calendars": [
        {
          "jurisdiction": "NZ-WGN",
          "revision": "community-2026-v1",
          "valid_from": "2026-01-01",
          "valid_to": "2026-12-31",
          "weekend_days": [5, 6],
          "excluded_dates": ["2026-01-19"]
        }
      ]
    }"#;
    let catalog = HolidayCalendarCatalog::from_json(custom).unwrap();
    let wgn = catalog.get("NZ-WGN").unwrap();
    assert!(wgn.excluded_dates.contains(&day(2026, 1, 19)));

    let single = HolidayCalendar::from_json(
        r#"{
          "jurisdiction": "CA-ON",
          "revision": "community-2026-v1",
          "valid_from": "2026-01-01",
          "valid_to": "2026-12-31",
          "weekend_days": [5, 6],
          "excluded_dates": ["2026-07-01"]
        }"#,
    )
    .unwrap();
    assert_eq!(single.jurisdiction, "CA-ON");

    for bad in [
        r#"{"schema_version":2,"calendars":[]}"#.to_string(),
        r#"{"schema_version":1,"calendars":[]}"#.to_string(),
        r#"{"schema_version":1,"extra":true,"calendars":[]}"#.to_string(),
        // Duplicate jurisdiction
        r#"{"schema_version":1,"calendars":[
            {"jurisdiction":"NZ","revision":"r1","valid_from":"2026-01-01","valid_to":"2026-12-31","weekend_days":[5,6],"excluded_dates":[]},
            {"jurisdiction":"NZ","revision":"r2","valid_from":"2026-01-01","valid_to":"2026-12-31","weekend_days":[5,6],"excluded_dates":[]}
        ]}"#.to_string(),
        // Excluded date outside valid_from..=valid_to
        r#"{"schema_version":1,"calendars":[
            {"jurisdiction":"NZ","revision":"r1","valid_from":"2026-01-01","valid_to":"2026-12-31","weekend_days":[5,6],"excluded_dates":["2027-01-01"]}
        ]}"#.to_string(),
        " ".repeat(MAX_CALENDAR_CATALOG_BYTES + 1),
    ] {
        assert_eq!(
            HolidayCalendarCatalog::from_json(&bad),
            Err(DeadlineRuleError::InvalidCalendar)
        );
    }
}

#[test]
fn calendar_months_preserve_leap_years_and_month_end_clamping() {
    let mut cal = calendar();
    cal.valid_from = day(1999, 1, 1);
    cal.valid_to = day(2028, 12, 31);
    cal.excluded_dates.clear();

    let mut policy = rule(StatutoryUnit::CalendarMonths, 1);
    policy.effective_from = day(1999, 1, 1);

    // Non-leap vs leap February clamping
    let res = calculate_explicit_deadline(day(2024, 1, 31), &policy, &cal, None).unwrap();
    assert_eq!(res.due_date, day(2024, 2, 29));

    // Century / year boundary
    let y2k = calculate_explicit_deadline(day(1999, 12, 31), &policy, &cal, None).unwrap();
    assert_eq!(y2k.due_date, day(2000, 1, 31));

    // Leap day + 12 months clamps to Feb 28; + 48 months stays Feb 29
    policy.period.amount = 12;
    assert_eq!(
        calculate_explicit_deadline(day(2024, 2, 29), &policy, &cal, None)
            .unwrap()
            .due_date,
        day(2025, 2, 28)
    );
    policy.period.amount = 48;
    assert_eq!(
        calculate_explicit_deadline(day(2024, 2, 29), &policy, &cal, None)
            .unwrap()
            .due_date,
        day(2028, 2, 29)
    );

    // Zero months is identity; overflow past Date::MAX returns DateOutOfRange
    policy.period.amount = 0;
    assert_eq!(
        calculate_explicit_deadline(day(2024, 2, 29), &policy, &cal, None)
            .unwrap()
            .due_date,
        day(2024, 2, 29)
    );

    let max = Date::MAX;
    cal.valid_to = max;
    policy.period.amount = 1;
    assert_eq!(
        calculate_explicit_deadline(max, &policy, &cal, None),
        Err(DeadlineRuleError::DateOutOfRange)
    );

    // Omitted effective_to continues deserializing as None.
    let mut raw = serde_json::to_value(&policy).unwrap();
    raw.as_object_mut().unwrap().remove("effective_to");
    let decoded: LegalClockRule = serde_json::from_value(raw).unwrap();
    assert_eq!(decoded.effective_to, None);
}
