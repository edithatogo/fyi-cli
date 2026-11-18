# Shared synthetic Alaveteli corpus, version 1

`manifest.json` binds 17 named cases to exact SHA-256 file identities, declared
response variants, payload schemas and expected outcomes. Existing fixtures in
the parent directory are referenced unchanged. The added variants cover bare
and wrapped arrays, Unicode, absent/null/additive fields, and truncated JSON,
RSS and Atom. All content is synthetic; names of response formats do not imply
certification by a live Alaveteli deployment.

Run the offline fixture gate:

```bash
python scripts/verify_mcp_fixture_corpus.py
python -m pytest -q tests/test_mcp_fixture_corpus.py -k 'not replay'
```

The verifier rejects unknown versions, missing/duplicate cases, changed bytes,
noncanonical or symlinked paths, oversized files, duplicate JSON keys, nonfinite
numbers, external schema references and unexpected syntax/schema outcomes. It
does not rewrite snapshots. Intentional incompatible changes require another
corpus version and a reviewed migration; historical fixtures remain available.

## Semantic evidence and boundaries

| Surface | Executable sensor | Scope |
| --- | --- | --- |
| Shared request/authority JSON and HTTP errors | `tests/test_mcp_fixture_corpus.py::test_shared_fixture_replay_uses_production_source_parser` | Twelve Linux-only cases replay these exact bytes through the real Rust MCP executable, official SDK and existing non-forwarding local TLS fixture; exact results, one GET, private errors and subsequent ping |
| Existing create/request JSON | `sync::tests::contract_fixtures_match_sync_parser_expectations` | Rust source parser, including missing required title |
| Existing RSS/Atom and HTTP error bodies | `sync::tests::feed_and_error_fixtures_match_sync_expectations` | Rust source adapter extracts expected IDs and hides error bodies |
| Malformed RSS/Atom | `scripts/verify_mcp_fixture_corpus.py` and its negative controls | Strict XML corpus oracle only; **production rejection is not established** |

The current production feed helper scans request-link text and does not strictly
parse XML. Both truncated feeds deliberately contain a complete request link or
ID, so they must not be presented as passing malformed-feed rejection tests for
that helper. Strict source-feed validation, read-only authority-feed/provenance
support and raw transfer/cache guarantees remain with issue #170's source owner.
The fixture corpus does not change those implementations.

The Linux source replay uses the existing SDK builder and
`FYI_MCP_TEST_BINARY` selection contract. It performs no live-network or write
operations. Other platforms explicitly skip the TLS replay while still running
the corpus and negative-control tests. Execution on one platform does not qualify
another platform, a live service or a later MCP revision.
