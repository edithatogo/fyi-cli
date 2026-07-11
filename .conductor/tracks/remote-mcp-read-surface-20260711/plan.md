# Plan: Remote MCP read surface

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
