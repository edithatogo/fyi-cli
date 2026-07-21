# Incremental jurisdiction capture completion

Issue: [fyi-cli #234](https://github.com/edithatogo/fyi-cli/issues/234). Programme: [foi-o #81](https://github.com/edithatogo/foi-o/issues/81).

For every FOI-O roadmap target, verify the current platform and this tool's ability to enumerate and download public request, response, attachment and timeline material. Add a bounded jurisdiction/platform adapter where absent, with stable identifiers, pagination/completeness evidence, rate limits, privacy controls, positive and negative fixtures, and an archive handoff manifest.

Discovery and fixture work does not authorize live capture. Operator authorization, platform policy and rights are hard gates; capture state must never be treated as legal evidence.

## Acceptance

- Every target has a tested supported/unsupported/blocked status and evidence date.
- Supported paths produce a versioned handoff accepted by `fyi-archive`; unsupported paths retain actionable blockers.
- Adapters preserve jurisdiction identity and cannot silently fall back across profiles.
