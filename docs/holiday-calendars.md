# Holiday calendars and community contributions

The embedded catalog supplies **2026 public/bank-holiday dates for explicitly
named scopes**. It is used by the Rust `deadlines::statutory` API when a caller
selects a calendar and supplies a matching legal-clock rule. It does not replace
the legacy weekday calculator, choose a law from a display language, or activate
live requests. All calculation results remain `ValidationStatus::Unvalidated`.

Sources were checked on **17 November 2026**. There are **37 source-checked
calendars, 444 named date records and 30 source records**. Three older DE/ES/FR
starters remain available without this source review. Source checking and
automated arithmetic tests do not constitute human legal approval.

## Files and coverage

The canonical files are under
[`crates/fyi-core/locales/calendars/v1/`](../crates/fyi-core/locales/calendars/v1/):

| File | Purpose |
| --- | --- |
| `catalog.json` | Runtime v1 calendars: exact scope, revision, coverage, weekends and complete exclusion arrays for the declared scope |
| `catalog.schema.json` | Structural contribution contract |
| `provenance.json` | Scope limitations, source-check date, publishers/URLs, named holidays and observed-date relationships |
| `provenance.schema.json` | Strict evidence contract; legal validation is fixed to `unvalidated` |

Every calendar covers **2026-01-01 through 2026-12-31 inclusive** and uses Saturday
and Sunday as its weekend. Calculations that cross the boundary fail with
`CalendarCoverage`; dates are never extrapolated into another year. Future data
requires its own source check and revision. Recheck official publications for
exceptional declarations before operational use even within this period.

The provenance file describes the limits of each scope. A national or state base
can be complete for its published base list while still being insufficient for a
particular locality, authority, employment arrangement or statute.

## Supported scopes

### New Zealand

These profiles use weekday-observed public holidays. Actual Anzac Day and Boxing
Day fall on Saturdays in 2026 and are already excluded by the weekend definition;
their Monday observances are named separately with `observed_for` evidence.
The national set includes Matariki on 10 July. Provincial anniversary boundaries
follow historical observance and local custom, not automatic modern-region mapping.

| Key | Scope and additional anniversary |
| --- | --- |
| `NZ` | National weekday-observed holidays; no provincial anniversary |
| `NZ-AUK` | Auckland Anniversary observance, 26 January |
| `NZ-WGN` | Wellington Anniversary observance, 19 January |
| `NZ-TKI` | Taranaki Anniversary observance, 9 March |
| `NZ-HKB` | Hawke's Bay observance, corroborated in Hastings, 23 October |
| `NZ-MBH` | Marlborough Anniversary observance, 2 November |
| `NZ-NSN` | Nelson Anniversary observance, corroborated in Tasman, 2 February |
| `NZ-CAN-NORTH` | Northern/central Canterbury Show Day observance, 13 November |
| `NZ-CAN-SOUTH` | South Canterbury observance, corroborated in Mackenzie, 28 September |
| `NZ-STL` | Southland observance, corroborated in Invercargill/Bluff, 7 April |
| `NZ-OTA-DUN` | Dunedin Otago Anniversary observance, 23 March |
| `NZ-OTA-COD` | Central Otago council localities named in the source, 23 March |
| `NZ-WTC-GREY` | Greymouth observance, 30 November |

