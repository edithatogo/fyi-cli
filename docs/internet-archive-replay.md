# Internet Archive replay

`internet-archive-replay` is the network-owning boundary for bounded retrieval
of raw Wayback payloads. It accepts only a versioned, self-digested approval
selection and writes verified payload objects, a resumable checkpoint, a
deterministic result projection, and an acquisition receipt.

The approval selection binds each row to the SHA-256 of its source CDX export
and preserves the CDX `digest`, `statuscode`, and `length` as provenance. CDX
`length` describes the archived record and is not treated as replay payload
length. Approval therefore supplies separate expected response status, payload
byte length, payload SHA-256, and optionally media type. A replay cannot create
or update its checkpoint unless all expected response properties match.

```powershell
fyi internet-archive-replay `
  --selection approved-selection.json `
  --allowed-target-host fyi.org.nz `
  --output-dir replay-objects `
  --result replay-result.json `
  --checkpoint replay-checkpoint.json `
  --receipt replay-receipt.json `
  --max-rows 100 `
  --max-payload-bytes 16777216 `
  --max-redirects 3 `
  --max-runtime-seconds 180
```

The archive request origin is fixed to `https://web.archive.org`. Redirects
must remain HTTPS on `web.archive.org`, use the raw `id_` replay form, and keep
the exact approved target URL. The target hostname must also exactly match
`--allowed-target-host`. Credentials, alternate ports, fragments, relative
redirects, target changes, and archive-host changes fail closed.

Responses are streamed under the configured byte cap. Payload files,
checkpoints, results, and receipts use temporary files plus atomic replacement;
symlink output boundaries are rejected. Resume verifies the checkpoint
self-digest, configuration binding, row order, deterministic object names, and
every persisted object's byte length and SHA-256 before making a request.

The committed fixture in `tests/fixtures/internet_archive_replay` is a
deterministic projection of the downstream `fyi-archive` Wayback replay
contract. Hosted live smoke, old/new downstream parity, pinned release
integration, NZ shadow parity, and archive cutover remain separate acceptance
gates.
