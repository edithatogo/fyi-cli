# Anthropic Claude Connector submission packet

This packet is prepared for Anthropic's documented Connector Directory route.
**Current repo-side status: `assets-ready`.** That means the submission
materials are prepared in-repo, but this is not evidence that a submission has
been filed or accepted. Keep the target non-live until an operator with access
to the submission flow records external evidence in
`packaging/registry-submissions.json`.

## Review notes

- `fyi-mcp` is local-first and defaults to stdio; the repo also documents an
  opt-in HTTP JSON-RPC transport for hosted HTTPS deployments where Connector
  review requires it.
- Local database mutations are possible, but remote authority writes and remote
  FOI submissions are not exposed by this packet.
- Network access is opt-in and must remain bounded by the repository's provider
  and rate-limit policies.
- The release source is GitHub Releases; version, checksum, provenance, and
  rollback evidence must be checked before filing.

## Operator checklist

1. Review `submission.json` against the current release.
2. Run the repository's Rust, security, packaging, and MCP smoke checks.
3. Verify a hosted HTTPS deployment satisfies `deploy/remote-mcp/README.md`,
   then submit through the documented Anthropic route using the maintainer
   account.
4. Record the resulting public URL, ticket, or rejection in the ledger and issue
   #198.