The [Employment New Zealand schedule](https://www.employment.govt.nz/leave-and-holidays/public-holidays/public-holidays-and-anniversary-dates)
is the national date source. Council corroboration is linked for each regional
event. The South Canterbury council listing was available through its search
index; direct retrieval returned HTTP 403. Greymouth's 2026 date is listed by
Employment NZ and follows the council's published Monday-nearest-1-December rule;
the council page's year-specific example is older. Those qualifications are
recorded with the calendar evidence.

There is no whole-Otago or whole-Westland fallback. Chatham Islands' national
listing has not yet been corroborated locally, so `NZ-CIT` is not embedded. The
NZ OIA/LGOIMA summer clock exclusion is a statutory rule concern under #65, not
an employment public holiday to insert indiscriminately into these calendars.

### Australia

`AU` contains only dates common to the eight state/mainland-territory base lists.
It is not a complete federal FOI calendar. External territories are outside this
dataset. The state lists follow the relevant government publications, linked per
date in the provenance file and cross-checked against the
[Fair Work Ombudsman 2026 list](https://www.fairwork.gov.au/employment-conditions/public-holidays/2026-public-holidays).

| Key | Scope and exclusions to check |
| --- | --- |
| `AU-ACT` | Territory full-day public holidays; excludes the bank-only holiday |
| `AU-NSW` | Statewide full-day dates; local declarations require a local calendar; excludes the bank-only holiday |
| `AU-VIC` | Default state calendar with Melbourne Cup on 3 November; not applicable where a non-metropolitan locality replaces that date |
| `AU-QLD` | Statewide full-day dates; no local show day or partial Christmas Eve |
| `AU-SA` | Statewide full-day dates; no partial Christmas Eve/New Year's Eve |
| `AU-WA` | Default state calendar with King's Birthday on 28 September; use a replacement profile where required |
| `AU-NT` | Territory full-day dates; no regional show day or partial Christmas Eve/New Year's Eve |
| `AU-TAS` | General statewide dates; no municipal holidays or public-service-only Easter Tuesday |

ACT and NSW include the additional Anzac Day holiday on 27 April. Victoria
includes the Friday before the AFL Grand Final on 25 September. Queensland,
Victoria, South Australia, the Northern Territory and Tasmania do not acquire
27 April merely because another state observes it.

The following local entries contain the entire applicable base list plus their
specific changes. They are independently selectable; loading a state base never
automatically selects a local entry.

| Key | Local difference from the state/territory base |
| --- | --- |
| `AU-QLD-BRISBANE` | Add Royal Queensland Show, 12 August |
| `AU-QLD-CAIRNS` | Add Cairns Annual Show, 17 July, for the Cairns Regional Council area |
| `AU-WA-KARRATHA` | Replace 28 September with 3 August for King's Birthday |
| `AU-WA-HEDLAND` | Same replacement for the Port Hedland local government district |
| `AU-TAS-HOBART` | Add Royal Hobart Regatta, 9 February, and Royal Hobart Show, 22 October |
| `AU-TAS-LAUN` | Add Royal Launceston Show, 8 October, and Recreation Day, 2 November |
| `AU-NT-ALICE` | Add Alice Springs Show Day, 3 July, within the official show region |
| `AU-NT-TENNANT` | Add Tennant Creek Show Day, 10 July, within the official show region |
| `AU-NT-KATH` | Add Katherine Show Day, 17 July, within the official show region |
| `AU-NT-DARWIN` | Add Darwin Show Day, 24 July, within the official show region |
| `AU-NT-BORRO` | Add Borroloola Show Day, 14 August, within the official show region |

The Cairns entry is not a calendar for all Far North Queensland. Tasmania's
public-service-only holidays and the NT show-region boundaries require explicit
applicability checks. A new local Victorian replacement must remove Melbourne
Cup Day before adding its replacement; a union alone is wrong.

**Partial-day limitation:** Queensland Christmas Eve begins at 18:00; South
Australia and the Northern Territory have Christmas Eve and New Year's Eve
holidays beginning at 19:00. The current API represents whole dates, so it does
not turn these partial days into whole-day exclusions. If the selected statute
or authority needs partial-day semantics, this API alone is insufficient.

### United Kingdom

| Key | Published bank-holiday schedule |
| --- | --- |
| `GB-ENG` | England, eight dates |
| `GB-WLS` | Wales, the same eight dates |
| `GB-SCT` | Scotland, ten dates including the exceptional 15 June World Cup holiday |
| `GB-NIR` | Northern Ireland, ten dates including St Patrick's Day and the 13 July substitute day |

The source is [GOV.UK's bank-holiday schedule](https://www.gov.uk/bank-holidays),
with the exceptional Scottish date also supported by the
[Royal Proclamation](https://privycouncil.independent.gov.uk/wp-content/uploads/2026/02/Scotland-Bank-Holiday-Proclamation.pdf).
[Scottish Government guidance](https://www.gov.scot/publications/bank-holidays/pages/current-year/)
distinguishes these national bank holidays from local council public holidays.
Easter Monday is not a Scotland-wide bank holiday.

**Migration:** the old embedded `GB` entry actually contained England/Wales dates.
It has been removed instead of silently reinterpreted. Select `GB-ENG` or
`GB-WLS` explicitly, with a matching rule jurisdiction. Caller-supplied v1
catalogs can still declare their own explicit `GB` scope, but the library does
not provide a UK-wide default.

### Legacy European starters

`DE`, `ES` and `FR` retain their prior arrays and revisions. They are expressly
listed under `unreviewed_calendars`, with a reason. This exception cannot be used
to admit a new unreviewed NZ/AU/UK calendar. Their national/regional source and
statutory review remains open under epic #39.

## Loading a community calendar

The public runtime format remains schema v1. Supply an explicit JSON string to
`HolidayCalendarCatalog::from_json` or `HolidayCalendar::from_json`; both operate
only in memory. Here is a **synthetic format example**, not a real holiday list:

```json
{
  "schema_version": 1,
  "calendars": [{
    "jurisdiction": "ZZ-DEMO",
    "revision": "synthetic-example-2026-v1",
    "valid_from": "2026-01-01",
    "valid_to": "2026-12-31",
    "weekend_days": [5, 6],
    "excluded_dates": ["2026-07-01"]
  }]
}
```

```rust
use fyi_core::deadlines::statutory::HolidayCalendarCatalog;

// `custom_json` is the caller's explicitly selected UTF-8 data, read by the caller.
let catalog = HolidayCalendarCatalog::from_json(custom_json)?;
let calendar = catalog.get("ZZ-DEMO").ok_or("missing explicit scope")?;
// Pass `calendar` to calculate_explicit_deadline with a matching LegalClockRule.
```

The loader rejects more than 256 KiB, unknown fields, duplicate jurisdictions,
duplicate excluded dates/weekend codes, reversed coverage, dates outside coverage
and a seven-day weekend. Keys must match
`^[A-Z]{2}(?:-[A-Z0-9]{1,8})*$`. Local suffixes are project-defined identifiers.
Raw arithmetic callers can still construct the existing `HolidayCalendar`
structure directly; the public fields and serialization format are unchanged.

Bare v1 parsing validates structure and arithmetic bounds. It does not establish
provenance or completeness. To propose an embedded contribution, use the source
contract below. A new custom file is not loaded or merged automatically.

## Contributing or updating dates

1. **Define the scope and covered period.** Distinguish a country base, state
   base, named observance, local area, bank holiday or public holiday. Check
   whether local dates replace a base date. Identify any partial-day, sector or
   authority exclusions that this engine cannot represent.
2. **Check current primary sources.** Prefer the responsible government,
   gazette or proclamation and local authority. Record publisher, title, HTTPS
   URL, access date and applicable date range. A source access date is not a
   human approval or an effective date of law. Record limitations honestly.
3. **Create a full explicit calendar.** Add all applicable dates within the
   declared scope and period, including observed/substitute and exceptional
   dates. Do not inherit or union local dates at runtime. Bump `revision` when
   dates, coverage, weekend assumptions, scope or evidence change.
4. **Add matching provenance.** Copy the exact jurisdiction, revision, coverage
   and weekend assumptions. Supply `coverage_kind`, a scope description,
   limitations and source IDs. Every excluded date needs a named `events` record
   with `kind` (`national`, `regional`, `observed` or `exceptional`) and source
   IDs. An observed event can record its actual date as `observed_for`. Distinct
   named holidays can share a date; exact duplicate events are rejected.
5. **Add independent expectations.** Include a published-date regression and a
   working-day result around a regional, observed or exceptional date. Check
   that nearby regions do not receive the holiday and that replacement dates
   are removed. Test coverage boundaries. Do not derive expected values by
   rereading the catalog under test.
6. **Run the offline checks and normal required PR checks.** Open a pull request
   linking the issue, changed scopes, sources, date differences and revision.
   Preserve `legal_validation: "unvalidated"` and the separate statutory-review
   gates. No live portal submission is needed for this work.

```bash
python scripts/verify_holiday_calendars.py
python -m pytest tests/test_holiday_calendar_catalog_schema.py tests/test_holiday_calendar_provenance.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p fyi-core --test holiday_calendar_data --test statutory_deadline_contract --locked
cargo test --workspace --all-features --locked
```

The validator also accepts `--catalog path.json --provenance evidence.json` for
reviewing an explicit local contribution. It makes no network request. It checks
schema validity, duplicate JSON fields, source identities, matching revisions,
coverage/weekends, date-set conservation, cited event dates within source
coverage, and source-check chronology. It cannot prove that an unlisted holiday
does not exist; reviewers must check completeness over the whole covered period.
Passing it means the evidence is internally consistent; reviewers still need to
inspect the sources and scope. Existing Python CI runs its positive
and mutation tests, including the embedded dataset.

To add another year, extend the coverage and enumerate every covered year's
dates with supporting evidence, or supply a separate caller-selected catalog.
One v1 catalog permits only one entry per exact key; duplicate yearly entries
are rejected. There is no extrapolation, network update or silent fallback.
