# Plan: jurisdiction-abstraction-core

## Phase 1: Instance Model & Embedded Catalog

### 1.1 Instance Model
- [ ] Task: Create `src/jurisdiction/` module structure
- [ ] Task: Define `Instance` struct with all fields (id, base_url, country, locale, foi_law, capabilities, status)
- [ ] Task: Write failing tests for Instance deserialization
- [ ] Task: Implement Instance model with serde support
- [ ] Task: Conductor - User Manual Verification 'Phase 1.1: Instance Model' (Protocol in workflow.md)

### 1.2 Embedded Catalog
- [ ] Task: Create `instances.toml` with seed data from Alaveteli deployment list
- [ ] Task: Write failing tests for catalog loading via `include_str!`
- [ ] Task: Implement catalog loading and parsing
- [ ] Task: Add nz-fyi, au-rtk, uk-wdtk as initial entries
- [ ] Task: Conductor - User Manual Verification 'Phase 1.2: Catalog' (Protocol in workflow.md)

### 1.3 User-Extensible Overrides
- [ ] Task: Write failing tests for `~/.config/fyi/instances.toml` override
- [ ] Task: Implement override loading and merging logic
- [ ] Task: Test override precedence
- [ ] Task: Conductor - User Manual Verification 'Phase 1.3: Overrides' (Protocol in workflow.md)

## Phase 2: FoiProvider Trait & Default Implementation

### 2.1 Trait Definition
- [ ] Task: Define `FoiProvider` trait with all required methods
- [ ] Task: Write failing tests for trait contract
- [ ] Task: Document trait methods with examples
- [ ] Task: Conductor - User Manual Verification 'Phase 2.1: Trait' (Protocol in workflow.md)

### 2.2 AlaveteliV2Provider Implementation
- [ ] Task: Create `AlaveteliV2Provider` struct parameterized by Instance
- [ ] Task: Write failing tests for provider initialization
- [ ] Task: Implement all trait methods using Instance configuration
- [ ] Task: Handle capabilities flags for per-instance quirks
- [ ] Task: Conductor - User Manual Verification 'Phase 2.2: Provider' (Protocol in workflow.md)

## Phase 3: Database Migration & Partitioning

### 3.1 Schema Migration
- [ ] Task: Create migration adding `instance_id` column to all tables
- [ ] Task: Add indexes on `instance_id` columns
- [ ] Task: Write failing tests for migration execution
- [ ] Task: Conductor - User Manual Verification 'Phase 3.1: Migration' (Protocol in workflow.md)

### 3.2 Data Backfill
- [ ] Task: Implement backfill logic for existing rows → `nz-fyi`
- [ ] Task: Write tests verifying backfill correctness
- [ ] Task: Test with production-like data volumes
- [ ] Task: Conductor - User Manual Verification 'Phase 3.2: Backfill' (Protocol in workflow.md)

### 3.3 Remote ID Namespacing
- [ ] Task: Update remote ID handling to include instance prefix
- [ ] Task: Write tests for collision prevention
- [ ] Task: Test cross-instance data isolation
- [ ] Task: Conductor - User Manual Verification 'Phase 3.3: Namespacing' (Protocol in workflow.md)

## Phase 4: Per-Instance Credential Management

### 4.1 Keyring Extension
- [ ] Task: Write failing tests for per-instance keyring storage
- [ ] Task: Extend keyring wrapper to namespace by instance_id
- [ ] Task: Test credential isolation between instances
- [ ] Task: Conductor - User Manual Verification 'Phase 4.1: Keyring' (Protocol in workflow.md)

### 4.2 Credential Migration
- [ ] Task: Migrate existing credentials to instance-namespaced format
- [ ] Task: Write tests for credential migration
- [ ] Task: Ensure backward compatibility
- [ ] Task: Conductor - User Manual Verification 'Phase 4.2: Migration' (Protocol in workflow.md)

## Phase 5: Config & CLI/MCP Surface

### 5.1 CLI Instance Commands
- [ ] Task: Implement `fyi instances list` command
- [ ] Task: Implement `fyi instances show <id>` command
- [ ] Task: Implement `fyi instances add` command
- [ ] Task: Write integration tests for all commands
- [ ] Task: Conductor - User Manual Verification 'Phase 5.1: CLI Commands' (Protocol in workflow.md)

### 5.2 Global Instance Flag
- [ ] Task: Add `--instance <id>` global flag to CLI
- [ ] Task: Add `--country <iso>` convenience flag
- [ ] Task: Implement config file default instance setting
- [ ] Task: Write tests for flag precedence (flag > config > default)
- [ ] Task: Conductor - User Manual Verification 'Phase 5.2: Flags' (Protocol in workflow.md)

### 5.3 MCP Instance Parameter
- [ ] Task: Add `instance` parameter to all MCP tools
- [ ] Task: Update MCP tool schemas
- [ ] Task: Write integration tests for MCP instance selection
- [ ] Task: Conductor - User Manual Verification 'Phase 5.3: MCP' (Protocol in workflow.md)

## Phase 6: Integration Testing & Documentation

### 6.1 Multi-Instance Integration Tests
- [ ] Task: Create test suite with multiple instances
- [ ] Task: Test data isolation between instances
- [ ] Task: Test instance switching
- [ ] Task: Test concurrent access to different instances
- [ ] Task: Conductor - User Manual Verification 'Phase 6.1: Integration' (Protocol in workflow.md)

### 6.2 Documentation
- [ ] Task: Document jurisdiction abstraction design
- [ ] Task: Create instance configuration guide
- [ ] Task: Write migration guide from single to multi-instance
- [ ] Task: Document provider trait for future extensions
- [ ] Task: Conductor - User Manual Verification 'Phase 6.2: Documentation' (Protocol in workflow.md)

## Completion Criteria
- [ ] All phases complete
- [ ] FoiProvider trait and AlaveteliV2Provider implemented
- [ ] Database migration complete, all data backfilled
- [ ] CLI and MCP instance management functional
- [ ] All tests passing with 90%+ coverage
- [ ] Documentation complete

## Track History
- **2026-07-08**: Track created for multi-jurisdictional expansion
