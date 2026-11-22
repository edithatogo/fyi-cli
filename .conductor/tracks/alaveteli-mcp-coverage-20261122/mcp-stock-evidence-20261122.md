# Stock Alaveteli MCP handler evidence — 2026-11-22

Source checkpoint: `841ec7a1c032d87982e55e79e447f4c4988969cb`, based on
`708868328b695863d6fb1d108215e6dee057ab8a`. Reference controller revision:
`3966415f0b2a9f445fa3bb25c06eaeb75ded6609`. The synthetic fixture
`tests/fixtures/remote_mcp/upstream-stock-v1.json` has SHA-256
`5e3b0ff25e20fc3ecc037644e144825e746c4d97cc4e4ee9ac26f42564e1486c`.

The stock profile is opt-in per allowlisted instance and takes an independent,
operator-configured authority key. It does not enable reads or writes itself.
The original profile remains the default. Stock source requests use the bounded
no-redirect, no-retry transport; no live source or authority was contacted.

The test calls the actual Rust JSON-RPC handler and source adapter with a
feature-gated in-process transport, not a running Alaveteli instance. It checks
all five fixture cases: request read; request import; correspondence import;
correspondence import with exact attachment bytes; and state read/preflight and
update. It checks method, path, form/JSON fields, synthetic key, response and
exhaustion of expected exchanges. The four write cases also check confirmation,
commit, token rejection and cached replay. Five corresponding unchanged legacy
fixture cases run through the same handlers. Negative tests cover missing key,
stale state, private errors, 403, redirects, malformed success, reflected key,
unsupported writes, attachment count/MIME/size/symlink and ETag forwarding.

All listed tests and code checks ran on macOS with outbound and loopback network
access denied by `/usr/bin/sandbox-exec -p '(version 1) (allow default) (deny network*)'`.
Rust was `rustc 1.100.0` / `cargo 1.100.0`; Python was the explicitly selected
repository-local CPython 3.15.2. Cargo used `--locked --offline`, an isolated
target directory, and zero incremental compilation.

- `cargo test --locked --offline -p fyi-mcp --features sdk-regression-fixture stock_contract_tests -j 4`: 7 passed, including all ten stock/legacy fixture cases.
- `cargo test --locked --offline -p fyi-core --lib sync::alaveteli_stock::tests -j 4`: 1 passed; an opaque HTTP client cannot dispatch a stock write.
- `cargo test --locked --offline -p fyi-mcp --test wire_contract -j 4`: 7 passed against the real executable, including profile startup and missing-key denial.
- `cargo test --locked --offline -p fyi-mcp remote_contract_tests -j 4`: 19 passed.
- `cargo test --locked --offline -p fyi-mcp write_gate -j 4`: 30 passed, including the durable journal tests.
- `cargo clippy --locked --offline -p fyi-core -p fyi-mcp --all-targets --features sdk-regression-fixture -- -D warnings`: passed.
- `cargo fmt --all -- --check`, `scripts/verify_frontier_conductor.py`, and `scripts/verify_remote_mcp_contract.py`: passed. The latter used an isolated repository-local 3.15.2 environment with cached `jsonschema` dependencies.
- `pytest -q tests/test_api_contract_inventory.py tests/test_frontier_conductor.py`: 19 passed; `ruff check scripts/api_contract_inventory.py` passed.

The stock profile has not been exercised through the official MCP SDK or the
pinned upstream application. The selected local 3.15.2 environment lacked the
SDK; an offline installation attempt could not find `mcp==1.30.0` in the cache.
The legacy actual-binary SDK evidence remains in the existing harness history.
No hosted check, deployment, account session, requester journey, or downstream
archive parity is implied. Stock state preflight is not atomic: upstream can
change state between read and write. Attachments are still reopened after
prepare, so their immutability is unproven. The MCP read returns provenance,
not an acquisition receipt. Retain `archive_cutover_allowed=false`.
