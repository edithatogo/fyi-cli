# Specification: jurisdiction-abstraction-core

## Overview
This track introduces the intellectual core abstraction: a **Jurisdiction/Instance registry** + **provider trait** in `fyi-core`. This creates a "tzdata for FOI" - a parameterized, multi-jurisdictional Freedom-of-Information operations platform.

## Functional Requirements
1. **Instance Model:**
   - Create `Instance` model in `fyi-core/src/jurisdiction/`
   - Fields: `id` slug (e.g., `nz-fyi`, `au-rtk`), `base_url`, `country` (ISO-3166), `locale` (BCP-47)
   - FOI law metadata: law name + citation, request-type term (OIA/FOI/RTI/IFG), statutory deadline (working days), appeal body
   - Capabilities flags: read/write/attachments/batch/feeds
   - Status: supported/experimental/community
2. **Embedded Catalog:**
   - Create `instances.toml` compiled via `include_str!`
   - Seed from public Alaveteli deployment list
   - Support user-extensible overrides via `~/.config/fyi/instances.toml`
   - Version like a data package
3. **FoiProvider Trait:**
   - Define hexagonal port trait with methods: `get_request`, `search`, `create_request`, `add_correspondence`, `update_state`, `list_authorities`, `feeds`, `discover`, `prefilled_url`, `health`
   - Implement `AlaveteliV2Provider` as default impl parameterized by `Instance`
   - Handle per-instance quirks via `capabilities` flags (no forks)
   - Leave room for non-Alaveteli FOI systems (e.g., US FOIA portals)
4. **Database Partitioning:**
   - Add `instance_id` column to: `requests`, `correspondence`, `authorities`, `sync_metadata`, `sync_outgoing_queue`
   - Create migration that backfills existing rows as `nz-fyi`
   - Namespace remote IDs by instance to avoid collisions
5. **Per-Instance Credentials:**
   - Extend keyring to support per-instance API keys
   - Build on existing multi-account support
6. **Config & CLI/MCP Surface:**
   - Add global `--instance <id>` / `--country` CLI flag
   - Support `fyi instances list|show|add` commands
   - Add `instance` parameter to MCP tools
   - Make prefilled-URL builder instance-aware

## Non-Functional Requirements
- **Extensibility:** User-configurable instance registry without recompilation
- **Performance:** Instance lookup < 1ms
- **Backward Compatibility:** Existing single-instance usage (nz-fyi) works unchanged
- **Security:** Per-instance credential isolation in keyring

## Acceptance Criteria
- Instance model and embedded catalog implemented
- FoiProvider trait with AlaveteliV2Provider default impl
- Database migration adds `instance_id` to all tables, backfills `nz-fyi`
- CLI `instances` commands functional
- MCP tools accept `instance` parameter
- All existing tests pass with backfilled instance_id
- New integration tests verify multi-instance isolation

## Out of Scope
- Non-English localization (covered by i18n track)
- Specific jurisdiction onboarding (covered by jurisdiction tracks)
- UI/dashboard changes

## Dependencies
- Depends on: `fyi-api-coverage-audit` (track 1)

## Success Metrics
- **Abstraction Quality:** Zero instance-specific code paths in provider
- **Test Coverage:** 90%+ coverage on jurisdiction module
- **Migration Success:** All rows backfilled, zero data loss
