# Explicit statutory deadline contract (issue #65)

The Rust API `fyi_core::deadlines::statutory::calculate_explicit_deadline`
calculates **unvalidated arithmetic**, not an authoritative legal due date.
It does not send requests, read network data, update persistence, infer a
jurisdiction from display locale, or silently choose a holiday calendar.

## Inputs and semantics

- The caller supplies a **receipt date**, not automatically the timestamp at
  which a draft was submitted. Statutory receipt, deemed receipt, time zones,
  service rules and cut-off times require separate legal interpretation.
- `LegalClockRule` explicitly names a regime, exact jurisdiction/subdivision,
  source reference, version, receipt-effective dates, receipt-day convention,
  duration and terminal-date adjustment.
- `StatutoryDuration` distinguishes working days, calendar days and calendar
  months. Months use Chrono's end-of-month clamping: a one-month period from
  31 January 2026 yields 28 February 2026 **before** any adjustment. Whether
  that convention is lawful in a particular regime must be reviewed.
- `HolidayCalendar` must match the exact jurisdiction and cover the entire
  calculation. It supplies weekend day codes (Monday=0 through Sunday=6),
  excluded dates (including **observed and regional** holidays) and a revision.
  Missing coverage is an error, not permission to use a national default.
- `DeadlineExtension` records an explicit reason, authority and duration.
  Extensions apply after the base period and before terminal adjustment.
  It is the caller's responsibility to verify whether a given extension is
  authorized, applicable, measured in that unit and applied in that order.
- `DeadlineAdjustment::NextWorkingDay` is opt-in. Calendar-day arithmetic
  does not automatically exclude weekends or holidays during counting.
  Inclusive calendar-month counting is rejected because its semantics depend
  on the governing law and cannot safely be guessed.
- All errors are typed, including different jurisdictions, ineffective rules,
  missing calendar coverage, invalid metadata, unrepresentable dates and
  excessive periods. Computed output has only
  `ValidationStatus::Unvalidated`; a claimed approved status cannot deserialize.

### Synthetic example (not a statute)

```rust
use fyi_core::deadlines::statutory::{
    calculate_explicit_deadline, DeadlineAdjustment, HolidayCalendar, LegalClockRule,
    ReceiptConvention, StatutoryDuration, StatutoryUnit,
};
use std::collections::BTreeSet;
use time::{Date, Month};

let date = |y, m: u8, d| Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap();
let calendar = HolidayCalendar {
    jurisdiction: "TEST-REGION".into(),
    revision: "synthetic-v1".into(),
    valid_from: date(2026, 1, 1),
    valid_to: date(2027, 12, 31),
    weekend_days: BTreeSet::from([5, 6]),
    excluded_dates: BTreeSet::from([date(2026, 7, 6)]),
};
let rule = LegalClockRule {
    id: "synthetic-rule".into(),
    jurisdiction: "TEST-REGION".into(),
    source_reference: "synthetic example only".into(),
    revision: "test-v1".into(),
    effective_from: date(2026, 1, 1),
    effective_to: None,
    period: StatutoryDuration { amount: 2, unit: StatutoryUnit::WorkingDays },
    receipt_convention: ReceiptConvention::ExcludeReceiptDay,
    due_date_adjustment: DeadlineAdjustment::None,
};
let result = calculate_explicit_deadline(date(2026, 7, 3), &rule, &calendar, None)?;
assert_eq!(result.due_date, date(2026, 7, 8));
# Ok::<(), fyi_core::deadlines::statutory::DeadlineRuleError>(())
```

## Acceptance still outstanding

This API is a bounded computational foundation, **not closure** of #65/#39.

1. #66 owns evidence-backed, maintained and reviewed national, state/regional,
   observed and exceptional closure calendars for NZ, AU, UK, DE, FR and ES.
   Each record needs a source, effective years and independent verification.
2. The jurisdiction/deadline owner must publish authoritative statutory-rule
   profiles (including precise triggering events, durations, receipt treatment,
   extension powers, public holidays and terminal-day conventions) and
   date-specific, source-referenced golden examples. Some regimes use **months**,
   not the `instances.toml` numeric day estimates.
3. Add integration to CLI/MCP/persistence **only after** verified rule/calendar
   selection can fail closed for unknown/ambiguous jurisdictions and stale data.
   The existing `deadlines.rs` `DeadlineInput` and
   `calculate_deadline_for_instance` remain compatibility **estimates** and do
   not use these calendars. Their outputs must not be represented as legal advice.
4. Verify human legal/translation/calendar acceptance, all six jurisdictions,
   end-to-end reminders, rule-update lifecycle and measured **<1 ms** deadline
   runtime before marking the parent Conductor phase accepted.

Tests in `crates/fyi-core/tests/statutory_deadline_contract.rs` are deliberately
synthetic: deterministic arithmetic regression evidence, not legal validation.
