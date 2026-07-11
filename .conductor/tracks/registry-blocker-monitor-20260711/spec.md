# Specification: External registry blocker monitor

Provide a read-only scheduled monitor for GitHub MCP Registry onboarding while
retaining explicit external-blocker semantics. The monitor must never publish,
modify registry data, or run in default CI.

## Acceptance criteria

- Poll Smithery, the official MCP Registry, and the GitHub curated surface.
- Emit stable JSON evidence and a fingerprinted Markdown summary.
- Comment on #32 only when evidence changes, avoiding weekly spam.
- Keep network access scheduled/manual only and require no secrets beyond the
  standard GitHub token.
- Test both blocked and listed states offline.
