# Receipt evidence for a single sync pull

The Rust CLI supports an acquisition receipt for one incremental API list
request using `fyi-cli sync pull --receipt <receipt> --raw-file <response>
--result-file <result>`. All three files are required and must be new, distinct
from one another and from the SQLite database and its sidecars. The database
must be specified as a filesystem path for this mode. The command rejects
symlinked paths, nested evidence paths and collisions before opening the
database or contacting the source.

The dedicated client makes one read-only GET, does not follow redirects, retry,
reuse a cache or decompress the HTTP body, and bounds that body to 16 MiB. It
requests identity encoding and rejects an encoded response it cannot parse. A
complete response is saved before database updates. Its actual status, redacted
URL, length and SHA-256 are recorded separately from the canonical result
projection; the stored incremental boundary is represented by a digest. Use
`fyi-cli verify-acquisition-receipt <receipt> --raw-file <response>
--result-file <result>` to check both files offline.

A complete HTTP error, redirect or parse/application failure produces a failed
receipt when the evidence can be published. An incomplete exchange cannot be
represented honestly by receipt v1 and produces no receipt. A filesystem error
or interruption can also leave partial evidence. Applying a list currently
updates database records individually, so an application or publication error
can leave partial database changes; the error result does not claim a count of
completed changes. Inspect the database and preserved evidence before resuming.

This increment covers only the incremental API list. Sync push and feed-based
receipt requests continue to fail before source or database I/O. Adaptive
pacing waits and response rate-limit headers are not yet fully captured in the
v1 evidence. Full network-command coverage, attachment/WARC linkage and
downstream shadow parity remain open under #310.
