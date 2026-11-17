# Plan: Remote MCP read surface

## Original-acceptance remainder — issue #170, 2026-11-17

The five read tools shipped in #176; the original checklist below did not record
their evidence and the registry incorrectly claimed completion. This increment
starts from `1f0ae6d0b00663d52ccb65904152ca4e933b3486` and implements the remaining
read contract using the existing SyncClient, MCP policy, telemetry, and offline
TLS harness. Existing source and write interfaces remain compatible.

- [x] Define and test versioned read errors, result provenance, and the two feed tools. `0f80180`
- [x] Retain one guarded source client per instance, propagate bounded read allowances,
  and implement source read/provenance plus feed discovery/pull planning without database writes. `0f80180`
- [x] Extend current MCP discovery/resources and offline compatibility fixtures while retaining historical fixtures. `0f80180`
- [x] Run focused and workspace tests, lint, offline SDK contracts, and independent Conductor review.
  All 12 required checks and the Linux/macOS/Windows read-contract jobs passed at `0f80180`.
- [x] Record exact commit and hosted CI evidence, reconcile original acceptance, and update registry/metadata. `0f80180`
  PR [#494](https://github.com/edithatogo/fyi-cli/pull/494); see [acceptance evidence](acceptance-evidence.md).

No live endpoint or authority submission is part of this increment. Feed discovery
returns an explicitly unverified canonical route; pull planning fetches and parses
one feed without fetching request details or mutating application data. Broader
Windows/macOS runtime qualification stays explicit until executed evidence exists.


## Review and checkpoint status

Implementation is committed in PR #494 at `0f80180`. The source, MCP and fixture
owners independently reviewed their boundaries; the corrected source identity
and wire-accounting findings are recorded in `acceptance-evidence.md`.

The four original phases below are mapped to current code and passing automated
evidence. The original #176 execution history is not reconstructed. This
increment has observed fixture-contract RED/green evidence; local Rust runtime
RED checks were blocked by compiler SIGKILL, and hosted jobs supply the actual
production runtime evidence. No unobserved historical test pass is claimed.

The repository's `.conductor/workflow.md` requires explicit user feedback before
creating a final manual-verification phase checkpoint. Those review entries stay
open; no user confirmation, completed checkpoint or merged/archived track is
claimed. This does not change the repository's required CI merge checks.

## Phase 1: Contract and configuration

- [x] Task: Define remote tool names, annotations, input/output schemas, and error envelope — `0f80180`
- [x] Task: Write failing discovery/configuration tests for default-disabled and read-enabled states — `0f80180`
- [x] Task: Wire validated policy and protected instance configuration into the MCP server — `0f80180`
- [ ] Task: Conductor review verification 'Contract and configuration'

## Phase 2: Core read tools

- [x] Task: Write failing WireMock tests for remote health, version, search, and request retrieval — `0f80180`
- [x] Task: Implement remote health, version, search, and request retrieval via SyncClient — `0f80180`
- [x] Task: Add bounded pagination, cache, timeout, and structured error behavior — `0f80180`
- [ ] Task: Conductor review verification 'Core read tools'

## Phase 3: Authorities and feeds

- [x] Task: Write failing tests for authority discovery, matching, and authority feeds — `0f80180`
- [x] Task: Implement remote authority discovery and authority-feed tools/resources — `0f80180`
- [x] Task: Add malformed feed, redaction, and guardrail regression coverage — `0f80180`
- [ ] Task: Conductor review verification 'Authorities and feeds'

## Phase 4: Integration and documentation

- [x] Task: Add offline MCP stdio end-to-end and smoke/system tests — `0f80180`
- [x] Task: Update MCP instructions, schemas, migration guide, threat model, and operator docs — `0f80180`
- [x] Task: Run full CI, coverage, security, compatibility, and automated Conductor review gates — `0f80180`; manual phase checkpoints below remain open
- [x] Task: Record GitHub issue #170 evidence and update metadata/registry — `0f80180`, PR #494 and acceptance-evidence.md
- [ ] Task: Conductor review verification 'Track closeout'
