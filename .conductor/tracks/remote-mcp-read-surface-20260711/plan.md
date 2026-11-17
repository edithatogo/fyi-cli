# Plan: Remote MCP read surface

## Original-acceptance remainder — issue #170, 2026-11-17

The five read tools shipped in #176; the original checklist below did not record
their evidence and the registry incorrectly claimed completion. This increment
starts from `1f0ae6d0b00663d52ccb65904152ca4e933b3486` and implements the remaining
read contract using the existing SyncClient, MCP policy, telemetry, and offline
TLS harness. Existing source and write interfaces remain compatible.

- [~] Define and test versioned read errors, result provenance, and the two feed tools.
- [~] Retain one guarded source client per instance, propagate bounded read allowances,
  and implement source read/provenance plus feed discovery/pull planning without database writes.
- [~] Extend current MCP discovery/resources and offline compatibility fixtures while retaining historical fixtures.
- [ ] Run focused and workspace tests, lint, offline SDK contracts, and independent Conductor review.
- [ ] Record exact commit and hosted CI evidence, reconcile original acceptance, and update registry/metadata.

No live endpoint or authority submission is part of this increment. Feed discovery
returns an explicitly unverified canonical route; pull planning fetches and parses
one feed without fetching request details or mutating application data. Broader
Windows/macOS runtime qualification stays explicit until executed evidence exists.

## Phase 1: Contract and configuration

- [ ] Task: Define remote tool names, annotations, input/output schemas, and error envelope
- [ ] Task: Write failing discovery/configuration tests for default-disabled and read-enabled states
- [ ] Task: Wire validated policy and protected instance configuration into the MCP server
- [ ] Task: Conductor review verification 'Contract and configuration'

## Phase 2: Core read tools

- [ ] Task: Write failing WireMock tests for remote health, version, search, and request retrieval
- [ ] Task: Implement remote health, version, search, and request retrieval via SyncClient
- [ ] Task: Add bounded pagination, cache, timeout, and structured error behavior
- [ ] Task: Conductor review verification 'Core read tools'

## Phase 3: Authorities and feeds

- [ ] Task: Write failing tests for authority discovery, matching, and authority feeds
- [ ] Task: Implement remote authority discovery and authority-feed tools/resources
- [ ] Task: Add malformed feed, redaction, and guardrail regression coverage
- [ ] Task: Conductor review verification 'Authorities and feeds'

## Phase 4: Integration and documentation

- [ ] Task: Add offline MCP stdio end-to-end and smoke/system tests
- [ ] Task: Update MCP instructions, schemas, migration guide, threat model, and operator docs
- [ ] Task: Run full CI, coverage, security, compatibility, and Conductor review gates
- [ ] Task: Record GitHub issue #170 evidence and update metadata/registry
- [ ] Task: Conductor review verification 'Track closeout'
