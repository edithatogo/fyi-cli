//! Source-backed 2026 regressions; public holiday data is not legal approval.
//! Source URLs and scope limitations live in locales/calendars/v1/provenance.json.

use chrono::NaiveDate;
use fyi_core::deadlines::statutory::{
    calculate_explicit_deadline, DeadlineAdjustment, DeadlineRuleError, HolidayCalendar,
    HolidayCalendarCatalog, LegalClockRule, ReceiptConvention, StatutoryDuration, StatutoryUnit,
    ValidationStatus, MAX_CALENDAR_CATALOG_BYTES,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

fn date(value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
}

fn one_working_day(jurisdiction: &str, receipt: &str) -> Result<NaiveDate, DeadlineRuleError> {
    let catalog = HolidayCalendarCatalog::embedded();
    let rule = LegalClockRule {
        id: "synthetic-arithmetic-test".into(),
        jurisdiction: jurisdiction.into(),
        source_reference: "synthetic; no statutory applicability claim".into(),
        revision: "test-v1".into(),
        effective_from: date("2025-01-01"),
        effective_to: None,
        period: StatutoryDuration {
            amount: 1,
            unit: StatutoryUnit::WorkingDays,
        },
        receipt_convention: ReceiptConvention::ExcludeReceiptDay,
        due_date_adjustment: DeadlineAdjustment::None,
    };
    let result = calculate_explicit_deadline(
        date(receipt),
        &rule,
        catalog.get(jurisdiction).unwrap(),
        None,
    )?;
    assert_eq!(result.validation_status, ValidationStatus::Unvalidated);
    Ok(result.due_date)
}

#[test]
fn official_australian_full_day_lists_include_actual_and_additional_days() {
    // State/territory lists checked 2026-11-17. Local/partial-day exclusions
    // differ and are deliberately tested separately below.
    let catalog = HolidayCalendarCatalog::embedded();
    for (key, expected) in [
        ("AU-ACT", "01-01 01-26 03-09 04-03 04-04 04-05 04-06 04-25 04-27 06-01 06-08 10-05 12-25 12-26 12-28"),
        ("AU-NSW", "01-01 01-26 04-03 04-04 04-05 04-06 04-25 04-27 06-08 10-05 12-25 12-26 12-28"),
        ("AU-VIC", "01-01 01-26 03-09 04-03 04-04 04-05 04-06 04-25 06-08 09-25 11-03 12-25 12-26 12-28"),
        ("AU-QLD", "01-01 01-26 04-03 04-04 04-05 04-06 04-25 05-04 10-05 12-25 12-26 12-28"),
        ("AU-SA", "01-01 01-26 03-09 04-03 04-04 04-05 04-06 04-25 06-08 10-05 12-25 12-26 12-28"),
        ("AU-WA", "01-01 01-26 03-02 04-03 04-05 04-06 04-25 04-27 06-01 09-28 12-25 12-26 12-28"),
        ("AU-NT", "01-01 01-26 04-03 04-04 04-05 04-06 04-25 05-04 06-08 08-03 12-25 12-26 12-28"),
        ("AU-TAS", "01-01 01-26 03-09 04-03 04-06 04-25 06-08 12-25 12-28"),
    ] {
        let expected: BTreeSet<_> = expected
            .split_whitespace()
            .map(|day| date(&format!("2026-{day}")))
            .collect();
        assert_eq!(catalog.get(key).unwrap().excluded_dates, expected, "{key}");
    }
}

#[test]
fn uk_constituent_bank_holiday_lists_include_the_special_scottish_proclamation() {
    let catalog = HolidayCalendarCatalog::embedded();
    for (key, expected) in [
        ("GB-ENG", "01-01 04-03 04-06 05-04 05-25 08-31 12-25 12-28"),
        ("GB-WLS", "01-01 04-03 04-06 05-04 05-25 08-31 12-25 12-28"),
        (
            "GB-SCT",
            "01-01 01-02 04-03 05-04 05-25 06-15 08-03 11-30 12-25 12-28",
        ),
        (
            "GB-NIR",
            "01-01 03-17 04-03 04-06 05-04 05-25 07-13 08-31 12-25 12-28",
        ),
    ] {
        let expected: BTreeSet<_> = expected
            .split_whitespace()
            .map(|day| date(&format!("2026-{day}")))
            .collect();
        assert_eq!(catalog.get(key).unwrap().excluded_dates, expected, "{key}");
    }
}

#[test]
fn corrected_and_observed_holidays_change_arithmetic_only_in_their_scope() {
    for (key, receipt, expected) in [
        ("AU-ACT", "2026-04-24", "2026-04-28"),
        ("AU-NSW", "2026-04-24", "2026-04-28"),
        ("AU-QLD", "2026-04-24", "2026-04-27"),
        ("AU-VIC", "2026-09-24", "2026-09-28"),
        ("GB-SCT", "2026-06-12", "2026-06-16"),
        ("GB-ENG", "2026-06-12", "2026-06-15"),
        ("GB-SCT", "2026-04-02", "2026-04-06"),
        ("GB-WLS", "2026-04-02", "2026-04-07"),
        ("GB-NIR", "2026-07-10", "2026-07-14"),
        ("NZ", "2026-04-24", "2026-04-28"),
        ("NZ", "2026-07-09", "2026-07-13"),
        ("NZ", "2026-12-24", "2026-12-29"),
    ] {
        assert_eq!(one_working_day(key, receipt), Ok(date(expected)), "{key}");
    }
}

#[test]
fn local_show_days_and_replacements_are_explicit() {
    for (key, receipt, expected) in [
        ("AU-QLD", "2026-07-16", "2026-07-17"),
        ("AU-QLD-CAIRNS", "2026-07-16", "2026-07-20"),
        ("AU-QLD-BRISBANE", "2026-08-11", "2026-08-13"),
        ("AU-QLD-CAIRNS", "2026-08-11", "2026-08-12"),
        ("AU-WA", "2026-09-25", "2026-09-29"),
        ("AU-WA-KARRATHA", "2026-09-25", "2026-09-28"),
        ("AU-WA-KARRATHA", "2026-07-31", "2026-08-04"),
        ("AU-WA-HEDLAND", "2026-07-31", "2026-08-04"),
        ("AU-NT-DARWIN", "2026-07-23", "2026-07-27"),
        ("AU-TAS-HOBART", "2026-10-21", "2026-10-23"),
        ("AU-TAS-LAUN", "2026-10-30", "2026-11-03"),
    ] {
        assert_eq!(one_working_day(key, receipt), Ok(date(expected)), "{key}");
    }
}

#[test]
fn nz_anniversaries_use_named_observances_without_geographic_fallback() {
    for (key, receipt, expected) in [
        ("NZ-AUK", "2026-01-23", "2026-01-27"),
        ("NZ-WGN", "2026-01-16", "2026-01-20"),
        ("NZ-TKI", "2026-03-06", "2026-03-10"),
        ("NZ-HKB", "2026-10-22", "2026-10-27"),
        ("NZ-MBH", "2026-10-30", "2026-11-03"),
        ("NZ-NSN", "2026-01-30", "2026-02-03"),
        ("NZ-CAN-NORTH", "2026-11-12", "2026-11-16"),
        ("NZ-CAN-SOUTH", "2026-09-25", "2026-09-29"),
        ("NZ-STL", "2026-04-02", "2026-04-08"),
        ("NZ-OTA-DUN", "2026-03-20", "2026-03-24"),
        ("NZ-OTA-COD", "2026-03-20", "2026-03-24"),
        ("NZ-WTC-GREY", "2026-11-27", "2026-12-01"),
    ] {
        assert_eq!(one_working_day(key, receipt), Ok(date(expected)), "{key}");
    }
}

#[test]
fn unknown_regions_and_unsupported_years_fail_closed() {
    let catalog = HolidayCalendarCatalog::embedded();
    for key in [
        "GB",
        "UK",
        "en-NZ",
        "au-qld",
        "AU-UNKNOWN",
        "NZ-OTA",
        "NZ-WTC",
        "NZ-CIT",
    ] {
        assert!(catalog.get(key).is_none(), "unexpected fallback for {key}");
    }
    for receipt in ["2025-12-31", "2026-12-31", "2027-01-01"] {
        assert_eq!(
            one_working_day("NZ", receipt),
            Err(DeadlineRuleError::CalendarCoverage)
        );
    }
}

fn community_entry() -> Value {
    json!({
        "jurisdiction": "CA-ON", "revision": "community-2026-v1",
        "valid_from": "2026-01-01", "valid_to": "2026-12-31",
        "weekend_days": [5, 6], "excluded_dates": ["2026-07-01"]
    })
}

#[test]
fn duplicate_dates_and_weekends_are_rejected_in_every_deserialization_path() {
    for (field, duplicate) in [
        ("weekend_days", json!([5, 6, 5])),
        ("excluded_dates", json!(["2026-07-01", "2026-07-01"])),
    ] {
        let mut entry = community_entry();
        entry[field] = duplicate;
        let serialized = entry.to_string();
        assert!(serde_json::from_str::<HolidayCalendar>(&serialized).is_err());
        assert_eq!(
            HolidayCalendar::from_json(&serialized),
            Err(DeadlineRuleError::InvalidCalendar)
        );
        let catalog = json!({"schema_version": 1, "calendars": [entry]}).to_string();
        assert_eq!(
            HolidayCalendarCatalog::from_json(&catalog),
            Err(DeadlineRuleError::InvalidCalendar)
        );
    }
}

#[test]
fn contribution_keys_match_schema_and_support_explicit_local_scopes() {
    for key in [
        "nz",
        "NZ-",
        "NZ--WGN",
        "NZ-Wellington",
        "NZ-123456789",
        " NZ",
        "NZ_ABC",
        "NŽ",
        "NZ-É",
        "NZ\n",
        "NZ-AUK\n",
    ] {
        let mut entry = community_entry();
        entry["jurisdiction"] = json!(key);
        assert_eq!(
            HolidayCalendar::from_json(&entry.to_string()),
            Err(DeadlineRuleError::InvalidCalendar)
        );
        let catalog = json!({"schema_version": 1, "calendars": [entry]}).to_string();
        assert_eq!(
            HolidayCalendarCatalog::from_json(&catalog),
            Err(DeadlineRuleError::InvalidCalendar)
        );
    }
    for key in [
        "NZ",
        "CA-ON",
        "AU-QLD-CAIRNS",
        "NZ-CAN-SOUTH",
        "ZZ-01234567",
    ] {
        let mut entry = community_entry();
        entry["jurisdiction"] = json!(key);
        assert_eq!(
            HolidayCalendar::from_json(&entry.to_string())
                .unwrap()
                .jurisdiction,
            key
        );
    }
}

#[test]
fn malformed_single_entries_and_size_boundary_are_checked() {
    for (field, value) in [
        ("revision", json!("  ")),
        ("weekend_days", json!([0, 1, 2, 3, 4, 5, 6])),
        ("weekend_days", json!([7])),
        ("excluded_dates", json!(["2026-02-30"])),
        ("excluded_dates", json!(["2027-01-01"])),
        ("valid_to", json!("2025-12-31")),
        ("unexpected", json!(true)),
    ] {
        let mut entry = community_entry();
        entry[field] = value;
        assert_eq!(
            HolidayCalendar::from_json(&entry.to_string()),
            Err(DeadlineRuleError::InvalidCalendar)
        );
    }
    let serialized = community_entry().to_string();
    let bounded = format!(
        "{}{}",
        " ".repeat(MAX_CALENDAR_CATALOG_BYTES - serialized.len()),
        serialized
    );
    assert!(HolidayCalendar::from_json(&bounded).is_ok());
    assert_eq!(
        HolidayCalendar::from_json(&(bounded + " ")),
        Err(DeadlineRuleError::InvalidCalendar)
    );
}
