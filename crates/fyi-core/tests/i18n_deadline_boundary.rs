//! These tests verify arithmetic boundaries and delegation, not statutory validity.

use chrono::{Days, NaiveDate};
use fyi_core::deadlines::{self, DeadlineInput, WorkingDayRule};
use fyi_core::i18n::{LocalizationEngine, SUPPORTED_LOCALES};
use fyi_core::jurisdiction::InstanceRegistry;

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

#[test]
fn locale_wrappers_preserve_canonical_weekday_arithmetic() {
    for locale in SUPPORTED_LOCALES {
        let engine = LocalizationEngine::new(locale);
        for start in [date(2026, 7, 3), date(2026, 7, 4), date(2026, 7, 6)] {
            for days in [0_u32, 1, 2, 5, 20] {
                let expected =
                    deadlines::add_working_days(start, days, WorkingDayRule::WeekdaysOnly);
                assert_eq!(
                    engine.try_add_working_days(start, days as usize).unwrap(),
                    expected,
                    "{locale}: {start} plus {days} working days"
                );
                assert_eq!(engine.add_working_days(start, days as usize), expected);
            }
        }
        assert_eq!(
            engine.try_add_working_days(date(2026, 7, 3), 2).unwrap(),
            date(2026, 7, 7)
        );
    }
}

#[test]
fn instance_wrappers_preserve_existing_default_and_nonpositive_period_policy() {
    let registry = InstanceRegistry::embedded().unwrap();
    let mut instance = registry.get("nz-fyi").unwrap().clone();
    let start = date(2026, 7, 3);
    let cases = [
        (None, date(2026, 7, 31)),
        (Some(i32::MIN), start),
        (Some(-1), start),
        (Some(0), start),
        (Some(1), date(2026, 7, 6)),
        (Some(20), date(2026, 7, 31)),
        (Some(30), date(2026, 8, 14)),
    ];
    for locale in SUPPORTED_LOCALES {
        let engine = LocalizationEngine::new(locale);
        for (period, expected) in cases {
            instance.foi_law.statutory_deadline_days = period;
            assert_eq!(
                engine.try_deadline_for_instance(start, &instance).unwrap(),
                expected,
                "{locale}: catalog period {period:?}"
            );
            assert_eq!(engine.deadline_for_instance(start, &instance), expected);
            assert_eq!(
                deadlines::calculate_deadline_for_instance(start, &instance).due_date,
                expected
            );
        }
    }
}

#[test]
fn message_locale_does_not_select_the_jurisdiction_or_its_period() {
    let registry = InstanceRegistry::embedded().unwrap();
    let start = date(2026, 7, 3);
    for locale in SUPPORTED_LOCALES {
        let engine = LocalizationEngine::new(locale);
        for instance_id in [
            "nz-fyi", "au-rtk", "uk-wdtk", "de-fds", "fr-cada", "es-tdas",
        ] {
            let instance = registry.get(instance_id).unwrap();
            let expected = deadlines::calculate_deadline_for_instance(start, instance);
            assert_eq!(
                engine.try_deadline_for_instance(start, instance).unwrap(),
                expected.due_date,
                "message locale {locale} with instance {instance_id}"
            );
            assert_eq!(expected.instance_id.as_deref(), Some(instance_id));
        }
    }
    let german = registry.get("de-fds").unwrap();
    assert_eq!(
        LocalizationEngine::new("en-NZ")
            .try_deadline_for_instance(start, german)
            .unwrap(),
        date(2026, 8, 14)
    );
    let new_zealand = registry.get("nz-fyi").unwrap();
    assert_eq!(
        LocalizationEngine::new("de-DE")
            .try_deadline_for_instance(start, new_zealand)
            .unwrap(),
        date(2026, 7, 31)
    );
}

