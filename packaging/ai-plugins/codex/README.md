# OpenAI plugin directory packet for Codex

This packet is prepared for OpenAI's documented plugin-directory / Codex review
flow. It is not evidence that a submission has been filed or accepted. The
public status remains `planned` until an operator with access to the submission
surface records public evidence in `packaging/registry-submissions.json`.

## Review notes

- `fyi-mcp` is local-first and defaults to stdio; the repo also documents an
  opt-in HTTP JSON-RPC transport for hosted deployments where a directory review
  requires it.
- Local database mutations are possible, but remote authority writes and remote
  FOI submissions are not exposed by this packet.
- Network access is opt-in and must remain bounded by the repository's provider
  and rate-limit policies.
- The release source is GitHub Releases; version, checksum, provenance, and
  rollback evidence must be checked before filing.

## Operator checklist

1. Review `submission.json` against the current release.
2. Run the repository's Rust, security, packaging, and MCP smoke checks.
3. Submit through the current OpenAI plugin-directory / Codex route using the
   maintainer account.
4. Record the resulting public URL, ticket, or rejection in the ledger and issue
   #197.
