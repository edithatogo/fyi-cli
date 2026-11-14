# Localization and request-template foundation

## Scope and review status

`fyi_core::i18n` formats messages. It does not choose a jurisdiction, verify a legal
citation, establish a statutory deadline, persist a request or submit anything.
Successful rendering is **not** evidence of legal validity or human translation
approval. The embedded text is an unreviewed rendering baseline.

The existing `i18n-localization-framework` track remains active. Its historical
completed status was not supported by its original acceptance criteria. Shared
Conductor registry reconciliation belongs to the coordinating workstream.

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
the existing default/negative period policy and exact capacity near `NaiveDate::MAX`.

### Legal-rule verification still required

The catalog currently provides a number of days and its adapter uses weekday-only
counting. This is a compatibility estimate, not evidence of statutory accuracy.
Before relying on a legal deadline, the deadline/jurisdiction workstream must verify
the time unit, triggering event, receipt-day treatment, national/regional/observed
holidays, special closure periods, extensions, transfers, consultation and suspension
rules, and review/appeal time limits against dated authoritative sources and reviewed
examples. The canonical module's unchecked direct APIs and linear runtime for very
large valid periods remain a separate follow-up; only i18n entry points receive the
representation guard in this increment. Legal reliance remains blocked pending that
work and human review.

## Adding or changing resources

1. Edit the appropriate UTF-8 `.ftl` file and keep legal references as variables
   supplied by the existing jurisdiction catalog.
2. Increment `version.json`'s revision. Do not claim human review from a passing test.
3. Add exact golden expectations and negative cases for missing messages/variables,
   braces, Unicode and fallback. A new advertised locale also needs an explicit
   supported identifier, embedded resource registration and resolution policy.
4. Run formatting, Clippy, localization tests, drafting regressions and the full
   required PR checks. Human reviewers must assess terminology, register, law names
   and review routes separately.

The baseline deliberately has a closed locale list. Adding arbitrary locales
without code changes and configurable external overrides remain track requirements;
the low-level data-only bundle parser does not imply an override-loading facility.

## Remaining acceptance work

Terminology/glossary coverage, holiday calendars, statutory-rule validation,
additional locales, external overrides, translation review and legal review remain
open. Performance and the track's 95% i18n coverage target require measured evidence.
The required workspace Rust coverage gate remains 90%; this work does not change it.
