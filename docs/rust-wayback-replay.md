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

A checkpoint records verified payload progress and is revalidated against the
selection, configuration and stored payloads on resume. Use new result, receipt
and response paths for each resumed run. A completed checkpoint can therefore
produce a legitimate zero-request receipt whose before and after hashes match.

If a complete response is captured but fails verification, a failed receipt
records the observed bytes and a generic failure projection. Receipt v1 cannot
represent an attempted exchange that produced no complete response, including
a timeout or a stream stopped at its byte limit. In that case the command fails
without publishing a receipt or result; any response files and checkpoint
already written remain for inspection. A crash or a filesystem error can also
leave partial evidence, so consumers must require and verify the receipt and
result together instead of treating file presence as success.

This increment does not complete #310's all-network-command receipt coverage or
establish Python/Rust behavioral equivalence, hosted source smoke, WARC linkage,
release pinning or downstream archive shadow parity.
