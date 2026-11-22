# Bounded Wayback replay in Rust

`fyi-cli internet-archive-replay` accepts a self-digested selection of approved
CDX rows and retrieves only the listed raw replay targets. The operator must
review and authorize the selection: a matching digest detects changes to the
selection but does not establish who approved it. The command is read-only but
performs outbound requests to the fixed `https://web.archive.org` origin.

The required inputs are `--selection`, `--allowed-target-host`, `--output-dir`,
`--responses-dir`, `--checkpoint`, `--result` and `--receipt`. Optional limits
are `--max-rows`, `--max-payload-bytes`, `--max-redirects`,
`--max-runtime-seconds` and `--request-timeout-seconds`. Run
`fyi-cli internet-archive-replay --help` for their defaults. Inputs and evidence
paths must be distinct; the payload and response directories cannot contain one
another or the other paths. Result and receipt paths must be new, and the
response directory must be absent or empty at the start of each run.

Optional `--wacz <path> --archive-link <path>` outputs preserve each observed
response entity in a WARC inside the WACZ and bind its ordered records to the
receipt. Both paths must be fresh, distinct from every other input and output,
and outside the payload and response directories. The WACZ name must end in
`.wacz`. If these options are omitted, replay retains its existing behavior.

For an operator-reviewed selection, a bounded invocation is:

```bash
fyi-cli internet-archive-replay \
  --selection approved-selection.json \
  --allowed-target-host example.org \
  --output-dir replay-objects \
  --responses-dir replay-responses-001 \
  --checkpoint replay-checkpoint.json \
  --result replay-result-001.json \
  --receipt replay-receipt-001.json \
  --wacz replay-001.wacz \
  --archive-link replay-001-link.json \
  --max-rows 1 \
  --max-payload-bytes 1048576 \
  --max-redirects 2 \
  --max-runtime-seconds 30 \
  --request-timeout-seconds 10
```

Replace the target host with the exact hostname approved in the selection. This
command contacts Wayback; the committed synthetic fixtures and automated tests
are not authorization for a live capture.

The guarded transport never follows redirects automatically or retries. The
replay engine validates each redirect against the archive origin and approved
original URL before the next request, then checks the final status, media type,
length and payload digest against the selection. Its configured byte and time
bounds are enforced independently of the shared network guardrails. The replay
transport regressions use injected in-process responses and open no sockets;
they do not establish the behavior of a live HTTP server.

Verified decoded payloads are stored under `--output-dir`. The original encoded
HTTP entity body for each physical response, including redirect responses, is
stored under `--responses-dir` as `<zero-based-sequence>-<sha256>.raw`. The v1
receipt lists these responses in the same order and records their status, byte
length, digest, redacted URL, zero retries, the request bounds, and checkpoint
hashes. The separately saved result projection is bound by its own digest; it
is not a substitute for the HTTP bytes. These files do not constitute a complete
WARC response or signed provenance record.

To check saved evidence offline, run `fyi-cli verify-acquisition-receipt
<receipt> --result-file <result> --responses-dir <responses-dir>`. The ordered
response check rejects missing, extra, modified or symlinked response files.
The receipt records zero retries and the absence of a named shared limiter;
the adaptive middleware's individual waits and rate-limit headers are not yet
recorded as rate evidence.

For an archived run, verify the receipt and its WACZ together:

```bash
fyi-cli verify-acquisition-receipt replay-receipt-001.json \
  --result-file replay-result-001.json \
  --responses-dir replay-responses-001 \
  --wacz replay-001.wacz \
  --archive-link replay-001-link.json
```

The verifier checks WACZ fixity and each indexed WARC response's order, URL,
status, entity length and SHA-256 against the verified receipt. It also checks
the link's receipt digest, WACZ digest and byte count, and each WARC record ID
and compressed range. The link is self-digested and does not establish who
approved the selection or signed the evidence.

The link schema is `urn:fyi-cli:wayback-wacz-link:1`. It contains
`receipt_sha256`, `wacz_sha256`, `wacz_bytes`, and an ordered `records`
array with `response_index`, `record_id`, `offset` and `length`. Offsets
and lengths address compressed bytes in the WACZ's `archive/data.warc.gz`
member. `link_sha256` hashes the canonical JSON document with that field
omitted, using the same canonicalization as acquisition receipts.

The WARC records preserve the encoded entity bytes. Their HTTP/1.1 framing is
reconstructed, the source HTTP version is recorded separately, and response
headers follow the existing public allowlist. URL query strings are redacted as
in the receipt. The WACZ is not a byte-for-byte copy of the full HTTP exchange.

A checkpoint records verified payload progress and is revalidated against the
selection, configuration and stored payloads on resume. Use new result, receipt
and response paths for each resumed run. A completed checkpoint can therefore
produce a legitimate zero-request receipt whose before and after hashes match.
Because WACZ requires at least one response record, a completed checkpoint
cannot produce a new WACZ. Resume without the archive options for that case.

New checkpoint bytes are checked against the same 16 MiB limit used by resume
before the corresponding payload or checkpoint is published. If accumulated
metadata exceeds that bound, the previous checkpoint and its verified payloads
remain usable; the failed row does not become completed progress. Any raw
response evidence already captured remains available for inspection.

If a complete response is captured but fails verification, a failed receipt
records the observed bytes and a generic failure projection. Receipt v1 cannot
represent an attempted exchange that produced no complete response, including
a timeout or a stream stopped at its byte limit. In that case the command fails
without publishing a receipt or result; any response files and checkpoint
already written remain for inspection. A crash or a filesystem error can also
leave partial evidence, so consumers must require and verify the receipt and
result together instead of treating file presence as success.
For archive output, consumers must additionally require and verify the WACZ and
link. They are published as separate files, so an error can leave partial
artifacts. If acquisition completes but archive publication fails, the command
returns an error while preserving the acquisition result and receipt. Choose
fresh result, receipt, response, WACZ and link paths for a subsequent run.

This increment does not complete #310's all-network-command receipt coverage or
establish Python/Rust behavioral equivalence, hosted source smoke, linkage for
other source adapters and attachments, release pinning or downstream archive
shadow parity. The finite offline Python/Rust cases and their two explicit
behavioral differences are documented in the
[replay conformance corpus](../conformance/internet-archive-replay/README.md).
