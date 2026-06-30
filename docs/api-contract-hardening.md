# API Contract Hardening

This checklist records the FYI/Alaveteli contract assumptions enforced by
`api-contract-hardening-20260630`.

## Supported Contract Shapes

- Request lists may be either a raw JSON array or an object with a `requests`
  array.
- `AlaveteliRequest.id`, `title`, and `body` are required for local sync.
- Optional request fields remain forward compatible; unknown fields are ignored
  by serde.
- Create-request responses require an FYI-issued `id` and public `url`.
- Public archive capture treats rendered FYI pages, attachments, discovery feeds,
  diff manifests, and archive health JSON as public-web contracts rather than
  privileged API contracts.

## Error Semantics

- Malformed JSON and missing required fields abort the pull before local rows are
  changed.
- HTTP 401, 403, 404, 429, and 5xx responses produce endpoint-specific error
  messages.
- Error messages include status and retry hints where available.
- Error messages do not include response bodies, API keys, tokens, credentials,
  request bodies, or sensitive raw payloads.
- Failed pushes leave local dirty data intact and move the queued submission to
  a recoverable failed state.

## Local Verification

Run the mocked contract checks before release:

```powershell
cargo +stable-x86_64-pc-windows-gnu test -p fyi-core sync::tests
.venv\Scripts\python.exe -m pytest tests/test_api_contract_inventory.py
.venv\Scripts\python.exe -m ruff check scripts/__init__.py scripts/api_contract_inventory.py tests/test_api_contract_inventory.py
```

Run full workspace checks before tagging a release:

```powershell
cargo +stable-x86_64-pc-windows-gnu fmt --all -- --check
cargo +stable-x86_64-pc-windows-gnu clippy --workspace --all-targets --all-features -- -D warnings
cargo +stable-x86_64-pc-windows-gnu test --workspace --all-features
```

The live FYI discovery smoke test remains opt-in:

```powershell
$env:FYI_LIVE_SMOKE = "1"
.venv\Scripts\python.exe -m pytest tests/test_discovery_smoke.py -m smoke
```
