# Integration Track Specification

## Track: Integrate FYI Request System History

### Overview
This track handles the migration and integration of accumulated FYI request system work from 14 versioned zip archives into the current repository.

### Source Archives
The following zip files contain the historical work:
- `fyi-request-system.zip` (base version)
- `fyi-request-system-v2.zip` through `fyi-request-system-v14.zip`

### Integration Principles

1. **Canonical Source**: Version 14 (v14) is treated as the canonical source of truth
2. **Provenance Preservation**: All original zip files are preserved in `archive/upstream-zips/`
3. **Deduplication**: Only the latest version of each file is kept in the main repository
4. **Conductor Reconciliation**: The existing Conductor setup (from initial scaffolding) is reconciled with the `.conductor/` structure from v14

### Scope

#### In Scope
- Copy all 14 zip files to `archive/upstream-zips/`
- Extract v14 contents to repository root
- Preserve `.conductor/` structure from v14 (contains 14 phase tracks)
- Create integration track with spec and plan
- Update project documentation (README, handover)
- Create migration report documenting the integration

#### Out of Scope
- Re-extracting and merging files from v1-v13 (v14 is canonical)
- Modifying existing v14 implementation code
- Creating new feature tracks (these will be separate tracks)

### Deliverables

1. **Archive Structure**
   - `archive/upstream-zips/` containing all 14 original zip files

2. **Integrated Repository**
   - Repository root contains v14 project files
   - `.conductor/` directory with all 14 phase tracks
   - `conductor/` directory (from initial setup) for new Conductor methodology

3. **Documentation**
   - Migration report (`docs/migration-report.md` or `handover/migration-report.md`)
   - Updated README with project lineage
   - Integration track (this track) with spec and plan

4. **Conductor Context**
   - `.conductor/tracks.md` registry file
   - Integration track metadata, spec, and plan
   - Updated product context reflecting integration

### Success Criteria

- [ ] All 14 zip files copied to `archive/upstream-zips/`
- [ ] Repository root contains deduplicated v14 project files
- [ ] `.conductor/` structure preserved with all 14 phase tracks
- [ ] Integration track created with proper metadata
- [ ] Migration report documents the integration process
- [ ] Tests pass (or known issues documented)
- [ ] Repository is clean and ready for ongoing work

### Technical Notes

**Project Structure (from v14)**:
```
src/fyi_system/     # Main Python package
tests/              # Test suite
.conductor/         # Conductor tracks (14 phases)
skills/             # Gemini CLI skills
prompts/            # Prompt templates
handover/           # Handover documentation
data/               # Sample data files
scripts/            # Utility scripts
```

**Tech Stack (from v14)**:
- Python with pyproject.toml
- SQLite for local-first data storage
- FYI.org.nz API integration
- Local-first architecture

### Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Duplicate Conductor structures (`.conductor/` vs `conductor/`) | Confusion | Keep both; `.conductor/` for existing phases, `conductor/` for new methodology |
| Test failures after integration | Blocked work | Run tests, fix issues, document unresolved problems |
| Missing files from v14 | Incomplete integration | Compare with v13 if needed; document gaps |
| Conflicting documentation | Confusion | Update README and handover with clear lineage |

### Dependencies

- None (this is a foundational integration track)

### Future Work

After this integration track is complete:
1. Reconcile tech stack (Python vs Rust decision)
2. Create new tracks for Rust-based CLI/MCP server development
3. Decide on FYI.org.nz API client library structure (single repo vs split crates)
