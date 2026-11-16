# Archive-format header and publication safety contract

This offline implementation was selectively imported from the frozen local `fyi-cli` source checkpoint `0ed3a87` under the existing Source workstream. It is not a network acquisition, archive-publication or Python-retirement switch.

## Header minimisation and evidence honesty

`WarcWriter` uses the **public-allowlist-v1** response-header policy by default and does not expose a raw-header preservation mode. Only Content-Type, Content-Encoding, Content-Language, Content-Range, ETag, Last-Modified, Date, Cache-Control, Expires, Vary and Accept-Ranges are retained, subject to validation and Connection-nominated header removal. All other incoming response header fields, including case-varied/multiple Set-Cookie, Set-Cookie2, WWW-Authenticate and unknown X-headers, are excluded. The reconstructed HTTP Content-Length is based on archived entity bytes, not source transfer framing.

WARC records explicitly include `WARC-FYI-Response-Header-Policy: public-allowlist-v1` so nobody can mistake the archived header section for the unmodified response. The WARC block digest covers **the resulting reconstructed and minimised HTTP header plus the entity payload**; the WARC payload digest covers the entity payload. No raw sensitive-header bytes, secret-derived hashes or secret values belong in public evidence or logs.

This policy **does not sanitise response bodies, URLs, query strings, redirections, CDXJ keys, page metadata or attachments**. It is not an automatic publication authorisation. Any need to retain byte-identical raw response headers requires a separately reviewed restricted storage workflow with access control, retention and an explicit no-public-export boundary; this library does not offer that workflow.

## Security, provenance and release gates

All fixtures contain fabricated data only. The negative contract ensures two differently cased Set-Cookie fields and authentication-like fields do not survive into the decompressed WARC HTTP response or verified WACZ; ordinary Content-Type and ETag remain and block/payload digests match the transformed bytes.

The standalone archive writer and verifier do not fetch, submit or publish anything. No live capture, externally accessible archive, release packet, downstream source switch or deletion is authorised by these tests. Source-owned capture-to-archive integration must separately demonstrate privacy policy enforcement, URL/body review, immutable publication, rollback and the pinned consumer shadow parity. Preserve `archive_cutover_allowed=false` and Python fallback.

## Licensing

The frozen source checkout declared `Apache-2.0` for its core. The current repository includes MIT-licensed code; to preserve both sets of notices, this mixed-source core declares `MIT AND Apache-2.0` with the complete Apache 2.0 licence under `LICENSES/Apache-2.0.txt`, retaining the root MIT licence unchanged. Any subsequent relicensing or third-party data-rights decision requires separate human review.

## Python development baseline

Any Python CI or development changes must use the user's pinned repo-local Python **3.15.1**, verify the selected virtual environment and preserve the existing protected checks. Python application packaging and its fallback are deliberately retained pending complete parity, release and consumer qualification.
