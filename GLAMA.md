# Glama Release Notes

This repository includes a root `Dockerfile` for Glama source builds.

## Dockerfile Admin

Use the repository Dockerfile from the current `master` commit.

Recommended release command:

```text
/usr/local/bin/fyi-mcp
```

The container defaults to an ephemeral SQLite database:

```text
FYI_MCP_EPHEMERAL=1
DATABASE_URL=sqlite::memory:
```

That keeps Glama release tests, tool discovery, and Try in Browser usage
self-contained. Local users who need persistent storage can run the binary
directly and set `DATABASE_URL` to a writable SQLite URL.

## Tool Definition Quality (TDQS)

Glama scores each tool 1–5 on purpose, usage guidelines, behavioral transparency,
parameter semantics, conciseness, and completeness
([TDQS blog](https://glama.ai/blog/2026-04-03-tool-definition-quality-score-tdqs)).
Server-level **Tool Definition Quality** is `0.6 * mean + 0.4 * min`, so the
weakest tool pulls the average down.

When improving `tools/list` text in `crates/fyi-mcp/src/main.rs`
(`enrich_tool_definitions`):

- State **when to use / when not / sibling alternatives**
- Disclose **side effects** (local SQLite only, destructive, idempotent)
- Keep **2–4 sentences**, front-loaded
- Prefer schema enums/min/max/default for structure; put intent in free text

After shipping description changes: **Sync Server** in Glama admin and
optionally publish a new release so the score page refreshes.

Public score page: https://glama.ai/mcp/servers/edithatogo/fyi-cli/score

## Release Checklist

1. Sync the Glama repository admin page to the latest `master` commit.
2. Open the Dockerfile admin page.
3. Deploy the root Dockerfile.
4. After the build test succeeds, publish a Glama release using the current
   server version.
5. Confirm the public Glama API no longer reports `hosting:local-only` and
   includes the FYI MCP tools.
6. Use Try in Browser once, preferably `check_status`, to seed recent usage.
7. Re-check Tool Definition Quality on the score page after sync/release.