#[test]
fn calendar_day_selection_remains_in_the_canonical_deadline_api() {
    let start = date(2026, 7, 3);
    let input = DeadlineInput::new(start, 2).with_rule(WorkingDayRule::CalendarDays);
    let calendar_deadline = deadlines::calculate_deadline(&input);
    assert_eq!(calendar_deadline.due_date, date(2026, 7, 5));
    assert_eq!(
        calendar_deadline.working_day_rule,
        WorkingDayRule::CalendarDays
    );
    for locale in SUPPORTED_LOCALES {
        let engine = LocalizationEngine::new(locale);
        assert_eq!(
            engine.try_add_working_days(start, 2).unwrap(),
            date(2026, 7, 7)
        );
        assert_eq!(deadlines::calculate_deadline(&input), calendar_deadline);
    }
}

#[test]
fn unrepresentable_counts_fail_without_wrapping_or_iterating_the_count() {
    let start = date(2026, 7, 3);
    for locale in SUPPORTED_LOCALES {
        let engine = LocalizationEngine::new(locale);
        assert!(engine.try_add_working_days(start, usize::MAX).is_err());
        assert!(engine
            .try_add_working_days(start, u32::MAX as usize)
            .is_err());
    }
}

#[cfg(target_pointer_width = "64")]
#[test]
fn count_just_above_u32_max_is_rejected_instead_of_becoming_zero() {
    let days = usize::try_from(u64::from(u32::MAX) + 1).unwrap();
    assert!(LocalizationEngine::new("en-NZ")
        .try_add_working_days(date(2026, 7, 3), days)
        .is_err());
}

#[test]
fn zero_days_at_date_boundaries_succeeds_and_advancing_past_max_fails() {
    for locale in SUPPORTED_LOCALES {
        let engine = LocalizationEngine::new(locale);
        for start in [NaiveDate::MIN, NaiveDate::MAX] {
            assert_eq!(engine.try_add_working_days(start, 0).unwrap(), start);
        }
        assert!(engine.try_add_working_days(NaiveDate::MAX, 1).is_err());
    }
}

#[test]
fn exact_remaining_weekday_capacity_near_max_is_accepted_without_saturation() {
    let engine = LocalizationEngine::new("en-GB");
    for days_before_max in 0..=15 {
        let start = NaiveDate::MAX
            .checked_sub_days(Days::new(days_before_max))
            .unwrap();
        // The canonical counter only traverses this bounded suffix, not the date range.
        let capacity =
            deadlines::working_days_between(start, NaiveDate::MAX, WorkingDayRule::WeekdaysOnly);
        for count in 0..=capacity {
            let expected = deadlines::add_working_days(start, count, WorkingDayRule::WeekdaysOnly);
            assert_eq!(
                engine.try_add_working_days(start, count as usize).unwrap(),
                expected,
                "{start} plus {count}, available capacity {capacity}"
            );
        }
        assert!(
            engine
                .try_add_working_days(start, capacity as usize + 1)
                .is_err(),
            "{start} must reject a count larger than remaining capacity {capacity}"
        );
    }
    let penultimate = NaiveDate::MAX.pred_opt().unwrap();
    if deadlines::is_weekday(NaiveDate::MAX) {
        assert_eq!(
            engine.try_add_working_days(penultimate, 1).unwrap(),
            NaiveDate::MAX
        );
    } else {
        assert!(engine.try_add_working_days(penultimate, 1).is_err());
    }
}

#[test]
fn instance_adapter_checks_range_after_applying_canonical_period_policy() {
    let registry = InstanceRegistry::embedded().unwrap();
    let mut instance = registry.get("nz-fyi").unwrap().clone();
    for locale in SUPPORTED_LOCALES {
        let engine = LocalizationEngine::new(locale);
        for period in [Some(-1), Some(0)] {
            instance.foi_law.statutory_deadline_days = period;
            assert_eq!(
                engine
                    .try_deadline_for_instance(NaiveDate::MAX, &instance)
                    .unwrap(),
                NaiveDate::MAX
            );
        }
        for period in [None, Some(1), Some(i32::MAX)] {
            instance.foi_law.statutory_deadline_days = period;
            assert!(engine
                .try_deadline_for_instance(NaiveDate::MAX, &instance)
                .is_err());
        }
        instance.foi_law.statutory_deadline_days = Some(i32::MAX);
        assert!(engine
            .try_deadline_for_instance(date(2026, 7, 3), &instance)
            .is_err());
    }
}
