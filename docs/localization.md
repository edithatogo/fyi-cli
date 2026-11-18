# Localization usage and contribution guide

Use this guide to [add a supported locale](#add-a-supported-locale),
[customize a request template](#customize-request-templates), or
[contribute a holiday calendar](#contribute-an-embedded-calendar).
Examples use the Rust `fyi-core` API and run without contacting a service.

## Scope and review status

`fyi_core::i18n` formats messages. It does not choose a jurisdiction, verify a legal
citation, establish a statutory deadline, persist a request or submit anything.
Successful rendering is **not** evidence of legal validity or human translation
approval. The embedded text is an unreviewed rendering baseline.

The [i18n-localization-framework plan](../.conductor/tracks/i18n-localization-framework/plan.md)
tracks implemented behavior separately from outstanding translation, calendar and
legal review. The guide describes the available APIs; it does not mark those
reviews complete or advertise runtime locale overrides that are not implemented.

## Supported identifiers and deterministic fallback

| Requested identifier | Lookup chain |
| --- | --- |
| `en-NZ` | `en-NZ` → `en` → `base` |
| `en-AU` | `en-AU` → `en` → `base` |
| `en-GB` | `en-GB` → `en` → `base` |
| `de-DE` | `de-DE` → `base` |
| `fr-FR` | `fr-FR` → `base` |
| `es-ES` | `es-ES` → `base` |

`base` uses the same English resource as `en`; there is one English source of
truth. Lookup is ordered, and the rendered message reports its actual source
layer. An existing message that fails to format returns an error; it does not
silently switch languages. An identifier absent from every layer is an error.

`LocalizationEngine::try_new` requires one of the six supported identifiers after
`unic-langid` normalizes case and underscore separators (`EN_nz` → `en-NZ`). It
distinguishes malformed identifiers from unsupported ones. Parsing establishes
structural well-formedness, not registration of every language subtag.

The source-compatible `new` constructor permits fallback. Other English regions,
including the existing `en-IE` and `en-US` instances, use neutral `en`. Other
German/French/Spanish identifiers use `de-DE`/`fr-FR`/`es-ES` respectively. Unknown
languages and malformed identifiers use `en`; neither becomes Spanish. Inspect
`resolution()` to distinguish exact, language, unknown-language and invalid-input
resolution. No environment locale, network response or filesystem ordering affects
selection. A message language never chooses or changes the legal jurisdiction.

## Versioned resources and strict rendering

Resources are embedded from `crates/fyi-core/locales/v1/`. `version.json` carries
the supported resource `format_version` (currently 1) and independent content
`revision`. Increment the revision whenever embedded wording changes. Change the
format version when its contract changes incompatibly; unsupported versions fail.

```rust
use fyi_core::i18n::LocalizationEngine;
use std::collections::BTreeMap;

let engine = LocalizationEngine::try_new("en-NZ")?;
let args = BTreeMap::from([("authority".into(), "Example Council".into())]);
let message = engine.render_message("salutation", &args)?;
assert_eq!(message.text, "Kia ora Example Council");
# Ok::<(), fyi_core::i18n::I18nError>(())
```

`MessageBundle::from_ftl` validates the format version, revision, locale, complete
Fluent syntax and duplicate entries. `require_messages` detects missing values.
Formatting checks Fluent's error collection and rejects partial output. Missing
variables, missing messages and unresolved expressions are errors. Extra variables
are ignored. Error messages do not include supplied variable values or resource
contents.

Fluent handles message grammar and variable substitution. `unic-langid` handles
structured language identifiers. These dependencies replace ad hoc prefix matching
and prepare translator-maintained resources without inventing a template language.
The engine implements its own explicit message-layer fallback: Fluent's locale
list alone does not provide resource lookup across bundles. See the primary
[Fluent bundle API](https://docs.rs/fluent-bundle/latest/fluent_bundle/bundle/struct.FluentBundle.html)
and [language identifier API](https://docs.rs/unic-langid/latest/unic_langid/struct.LanguageIdentifier.html).

## Data-only interpolation and Unicode

Variable values are strings and are interpolated once. Braces, Fluent-looking text,
shell-looking text and markup in a variable stay literal. No callback functions
are registered, and formatting performs no I/O or arbitrary template execution.
An unknown function expression fails instead of executing application code.

UTF-8 user text is preserved, including combining marks, Māori macrons, non-Latin
names and emoji. There is no NFC/NFKC normalization or removal of user-supplied
characters. Precomposed and decomposed spellings therefore remain byte-distinct.
Identifier normalization is separate from content normalization. Fluent's automatic
directional isolation is disabled to preserve the established plain-text output;
RTL display support remains outside the locale coverage promise.

Message output is plain text, not trusted HTML, Markdown, a shell command, SQL or
an email header. Consumers must apply escaping appropriate to their final display
or transport context. Nothing in the plain-text renderer executes those contexts.

## Compatibility

`LocaleBundle` retains its serialized `{locale, translations}` shape, lowercase
legacy locale field, `{authority}` placeholder and caller-supplied `translate`
fallback. New consumers should use strict rendering rather than silent key fallback.
`LocalizationEngine::locale()` retains the original caller-supplied identifier.
Existing string-returning scaffolds remain unvalidated; the legacy generic scaffold
keeps its sign-off on the final line for `drafting.rs`.

## Typed request and letter contracts

`RequestTemplateInput` supplies the authority, requester, title, body, variant,
register and optional date/reference/review information. `LegalCitation` borrows
the existing `Instance`/`FoiLaw` fields. Rendering in another language preserves
the supplied jurisdiction, law, citation and request term. A catalog citation is
displayed as supplied, even when it repeats the law name; the renderer does not
silently rewrite legal source data.

```rust
use fyi_core::i18n::{LocalizationEngine, OutputFormat, RequestTemplateInput};
use fyi_core::jurisdiction::InstanceRegistry;

let catalog = InstanceRegistry::embedded()?;
let instance = catalog.get("nz-fyi").expect("embedded NZ fixture");
let input = RequestTemplateInput::new(
    "Example Council", "Example Requester", "Budget records",
    "Please provide the adopted budget and its annexes.", instance,
);
let draft = LocalizationEngine::try_new("en-NZ")?
    .render_request(&input, OutputFormat::PlainText)?;
assert_eq!(draft.title, "Budget records");
# Ok::<(), anyhow::Error>(())
```

| Variant | Required additional inputs | Effect |
| --- | --- | --- |
| `Request` | None | Retains the supplied title; optional reference is displayed. |
| `FollowUp` | Request reference | Localized follow-up title and opening. |
| `InternalReview` | Request reference | Localized request for internal review. |
| `Appeal` | Request reference and supplied review contact | Localized appeal opening addressing that contact. |

Formal and informal registers choose localized salutations and sign-offs. All four
variants and both registers have resources for the six baseline locales. Wording
is a rendering fixture pending language and legal review, not an approved form or
proof that the selected remedy is available.

Required fields must be nonempty. Single-line fields reject control characters,
Unicode line separators and paragraph separators. The body and review instructions
permit ordinary multiline text, preserving supplied Unicode; terminal control
sequences are rejected. An optional value supplied as empty is an error. `date` is
caller-formatted text; neither date localization nor statutory calculation is
inferred from it. New consumers should use the typed API, since legacy scaffolds
retain their permissive input signatures for source compatibility.

Review information is opt-in for ordinary requests. `ReviewInformation::from_instance`
can reuse the catalog's existing review contact, but choosing it does not establish
applicability. The caller may supply instructions separately. The output contains
`RenderedUnvalidated`, selected locale/fallback information, content revision,
jurisdiction identifier and per-message source layers. There is no validated or
submitted state in this renderer.

`PlainText` preserves text exactly. `HtmlText` escapes the complete rendered title
and body once (`&`, `<`, `>`, quotes and apostrophes). It is text for an HTML display
context, not an HTML document or email header. Provenance fields remain structured
data and need context-appropriate handling by their consumer.

## Customize request templates

### Change a particular letter

Set fields on `RequestTemplateInput` to select an existing variant and register,
provide a reference or caller-formatted date, and supply your own body. This does
not require editing resources or rebuilding the library:

```rust
use fyi_core::i18n::{
    LetterRegister, LocalizationEngine, OutputFormat, RequestTemplateInput,
    TemplateVariant, ValidationStatus,
};
use fyi_core::jurisdiction::InstanceRegistry;

let registry = InstanceRegistry::embedded()?;
let instance = registry.get("nz-fyi").expect("embedded NZ fixture");
let mut input = RequestTemplateInput::new(
    "Example Council", "Example Requester", "Budget records",
    "Please let me know the progress of this request.", instance,
);
input.variant = TemplateVariant::FollowUp;
input.register = LetterRegister::Informal;
input.request_reference = Some("EXAMPLE-42");
input.date = Some("17 November 2026");

let draft = LocalizationEngine::try_new("en-NZ")?
    .render_request(&input, OutputFormat::PlainText)?;
assert_eq!(draft.title, "Follow-up: Budget records");
assert!(draft.body.contains("EXAMPLE-42"));
assert!(draft.body.ends_with("Kind regards\nExample Requester"));
assert_eq!(draft.jurisdiction_id, "nz-fyi");
assert_eq!(draft.validation, ValidationStatus::RenderedUnvalidated);
# Ok::<(), Box<dyn std::error::Error>>(())
```

An appeal also requires `input.review = Some(ReviewInformation { authority: ...,
instructions: ... })`. Review information remains an explicit caller decision;
changing the variant does not establish review rights or send the letter.

### Change bundled wording

Edit `crates/fyi-core/locales/v1/<locale>/common.ftl`. English regions inherit
shared wording from `en/common.ftl`; edit that file for a shared change or add the
same message key to a regional file for a regional override. Other baseline
languages have their own complete resources. For example, this is a possible
English `follow_up_body` wording change:

```ftl
follow_up_body = Please send an update on { $request_reference }, requested under { $legal_citation }.
```

Keep message identifiers and Fluent variable names stable. The current typed
renderer uses these message contracts:

| Message keys | Variables used by the bundled wording |
| --- | --- |
| `salutation`, `informal_salutation` | `authority` |
| `closing`, `informal_closing` | None |
| `request_title`, `follow_up_title`, `internal_review_title`, `appeal_title` | `title` |
| `legal_citation` | `law_name`, `citation` |
| `request_body` | `legal_citation`, `request_term` |
| `follow_up_body`, `internal_review_body` | `request_reference`, `legal_citation` |
| `appeal_body` | `review_authority`, `request_reference`, `legal_citation` |
| `date_line` | `date` |
| `reference_line` | `request_reference` |
| `review_contact` | `review_authority` |

The renderer always supplies `authority`, `title`, `law_name` and `request_term`.
It formats `legal_citation` first when a catalog citation exists, then supplies
that result (or the law name alone) to the remaining messages. `citation`, `date`,
`request_reference` and `review_authority` are available only when provided.
Do not make an ordinary request depend on an optional follow-up or appeal field.

The request body, requester name and review instructions are appended as literal
sections by [the typed renderer](../crates/fyi-core/src/i18n/templates.rs).
They are **not** Fluent variables: adding `{ $body }`, `{ $requester }` or
`{ $review_instructions }` to a resource will fail formatting. Changing section
order or introducing another typed input requires a separate renderer/API change.

Increment `locales/v1/version.json`'s content revision after a wording change,
update the affected hand-authored golden fixtures and their revision assertions,
and rebuild. Review the new expected wording rather than mechanically accepting
whatever the renderer produces. Run the [focused checks](#verification).

### Render a standalone custom message

`MessageBundle::from_ftl` accepts a caller-supplied UTF-8 Fluent string. This is
useful for a custom message with its own variables and content revision:

```rust
use fyi_core::i18n::{BundleVersion, MessageBundle};
use std::collections::BTreeMap;

let bundle = MessageBundle::from_ftl(
    "en-NZ",
    BundleVersion { format_version: 1, revision: "example-v1".into() },
    "notice = Draft for { $authority }",
)?;
bundle.require_messages(&["notice"])?;
let args = BTreeMap::from([("authority".into(), "Example Council".into())]);
assert_eq!(bundle.render("notice", &args)?, "Draft for Example Council");
# Ok::<(), fyi_core::i18n::I18nError>(())
```

This bundle does not replace `LocalizationEngine`'s embedded resources, register
a supported locale, compose a typed letter, or load a file. Callers own any I/O
and output-context escaping. Configurable external letter overrides remain open
under [#39](https://github.com/edithatogo/fyi-cli/issues/39).

## Explicit jurisdiction terminology

`i18n::terminology::TerminologyRegistry` supplies eleven typed concepts:
law name, law abbreviation, request, authority, deadline, department, ministry,
agency, follow-up, internal review and appeal. Select the legal regime explicitly;
a country code, instance identifier or display language is not a regime selector.
Unknown identifiers return an error instead of falling back to a different law.

```rust
use fyi_core::i18n::{LocalizationEngine, ValidationStatus};
use fyi_core::i18n::terminology::{Term, TerminologyRegistry};

let registry = TerminologyRegistry::embedded()?;
let german = LocalizationEngine::try_new("de-DE")?;
let term = registry.lookup("nz-oia", &german, Term::Request)?;
assert_eq!(term.text, "OIA-Anfrage");
assert_eq!(term.jurisdiction_id, "nz-oia");
assert_eq!(term.validation, ValidationStatus::RenderedUnvalidated);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The selected regime survives every locale change. Legal names and abbreviations
are returned literally; display labels use the existing strict Fluent renderer
and existing locale-resolution chain. English variants share the English glossary;
German, French and Spanish have their own resources. Results report the requested
and resolved locale, actual message source, catalog revision, message revision and
dated source reference. Literal law names have no message-source locale.

The embedded catalog supports the following explicit profiles. References were
checked on **2026-11-14** for law names, abbreviations and terminology context.
They do not establish applicability to a particular authority or approve our
translations.

| Profile | Terminology scope | Reference |
| --- | --- | --- |
| `nz-oia` | NZ central-government OIA; local-government LGOIMA remains separate | [NZ Ombudsman](https://www.ombudsman.parliament.nz/what-ombudsman-can-help/requests-official-information/your-ability-request-official-information) |
| `au-foi` | Australian Commonwealth FOI | [OAIC](https://www.oaic.gov.au/about-the-OAIC/access-our-information/freedom-of-information-requests-to-the-oaic) |
| `au-qld-rti` | Queensland RTI | [Queensland Government](https://www.rti.qld.gov.au/right-to-information-act) |
| `au-tas-rti` | Tasmanian RTI | [Ombudsman Tasmania](https://www.ombudsman.tas.gov.au/right-to-information/right-to-information-faqs) |
| `gb-foia` | UK FOIA 2000 / FOI; Scottish FOISA is separate | [ICO](https://ico.org.uk/for-organisations/foi/what-is-the-foi-act-and-are-we-covered/) |
| `de-ifg` | German federal IFG; does not select a state law | [Federal Ministry of Finance](https://www.bundesfinanzministerium.de/Web/DE/Service/Kontakt/Informationsfreiheitsgesetz/informationsfreiheitsgesetz.html) |
| `fr-crpa` | French CRPA access terminology; CADA is a review body | [CADA](https://www.cada.fr/administration/la-communication-des-documents-administratifs) |
| `es-ltaibg` | Spanish LTAIBG / Ley 19/2013 terminology | [Council for Transparency and Good Governance](https://consejodetransparencia.es/consejo-transparencia/que-es) |
| `us-foia` | US federal FOIA; state public-records laws are separate | [FOIA.gov](https://www.foia.gov/about.html) |

This is a bounded display glossary, not a comprehensive law catalog. No existing
instance catalog, letter citation, deadline or submission route is rewritten.
An agency/ministry/department noun is a display choice, not a legal classification.
Likewise, rendering a follow-up, internal-review or appeal label does not establish
that a particular remedy exists. All output remains plain text and
`rendered_unvalidated`; consumers apply the escaping required by their display.

### Extending the glossary

Profiles and four shared language resources live in
`crates/fyi-core/locales/terminology/v1/`, independently versioned from letter
resources. To add a regime, add an explicit identifier and sourced profile to
`catalog.json`, update its content revision, and add exact golden expectations.
No Rust registration table is needed for an additional profile.

The data-only `TerminologyRegistry::from_json` API accepts an entire caller-supplied
catalog in memory. It performs no filesystem or network access, does not merge
silently with embedded profiles and does not load Fluent overrides. Callers choose
which registry to use. This provides a profile extension point; arbitrary locale
registration and external message-resource loading remain separate requirements.

Catalog format version 1 rejects unknown fields, duplicate identifiers, empty
catalogs, malformed dates and oversized input (256 KiB maximum). Identifiers use
lowercase ASCII letters, digits and hyphens; country metadata uses two uppercase
ASCII letters. These syntax checks do not validate an ISO assignment or legal
coverage. Names must be nonempty, bounded single-line text without control or
Unicode line/paragraph separators. Source references must be HTTPS URLs without
credentials, query strings or fragments. Validation errors omit supplied values
and source text. Source URLs are recorded, never fetched by the registry.

Run the existing Rust integration harness:

```sh
cargo test --locked --offline -p fyi-core --test i18n_terminology
```

The offline command requires a pre-populated dependency cache. Review the six
hand-authored glossaries in `crates/fyi-core/tests/fixtures/i18n/terminology.json` with competent
language/jurisdiction reviewers before claiming translation or legal approval.
The automated matrix checks nine profiles × six locales × eleven concepts,
fallback provenance, malformed catalogs, literal Unicode and concurrent lookup.

## Deadline boundary

The i18n module now delegates arithmetic and catalog defaults to `deadlines.rs`.
It maintains no holiday calendar, statutory-rule catalog or competing day loop.
Locale selection cannot alter the existing deadline policy.

`try_add_working_days` and `try_deadline_for_instance` return errors for an
unrepresentable count or result. A small capacity guard uses complete weeks and at
most six weekday checks before calling the canonical calculator. It prevents
terminal-date saturation without duplicating jurisdiction-specific rules. Existing
infallible methods preserve ordinary results and panic on unsupported bounds;
new consumers should use the fallible methods. Zero days at either date boundary
remains valid. Tests compare all six locales with the canonical module, including
the existing default/negative period policy and exact capacity near `Date::MAX`.

### Explicit calendar-aware arithmetic

The separate `deadlines::statutory::calculate_explicit_deadline` API takes a
caller-selected `LegalClockRule` and matching `HolidayCalendar`. It supports
explicit working-day, calendar-day and calendar-month periods, receipt-day
conventions, extensions and terminal-date adjustment. See the
[deadline rule contract](deadline-rule-contract.md) for these semantics.

Loading a holiday catalog does not alter `try_add_working_days`,
`try_deadline_for_instance` or `deadlines::calculate_deadline_for_instance`.
Those compatibility helpers still use weekday-only estimates and never select a
calendar from a display locale or an instance identifier.

### Legal-rule verification still required

The instance catalog's `statutory_deadline_days` field and compatibility adapter
provide a weekday-only estimate, not evidence of statutory accuracy.
Before relying on a legal deadline, the deadline/jurisdiction workstream must verify
the time unit, triggering event, receipt-day treatment, national/regional/observed
holidays, special closure periods, extensions, transfers, consultation and suspension
rules, and review/appeal time limits against dated authoritative sources and reviewed
examples. The compatibility module's unchecked direct APIs and linear runtime for
very large valid periods remain a separate follow-up. The explicit statutory API
has its own range/period checks; neither API establishes legal applicability.

## Add a supported locale

The advertised locale list is currently closed. Adding another supported locale
is a source contribution and requires recompilation. `MessageBundle::from_ftl`
can parse another well-formed language identifier, but cannot register it with
`LocalizationEngine` or install a runtime override.

For example, a future `it-IT` contribution would require all of these steps;
Italian is **not currently supported**:

1. Choose the normalized identifier and define its intended fallback chain.
   Document the distinction between display language, terminology regime and
   calendar jurisdiction. Adding a locale does not add a law, platform or holiday
   calendar.
2. Create `crates/fyi-core/locales/v1/it-IT/common.ftl`. Use
   `en/common.ftl` as the message-key inventory, translating the typed-letter,
   legacy and deadline-label messages while preserving variable names. An English
   regional locale may inherit shared English keys; a new non-English locale
   needs translated messages rather than accidental English fallback.
3. In `crates/fyi-core/src/i18n.rs`, add the exact identifier to
   `SUPPORTED_LOCALES` and update its array length. Register the resource in
   `embedded_bundles()` with an explicit `include_str!` entry. A directory alone
   is not discovered automatically.
4. Update `resolve_locale()` if other identifiers in that language should use the
   new locale. Exact registration would enable `try_new("it-IT")`; it would not
   by itself change `new("it-CH")`, which currently falls back to English. Specify
   and test the intended compatibility fallback; strict `try_new` still rejects
   identifiers outside the advertised list.
5. For a new glossary language, add
   `crates/fyi-core/locales/terminology/v1/it-IT.ftl` and register it in
   `crates/fyi-core/src/i18n/terminology.rs`'s `bundles()`. Terminology follows the
   engine's locale chain and uses English for `base`. Adding letter resources
   alone would leave glossary terms falling back to English. Keep legal names
   and abbreviations literal in the separately sourced regime catalog.
6. Increment the letter content revision in `locales/v1/version.json`, relative
   to `crates/fyi-core/`. If glossary content changes, also increment
   `version.revision` in `locales/terminology/v1/catalog.json`. Extend the
   hand-authored `crates/fyi-core/tests/fixtures/i18n/messages.json`,
   `requests.json` and `terminology.json` fixtures, including message-source
   expectations. Update explicit locale inventories and revision assertions in
   `i18n_messages.rs`, `i18n_locale_goldens.rs`, `i18n_request_goldens.rs` and any
   affected tests.
7. Run the [verification commands](#verification). Check exact selection,
   fallback provenance, missing keys/variables, literal braces and Unicode,
   output escaping, and all four letter variants in both registers. Have
   competent language/jurisdiction reviewers assess the wording and record
   their findings independently of automated test success.

### Translation contribution checklist

Keep resources UTF-8 and retain translator comments explaining purpose, register
and variable meaning. Translate surrounding prose without translating message
identifiers or variable names. Use the supplied legal references; a translation
must not substitute a different country's law, invent a deadline or add review
rights. Preserve user-provided names and text without normalizing their Unicode.

Golden expectations should be authored and reviewed independently of the rendered
output. Use fictional authorities/requesters and public or synthetic examples.
Record the locale, exact content revision/commit, reviewed variants and glossary
concepts, reviewer role, review date, findings and unresolved questions in the PR
or track evidence. Rendering tests establish behavior; they do not establish
translation quality or legal approval. RTL support, arbitrary runtime locale
registration and external letter-resource overrides remain separate track work.

## Holiday calendars

`fyi_core::deadlines::statutory::HolidayCalendarCatalog` provides an offline,
versioned JSON catalog for explicit deadline arithmetic. The **2026** dataset now
contains 37 source-checked NZ/AU/UK calendars, plus the three unchanged, unreviewed
DE/FR/ES starters. Every source-checked date has a named event and authoritative
source reference in `crates/fyi-core/locales/calendars/v1/provenance.json`.

See the [holiday calendar guide](holiday-calendars.md) for the complete scope table,
source-review boundaries, local profiles, contribution process and commands.

- **Select the exact scope.** `NZ` and `AU` are national/common-date bases. They
  do not select an anniversary, state, local show day or statutory regime.
  `GB-ENG`, `GB-WLS`, `GB-SCT` and `GB-NIR` identify the four UK constituents.
  The old ambiguous embedded `GB` key has been removed; `get("GB")` returns `None`.
- **Check coverage and applicability.** Embedded dates cover 1 January through
  31 December 2026 only. Source checking establishes published holiday dates;
  every arithmetic result still has `ValidationStatus::Unvalidated`. The caller
  must supply an applicable, reviewed `LegalClockRule` and a complete calendar for
  that rule. Partial-day holidays and law-specific closures are not inferred.
- **Load contributions explicitly.** `HolidayCalendarCatalog::from_json` and
  `HolidayCalendar::from_json` accept caller-owned JSON without filesystem or
  network I/O. Existing valid v1 inputs and the public `HolidayCalendar` fields
  remain supported. Payloads are bounded to 256 KiB. Unknown fields, duplicate
  keys/dates/weekend codes, invalid scope identifiers and invalid coverage fail.
- **No fallback.** Display locale cannot select a legal calendar. An unknown local
  key returns `None`; a rule/calendar jurisdiction mismatch or a calculation
  beyond coverage returns an error.

Contribution keys begin with an uppercase two-letter country code and may have
hyphen-separated uppercase alphanumeric scope components of one to eight
characters, such as `AU-QLD-CAIRNS` or `NZ-CAN-SOUTH`. These local suffixes are
project identifiers, not automatic geocoding or a claim of ISO standardisation.
`weekend_days` uses `0 = Monday .. 6 = Sunday`. Each regional entry contains its
full date set; local replacements remove the superseded date before adding the
replacement. Merely unioning calendars can produce an incorrect result.

For a contributed embedded calendar, update the catalog, matching provenance and
regression tests, then run:

```bash
python scripts/verify_holiday_calendars.py
python -m pytest tests/test_holiday_calendar_catalog_schema.py tests/test_holiday_calendar_provenance.py
cargo test -p fyi-core --test holiday_calendar_data --test statutory_deadline_contract --locked
```

The offline evidence validator runs automatically through the existing Python
Tests job. It verifies schemas and cross-file consistency; it does not fetch a
source or confer legal approval. Community contributions must be checked against
current authoritative sources before their new revision is accepted.

## Remaining acceptance work

Additional terminology regimes, holiday calendars, statutory-rule validation,
additional locales, external overrides, translation review and legal review remain
open. Performance requires measured evidence; historical coverage measurements
are recorded in the track and do not certify a changed implementation.
The required workspace Rust coverage gate remains 90%; this work does not change it.
