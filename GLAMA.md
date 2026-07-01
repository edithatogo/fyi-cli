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

## Release Checklist

1. Sync the Glama repository admin page to the latest `master` commit.
2. Open the Dockerfile admin page.
3. Deploy the root Dockerfile.
4. After the build test succeeds, publish a Glama release using the current
   server version.
5. Confirm the public Glama API no longer reports `hosting:local-only` and
   includes the FYI MCP tools.
6. Use Try in Browser once, preferably `check_status`, to seed recent usage.
