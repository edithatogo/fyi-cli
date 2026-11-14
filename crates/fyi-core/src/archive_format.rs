//! Streaming WARC 1.1 responses and immutable WACZ 1.1.1 capture bundles.
//!
//! The input payload is the HTTP entity after transfer decoding. Entity headers
//! must describe those bytes, including Content-Encoding when still applicable.
//! Replay uses reconstructed HTTP/1.1 framing and retains the source HTTP version
//! separately. A response from HTTP/2 or HTTP/3 therefore has a logical HTTP/1.1
//! status line in its WARC block.
//!
//! WARC records have independent gzip members. WACZ stores that file verbatim,
//! with sorted CDXJ, pages JSONL, a resource manifest and SHA-256 fixity metadata.
//! Verification accepts this module's bounded five-member bundle profile and
//! streams response payloads without extracting archive paths.
//!
//! Standards and implementation references
//! - <https://iipc.github.io/warc-specifications/specifications/warc-format/warc-1.1/>
//! - <https://specs.webrecorder.net/wacz/1.1.1/>
//! - <https://docs.rs/zip/9.5.1/zip/>
//! - <https://docs.rs/flate2/1.1.11/flate2/>
//! - <https://docs.rs/surt-rs/0.1.3/surt_rs/>

use anyhow::{anyhow, bail, ensure, Context, Result};
use chrono::{DateTime, Datelike, SecondsFormat, Utc};
use data_encoding::BASE32;
use flate2::{bufread::GzDecoder, Compression, GzBuilder};
use reqwest::{header::HeaderName, header::HeaderValue, StatusCode, Url, Version};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::rc::Rc;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const MAX_HEADER_BYTES: usize = 128 * 1024;
const MAX_METADATA_BYTES: u64 = 16 * 1024 * 1024;
const WARC_PATH: &str = "archive/data.warc.gz";
const INDEX_PATH: &str = "indexes/index.cdxj";
const PAGES_PATH: &str = "pages/pages.jsonl";
const MANIFEST_PATH: &str = "datapackage.json";
const DIGEST_PATH: &str = "datapackage-digest.json";
const MEMBER_PATHS: [&str; 5] = [
    WARC_PATH,
    INDEX_PATH,
    PAGES_PATH,
    MANIFEST_PATH,
    DIGEST_PATH,
];
/// Only these response headers enter public WARC/WACZ exports. All other
/// source headers are omitted; the source exchange is *not* byte-preserved.
/// This is header minimisation, not a general public-release safety claim:
/// URLs, entity bodies and upstream consent require separate review.
const PUBLIC_RESPONSE_HEADERS: [&str; 11] = [
    "accept-ranges",
    "cache-control",
    "content-encoding",
    "content-language",
    "content-range",
    "content-type",
    "date",
    "etag",
    "expires",
    "last-modified",
    "vary",
];

const FRAMING_HEADERS: [&str; 10] = [
    "connection",
    "content-length",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "proxy-connection",
];

/// One acquired response whose logical headers describe its entity bytes.
///
/// `WarcWriter` always enforces the public-allowlist-v1 header policy.
/// It cannot preserve raw credentials, response cookies or unlisted headers.
/// Exact-header forensic storage, if authorised, belongs in a separate
/// restricted system; do not export restricted captures through this API.
/// The entity body and target URL are **not** sanitised by this policy.
pub struct HttpResource<'a> {
    pub kind: &'a str,
    pub url: &'a Url,
    pub status: u16,
    pub headers: &'a [(String, Vec<u8>)],
    pub payload: &'a [u8],
    pub captured_at: DateTime<Utc>,
    pub http_version: Version,
}

/// Exact payload identity and compressed replay location for a response record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArchivedRecord {
    pub kind: String,
    pub url: String,
    pub content_type: String,
    pub size: u64,
    pub sha256: String,
    pub warc_record_id: String,
    pub offset: u64,
    pub length: u64,
    pub status: u16,
    pub captured_at: DateTime<Utc>,
    pub http_version: String,
}

/// A replay entry point that must match a response in the capture.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArchivePage {
    pub url: String,
    pub ts: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Package identity returned after all format and fixity checks have passed.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageSummary {
    pub record_count: usize,
    pub bytes: u64,
    pub sha256: String,
}

struct CountingWriter<W> {
    inner: W,
    bytes: u64,
}

struct CheckpointState<F> {
    callback: F,
    failure: Option<String>,
}

type SharedCheckpoint<F> = Rc<RefCell<CheckpointState<F>>>;

fn run_checkpoint<F: FnMut() -> Result<()>>(checkpoint: &SharedCheckpoint<F>) -> Result<()> {
    let mut state = checkpoint.borrow_mut();
    if let Some(failure) = &state.failure {
        bail!("{failure}");
    }
    if let Err(error) = (state.callback)() {
        state.failure = Some(format!("{error:#}"));
        return Err(error);
    }
    Ok(())
}

struct CheckpointFile<F> {
    file: File,
    checkpoint: SharedCheckpoint<F>,
}

impl<F: FnMut() -> Result<()>> CheckpointFile<F> {
    fn check(&self) -> io::Result<()> {
        run_checkpoint(&self.checkpoint).map_err(|error| io::Error::other(error.to_string()))
    }

    fn sync_all(&self) -> io::Result<()> {
        self.check()?;
        self.file.sync_all()?;
        self.check()
    }
}

impl<F: FnMut() -> Result<()>> Read for CheckpointFile<F> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.check()?;
        self.file.read(bytes)
    }
}

impl<F: FnMut() -> Result<()>> Write for CheckpointFile<F> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.check()?;
        self.file.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.check()?;
        self.file.flush()
    }
}

impl<F: FnMut() -> Result<()>> Seek for CheckpointFile<F> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.check()?;
        self.file.seek(position)
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = self.inner.write(bytes)?;
        self.bytes = self
            .bytes
            .checked_add(count as u64)
            .ok_or_else(|| io::Error::other("archive byte count overflow"))?;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Writes independently compressed response records to a fresh output stream.
///
/// Offsets start at zero. A source or header validation error leaves the stream
/// usable. An I/O failure poisons the writer and is also reported by `finish`.
pub struct WarcWriter<W: Write> {
    output: CountingWriter<W>,
    failed: bool,
}

impl<W: Write> WarcWriter<W> {
    pub fn new(output: W) -> Self {
        Self {
            output: CountingWriter {
                inner: output,
                bytes: 0,
            },
            failed: false,
        }
    }

    pub fn append(&mut self, resource: &HttpResource<'_>) -> Result<ArchivedRecord> {
        ensure!(!self.failed, "WARC writer has an earlier I/O failure");
        validate_url(resource.url)?;
        validate_kind(resource.kind)?;
        validate_timestamp(resource.captured_at)?;
        ensure!(
            (100..=599).contains(&resource.status),
            "invalid HTTP response status"
        );
        ensure!(
            !(resource.status < 200 || resource.status == 204 || resource.status == 304)
                || resource.payload.is_empty(),
            "bodyless HTTP response status has an entity payload"
        );
        let source_version = source_version(resource.http_version)?;
        let http_header = response_header(resource)?;
        let content_type = content_type(resource.headers)?;
        let payload_digest = Sha256::digest(resource.payload);
        let mut block_digest = Sha256::new();
        block_digest.update(&http_header);
        block_digest.update(resource.payload);
        let record_id = record_id()?;
        let block_length = (http_header.len() as u64)
            .checked_add(resource.payload.len() as u64)
            .context("WARC block length overflow")?;
        let header = format!(
            "WARC/1.1\r\nWARC-Type: response\r\nWARC-Record-ID: {record_id}\r\nWARC-Date: {}\r\nWARC-Target-URI: {}\r\nContent-Type: application/http;msgtype=response\r\nContent-Length: {block_length}\r\nWARC-Block-Digest: sha256:{}\r\nWARC-Payload-Digest: sha256:{}\r\nWARC-FYI-Kind: {}\r\nWARC-FYI-Source-HTTP-Version: {source_version}\r\nWARC-FYI-Response-Header-Policy: public-allowlist-v1\r\n\r\n",
            rfc3339(resource.captured_at),
            resource.url,
            BASE32.encode(&block_digest.finalize()),
            BASE32.encode(&payload_digest),
            resource.kind,
        );
        ensure!(
            header.len() <= MAX_HEADER_BYTES,
            "WARC header exceeds the format limit"
        );
        let offset = self.output.bytes;
        let result = (|| -> io::Result<()> {
            let mut gzip = GzBuilder::new()
                .mtime(0)
                .write(&mut self.output, Compression::default());
            gzip.write_all(header.as_bytes())?;
            gzip.write_all(&http_header)?;
            gzip.write_all(resource.payload)?;
            gzip.write_all(b"\r\n\r\n")?;
            gzip.finish()?;
            Ok(())
        })();
        if let Err(error) = result {
            self.failed = true;
            return Err(error).context("writing compressed WARC response");
        }
        Ok(ArchivedRecord {
            kind: resource.kind.to_string(),
            url: resource.url.to_string(),
            content_type,
            size: resource.payload.len() as u64,
            sha256: hex(&payload_digest),
            warc_record_id: record_id,
            offset,
            length: self.output.bytes - offset,
            status: resource.status,
            captured_at: resource.captured_at,
            http_version: source_version.to_string(),
        })
    }

    pub fn finish(mut self) -> Result<W> {
        ensure!(!self.failed, "WARC writer has an earlier I/O failure");
        self.output.flush().context("flushing WARC output")?;
        Ok(self.output.inner)
    }
}

fn validate_url(url: &Url) -> Result<()> {
    ensure!(
        matches!(url.scheme(), "http" | "https") && url.host_str().is_some(),
        "archive URL must be absolute HTTP(S)"
    );
    ensure!(
        url.username().is_empty() && url.password().is_none(),
        "archive URL contains credentials"
    );
    ensure!(url.fragment().is_none(), "archive URL contains a fragment");
    ensure!(
        url.as_str().bytes().all(|byte| byte.is_ascii_graphic()),
        "archive URL contains unsafe characters"
    );
    Ok(())
}

fn parse_url(value: &str) -> Result<Url> {
    let url = Url::parse(value).context("invalid archive URL")?;
    validate_url(&url)?;
    ensure!(
        url.as_str() == value,
        "archive URL is not in canonical absolute form"
    );
    Ok(url)
}

fn validate_kind(kind: &str) -> Result<()> {
    ensure!(
        !kind.is_empty() && kind.len() <= 256 && kind.bytes().all(|byte| byte.is_ascii_graphic()),
        "invalid archive resource kind"
    );
    Ok(())
}

fn validate_timestamp(timestamp: DateTime<Utc>) -> Result<()> {
    ensure!(
        (1..=9999).contains(&timestamp.year()),
        "archive timestamp is outside the CDX calendar range"
    );
    Ok(())
}

fn rfc3339(timestamp: DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

fn source_version(version: Version) -> Result<&'static str> {
    match version {
        Version::HTTP_10 => Ok("HTTP/1.0"),
        Version::HTTP_11 => Ok("HTTP/1.1"),
        Version::HTTP_2 => Ok("HTTP/2"),
        Version::HTTP_3 => Ok("HTTP/3"),
        _ => bail!("unsupported source HTTP version"),
    }
}

fn validate_headers(headers: &[(String, Vec<u8>)]) -> Result<()> {
    let mut length = 0usize;
    for (name, value) in headers {
        HeaderName::from_bytes(name.as_bytes()).context("invalid HTTP header name")?;
        HeaderValue::from_bytes(value).context("invalid HTTP header value")?;
        length = length
            .checked_add(name.len() + value.len() + 4)
            .context("HTTP header size overflow")?;
        ensure!(
            length <= MAX_HEADER_BYTES - 128,
            "HTTP headers exceed the format limit"
        );
    }
    Ok(())
}

fn content_type(headers: &[(String, Vec<u8>)]) -> Result<String> {
    let mut values = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("content-type"));
    let content_type = values
        .next()
        .map(|(_, value)| std::str::from_utf8(value))
        .transpose()
        .context("invalid Content-Type encoding")?
        .unwrap_or("application/octet-stream")
        .trim();
    ensure!(values.next().is_none(), "multiple Content-Type headers");
    ensure!(
        !content_type.is_empty()
            && content_type
                .bytes()
                .all(|byte| byte.is_ascii_graphic() || byte == b' ' || byte == b'\t'),
        "invalid Content-Type value"
    );
    Ok(content_type.to_string())
}

fn response_header(resource: &HttpResource<'_>) -> Result<Vec<u8>> {
    validate_headers(resource.headers)?;
    let mut excluded: BTreeSet<String> = FRAMING_HEADERS
        .iter()
        .map(|name| name.to_string())
        .collect();
    for (_, value) in resource
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("connection"))
    {
        for token in std::str::from_utf8(value)
            .context("invalid Connection header")?
            .split(',')
        {
            let token = token.trim();
            HeaderName::from_bytes(token.as_bytes()).context("invalid Connection header token")?;
            excluded.insert(token.to_ascii_lowercase());
        }
    }
    let status = StatusCode::from_u16(resource.status).context("invalid HTTP response status")?;
    let mut bytes = format!(
        "HTTP/1.1 {} {}\r\n",
        resource.status,
        status.canonical_reason().unwrap_or("")
    )
    .into_bytes();
    for (name, value) in resource.headers {
        let normalized = name.to_ascii_lowercase();
        if !excluded.contains(&normalized) && PUBLIC_RESPONSE_HEADERS.contains(&normalized.as_str())
        {
            bytes.extend_from_slice(name.as_bytes());
            bytes.extend_from_slice(b": ");
            bytes.extend_from_slice(value);
            bytes.extend_from_slice(b"\r\n");
        }
    }
    write!(
        &mut bytes,
        "Content-Length: {}\r\n\r\n",
        resource.payload.len()
    )?;
    ensure!(
        bytes.len() <= MAX_HEADER_BYTES,
        "HTTP response header exceeds the format limit"
    );
    Ok(bytes)
}

fn record_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|_| anyhow!("secure WARC record identifier generation failed"))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let value = hex(&bytes);
    Ok(format!(
        "<urn:uuid:{}-{}-{}-{}-{}>",
        &value[..8],
        &value[8..12],
        &value[12..16],
        &value[16..20],
        &value[20..]
    ))
}

fn hex(bytes: &[u8]) -> String {
    data_encoding::HEXLOWER.encode(bytes)
}

#[derive(Deserialize, Serialize)]
struct ResourceDescriptor {
    name: String,
    path: String,
    bytes: u64,
    hash: String,
}

#[derive(Deserialize, Serialize)]
struct Manifest {
    profile: String,
    wacz_version: String,
    resources: Vec<ResourceDescriptor>,
    title: String,
    description: String,
    created: DateTime<Utc>,
    software: String,
    #[serde(rename = "mainPageUrl", skip_serializing_if = "Option::is_none")]
    main_page_url: Option<String>,
    #[serde(rename = "mainPageDate", skip_serializing_if = "Option::is_none")]
    main_page_date: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct ManifestDigest {
    path: String,
    hash: String,
}

#[derive(Deserialize, Serialize)]
struct CdxEntry {
    url: String,
    mime: String,
    status: String,
    digest: String,
    length: String,
    offset: String,
    filename: String,
    warc_record_id: String,
}

struct IndexedRecord {
    key: String,
    timestamp: String,
    entry: CdxEntry,
}

/// Creates a new WACZ file and verifies its complete contents before returning.
///
/// The caller owns the source WARC. Existing outputs are preserved. An error
/// removes only the incomplete file created by this call. Source records must
/// cover every compressed byte with contiguous, independently readable members.
pub fn write_wacz(
    output: &Path,
    warc_path: &Path,
    records: &[ArchivedRecord],
    pages: &[ArchivePage],
) -> Result<PackageSummary> {
    write_wacz_with_checkpoint(output, warc_path, records, pages, || Ok(()))
}

/// Streams a package with fallible checkpoints at file I/O and phase boundaries.
pub fn write_wacz_with_checkpoint<F: FnMut() -> Result<()>>(
    output: &Path,
    warc_path: &Path,
    records: &[ArchivedRecord],
    pages: &[ArchivePage],
    checkpoint: F,
) -> Result<PackageSummary> {
    let checkpoint = Rc::new(RefCell::new(CheckpointState {
        callback: checkpoint,
        failure: None,
    }));
    run_checkpoint(&checkpoint)?;
    ensure!(
        output
            .extension()
            .is_some_and(|extension| extension == "wacz"),
        "WACZ output path must have a .wacz extension"
    );
    ensure!(!records.is_empty(), "WACZ requires response records");
    let source = File::open(warc_path).context("opening source WARC")?;
    ensure!(
        source.metadata()?.is_file(),
        "source WARC must be a regular file"
    );
    let mut source = CheckpointFile {
        file: source,
        checkpoint: checkpoint.clone(),
    };
    validate_source_records(&mut source, records)?;
    run_checkpoint(&checkpoint)?;
    validate_pages(pages, records)?;
    source.seek(SeekFrom::Start(0))?;
    let (warc_length, warc_hash) = hash_reader(&mut source)?;
    source.seek(SeekFrom::Start(0))?;
    let index = index_bytes(records)?;
    let page_bytes = pages_bytes(pages)?;
    let manifest = Manifest {
        profile: "data-package".into(),
        wacz_version: "1.1.1".into(),
        resources: vec![
            ResourceDescriptor {
                name: "data.warc.gz".into(),
                path: WARC_PATH.into(),
                bytes: warc_length,
                hash: format!("sha256:{warc_hash}"),
            },
            descriptor(INDEX_PATH, &index),
            descriptor(PAGES_PATH, &page_bytes),
        ],
        title: pages
            .first()
            .and_then(|page| page.title.clone())
            .unwrap_or_else(|| "FYI archive".into()),
        description: "Archived HTTP entity responses with reconstructed HTTP/1.1 framing.".into(),
        created: Utc::now(),
        software: format!("fyi-core/{}", env!("CARGO_PKG_VERSION")),
        main_page_url: pages.first().map(|page| page.url.clone()),
        main_page_date: pages.first().map(|page| rfc3339(page.ts)),
    };
    let manifest_bytes = metadata_json(&manifest)?;
    let digest = metadata_json(&ManifestDigest {
        path: MANIFEST_PATH.into(),
        hash: format!("sha256:{}", hex(&Sha256::digest(&manifest_bytes))),
    })?;
    run_checkpoint(&checkpoint)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(output)
        .context("creating immutable WACZ output")?;
    let file = CheckpointFile {
        file,
        checkpoint: checkpoint.clone(),
    };
    let result = (|| -> Result<PackageSummary> {
        let mut archive = ZipWriter::new(file);
        archive.start_file(
            WARC_PATH,
            stored_options().large_file(warc_length >= u32::MAX as u64),
        )?;
        let (copied, copied_hash) = copy_and_hash(&mut source, &mut archive)?;
        ensure!(
            copied == warc_length && copied_hash == warc_hash,
            "source WARC changed during packaging"
        );
        for (name, bytes) in [
            (INDEX_PATH, index.as_slice()),
            (PAGES_PATH, page_bytes.as_slice()),
            (MANIFEST_PATH, manifest_bytes.as_slice()),
            (DIGEST_PATH, digest.as_slice()),
        ] {
            archive.start_file(name, stored_options())?;
            archive.write_all(bytes)?;
        }
        archive
            .finish()?
            .sync_all()
            .context("persisting WACZ output")?;
        verify_wacz_inner(output, checkpoint.clone())
    })();
    match result {
        Ok(summary) => Ok(summary),
        Err(error) => {
            fs::remove_file(output).context("removing incomplete WACZ output")?;
            Err(error)
        }
    }
}

fn stored_options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o600)
}

fn descriptor(path: &str, bytes: &[u8]) -> ResourceDescriptor {
    ResourceDescriptor {
        name: path.rsplit('/').next().unwrap_or(path).to_string(),
        path: path.to_string(),
        bytes: bytes.len() as u64,
        hash: format!("sha256:{}", hex(&Sha256::digest(bytes))),
    }
}

fn validate_pages(pages: &[ArchivePage], records: &[ArchivedRecord]) -> Result<()> {
    let mut ids = BTreeSet::new();
    for page in pages {
        parse_url(&page.url)?;
        validate_timestamp(page.ts)?;
        ensure!(
            records
                .iter()
                .any(|record| record.url == page.url && record.captured_at == page.ts),
            "page entry does not match a captured response"
        );
        if let Some(id) = &page.id {
            ensure!(
                !id.is_empty() && ids.insert(id),
                "page identifiers must be nonempty and unique"
            );
        }
    }
    Ok(())
}

fn index_bytes(records: &[ArchivedRecord]) -> Result<Vec<u8>> {
    let mut lines = Vec::with_capacity(records.len());
    for record in records {
        let entry = CdxEntry {
            url: record.url.clone(),
            mime: mime_type(&record.content_type),
            status: record.status.to_string(),
            digest: format!("sha256:{}", record.sha256),
            length: record.length.to_string(),
            offset: record.offset.to_string(),
            filename: "data.warc.gz".into(),
            warc_record_id: record.warc_record_id.clone(),
        };
        let mut line = BoundedMetadata::default();
        write!(
            &mut line,
            "{} {} ",
            surt_key(&record.url)?,
            record.captured_at.format("%Y%m%d%H%M%S")
        )?;
        serde_json::to_writer(&mut line, &entry)?;
        line.write_all(b"\n")?;
        lines.push(line.bytes);
    }
    lines.sort();
    let mut output = BoundedMetadata::default();
    for line in lines {
        output.write_all(&line)?;
    }
    Ok(output.bytes)
}

fn surt_key(value: &str) -> Result<String> {
    let mut url = parse_url(value)?;
    // The SURT crate sorts query names before folding case. Fold first so its
    // sort agrees with the reference canonicalizer for mixed-case query names.
    let query = url.query().map(str::to_lowercase);
    url.set_query(query.as_deref());
    let mut key = surt_rs::generate_surt(url.as_str()).context("canonicalizing CDXJ URL")?;
    // The reference SURT represents an IPv6 host without URI brackets.
    if let Some(ipv6) = key.strip_prefix('[') {
        let end = ipv6.find(']').context("invalid IPv6 CDXJ host")?;
        key = format!("{}{}", &ipv6[..end], &ipv6[end + 1..]);
    }
    // Percent decoding can introduce query separators. Sort the resulting
    // tokens as well so replay lookup agrees with the reference canonicalizer.
    if let Some((path, query)) = key.split_once('?') {
        let mut tokens: Vec<&str> = query.split('&').collect();
        tokens.sort();
        key = format!("{path}?{}", tokens.join("&"));
    }
    // Decoding in the SURT library can reintroduce whitespace and UTF-8 bytes.
    // CDXJ's first token must remain a single safe ASCII field.
    let mut escaped = String::with_capacity(key.len());
    for byte in key.bytes() {
        if byte.is_ascii_graphic() {
            escaped.push(byte as char);
        } else {
            use std::fmt::Write as _;
            write!(&mut escaped, "%{byte:02x}")?;
        }
    }
    ensure!(!escaped.is_empty(), "empty CDXJ key");
    Ok(escaped)
}

fn mime_type(value: &str) -> String {
    value
        .split(';')
        .next()
        .unwrap_or(value)
        .trim()
        .to_ascii_lowercase()
}

fn pages_bytes(pages: &[ArchivePage]) -> Result<Vec<u8>> {
    let mut output = BoundedMetadata::default();
    output
        .write_all(b"{\"format\":\"json-pages-1.0\",\"id\":\"pages\",\"title\":\"All Pages\"}\n")?;
    for page in pages {
        serde_json::to_writer(&mut output, page)?;
        output.write_all(b"\n")?;
    }
    Ok(output.bytes)
}

#[derive(Default)]
struct BoundedMetadata {
    bytes: Vec<u8>,
}

impl Write for BoundedMetadata {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > MAX_METADATA_BYTES.saturating_sub(self.bytes.len() as u64) {
            return Err(io::Error::other("archive metadata exceeds 16 MiB"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn metadata_json(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut output = BoundedMetadata::default();
    serde_json::to_writer(&mut output, value)?;
    Ok(output.bytes)
}

fn hash_reader(input: &mut impl Read) -> Result<(u64, String)> {
    copy_and_hash(input, &mut io::sink())
}

fn copy_and_hash(input: &mut impl Read, output: &mut impl Write) -> Result<(u64, String)> {
    let mut digest = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let length = input.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        output.write_all(&buffer[..length])?;
        digest.update(&buffer[..length]);
        bytes = bytes
            .checked_add(length as u64)
            .context("archive length overflow")?;
    }
    Ok((bytes, hex(&digest.finalize())))
}

/// Verifies this module's five-member WACZ profile, fixity and replay linkage.
///
/// JSON, JSONL and CDXJ members are limited to 16 MiB each. WARC bytes are hashed
/// and decompressed as a stream. Every index range must contain exactly one
/// complete response record, and ranges must cover the entire stored WARC.
pub fn verify_wacz(path: &Path) -> Result<PackageSummary> {
    verify_wacz_with_checkpoint(path, || Ok(()))
}

/// Verifies a package with fallible checkpoints throughout streamed file I/O.
pub fn verify_wacz_with_checkpoint<F: FnMut() -> Result<()>>(
    path: &Path,
    checkpoint: F,
) -> Result<PackageSummary> {
    verify_wacz_inner(
        path,
        Rc::new(RefCell::new(CheckpointState {
            callback: checkpoint,
            failure: None,
        })),
    )
}

fn verify_wacz_inner<F: FnMut() -> Result<()>>(
    path: &Path,
    checkpoint: SharedCheckpoint<F>,
) -> Result<PackageSummary> {
    run_checkpoint(&checkpoint)?;
    let file = File::open(path).context("opening WACZ input")?;
    let mut file = CheckpointFile {
        file,
        checkpoint: checkpoint.clone(),
    };
    preflight_zip_directory(&mut file)?;
    let mut archive = ZipArchive::new(file).context("reading WACZ ZIP directory")?;
    ensure!(!archive.has_duplicate_names(), "duplicate ZIP member names");
    ensure!(
        archive.len() == MEMBER_PATHS.len(),
        "unexpected WACZ member count"
    );
    let mut names = BTreeSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = std::str::from_utf8(entry.name_raw()).context("non-UTF-8 WACZ member name")?;
        ensure!(
            MEMBER_PATHS.contains(&name) && names.insert(name.to_string()),
            "unsafe or unexpected WACZ member path"
        );
        ensure!(
            !entry.is_dir() && !entry.is_symlink() && !entry.encrypted(),
            "unsupported WACZ member type"
        );
        ensure!(
            entry.compression() == CompressionMethod::Stored,
            "WACZ profile requires stored ZIP members"
        );
    }
    let manifest_bytes = small_member(&mut archive, MANIFEST_PATH)?;
    let digest: ManifestDigest = serde_json::from_slice(&small_member(&mut archive, DIGEST_PATH)?)
        .context("invalid WACZ manifest digest")?;
    ensure!(
        digest.path == MANIFEST_PATH
            && digest.hash == format!("sha256:{}", hex(&Sha256::digest(&manifest_bytes))),
        "WACZ manifest digest mismatch"
    );
    let manifest: Manifest =
        serde_json::from_slice(&manifest_bytes).context("invalid WACZ manifest")?;
    ensure!(
        manifest.profile == "data-package" && manifest.wacz_version == "1.1.1",
        "unsupported WACZ profile version"
    );
    ensure!(
        manifest.resources.len() == 3,
        "WACZ resource list is incomplete"
    );
    let mut resources = BTreeSet::new();
    for resource in &manifest.resources {
        ensure!(
            [WARC_PATH, INDEX_PATH, PAGES_PATH].contains(&resource.path.as_str())
                && resources.insert(resource.path.as_str()),
            "unexpected or duplicate WACZ resource path"
        );
        ensure!(
            resource.name == resource.path.rsplit('/').next().unwrap_or_default(),
            "WACZ resource name/path mismatch"
        );
        let mut entry = archive.by_name(&resource.path)?;
        ensure!(
            entry.size() == resource.bytes,
            "WACZ resource length mismatch"
        );
        if resource.path != WARC_PATH {
            ensure!(
                entry.size() <= MAX_METADATA_BYTES,
                "archive metadata exceeds 16 MiB"
            );
        }
        let (bytes, hash) = hash_reader(&mut entry)?;
        ensure!(
            bytes == resource.bytes && resource.hash == format!("sha256:{hash}"),
            "WACZ resource digest mismatch"
        );
    }
    let index = parse_index(&small_member(&mut archive, INDEX_PATH)?)?;
    ensure!(!index.is_empty(), "WACZ index has no responses");
    let warc_size = archive.by_name(WARC_PATH)?.size();
    let mut ordered: Vec<&IndexedRecord> = index.iter().collect();
    ordered.sort_by_key(|record| record.entry.offset.parse::<u64>().unwrap_or(u64::MAX));
    let mut warc = archive.by_name(WARC_PATH)?;
    let mut offset = 0u64;
    let mut record_ids = BTreeSet::new();
    let mut response_pages = Vec::with_capacity(ordered.len());
    for indexed in ordered {
        let expected_offset = decimal(&indexed.entry.offset)?;
        let length = decimal(&indexed.entry.length)?;
        ensure!(
            expected_offset == offset && length > 0,
            "WACZ index has inconsistent response ranges"
        );
        offset = offset
            .checked_add(length)
            .context("WARC index range overflow")?;
        ensure!(offset <= warc_size, "WARC index exceeds the stored member");
        let parsed = read_record(&mut warc, length)?;
        ensure!(
            parsed.url == indexed.entry.url
                && mime_type(&parsed.content_type) == indexed.entry.mime
                && parsed.status.to_string() == indexed.entry.status
                && format!("sha256:{}", parsed.sha256) == indexed.entry.digest
                && parsed.warc_record_id == indexed.entry.warc_record_id,
            "CDXJ response identity mismatch"
        );
        ensure!(
            parsed.captured_at.format("%Y%m%d%H%M%S").to_string() == indexed.timestamp
                && surt_key(&parsed.url)? == indexed.key,
            "CDXJ URL/timestamp mismatch"
        );
        ensure!(
            record_ids.insert(parsed.warc_record_id),
            "duplicate WARC record identifiers"
        );
        response_pages.push((parsed.url, parsed.captured_at));
    }
    ensure!(offset == warc_size, "WARC member has unindexed bytes");
    let mut extra = [0u8; 1];
    ensure!(
        warc.read(&mut extra)? == 0,
        "WARC member has trailing content"
    );
    drop(warc);
    let pages = parse_pages(&small_member(&mut archive, PAGES_PATH)?)?;
    let mut ids = BTreeSet::new();
    for page in &pages {
        parse_url(&page.url)?;
        validate_timestamp(page.ts)?;
        ensure!(
            response_pages
                .iter()
                .any(|(url, ts)| url == &page.url && ts == &page.ts),
            "WACZ page does not match a response"
        );
        if let Some(id) = &page.id {
            ensure!(
                !id.is_empty() && ids.insert(id),
                "duplicate or empty WACZ page identifier"
            );
        }
    }
    ensure!(
        manifest.main_page_url == pages.first().map(|page| page.url.clone())
            && manifest.main_page_date == pages.first().map(|page| rfc3339(page.ts)),
        "WACZ main page metadata mismatch"
    );
    let mut file = archive.into_inner();
    file.seek(SeekFrom::Start(0))?;
    let (bytes, sha256) = hash_reader(&mut file)?;
    run_checkpoint(&checkpoint)?;
    Ok(PackageSummary {
        record_count: index.len(),
        bytes,
        sha256,
    })
}

fn preflight_zip_directory(file: &mut (impl Read + Seek)) -> Result<()> {
    // Five entries need small directory metadata even when the WARC uses ZIP64
    // sizes and offsets. Check this before the ZIP reader allocates metadata.
    let length = file.seek(SeekFrom::End(0))?;
    let tail_length = length.min(u16::MAX as u64 + 22);
    file.seek(SeekFrom::End(-(tail_length as i64)))?;
    let mut tail = Vec::new();
    Read::by_ref(file)
        .take(tail_length)
        .read_to_end(&mut tail)?;
    let (end_offset, end) = tail
        .windows(22)
        .enumerate()
        .rev()
        .find(|(offset, bytes)| {
            bytes.starts_with(b"PK\x05\x06")
                && *offset + 22 + u16::from_le_bytes([bytes[20], bytes[21]]) as usize == tail.len()
        })
        .context("missing or truncated ZIP end record")?;
    ensure!(
        u16::from_le_bytes([end[4], end[5]]) == 0
            && u16::from_le_bytes([end[6], end[7]]) == 0
            && u16::from_le_bytes([end[8], end[9]]) as usize == MEMBER_PATHS.len()
            && u16::from_le_bytes([end[10], end[11]]) as usize == MEMBER_PATHS.len(),
        "WACZ profile requires exactly five members in one ZIP volume"
    );
    let directory_bytes = u32::from_le_bytes([end[12], end[13], end[14], end[15]]);
    let directory_offset = u32::from_le_bytes([end[16], end[17], end[18], end[19]]);
    let has_zip64 = end_offset >= 20 && tail[end_offset - 20..end_offset - 16] == *b"PK\x06\x07";
    if has_zip64 {
        let locator = &tail[end_offset - 20..end_offset];
        ensure!(
            locator[4..8] == [0; 4] && locator[16..20] == [1, 0, 0, 0],
            "unsupported ZIP64 volume layout"
        );
        let record_offset = u64::from_le_bytes(locator[8..16].try_into()?);
        let locator_offset = length - tail_length + end_offset as u64 - 20;
        ensure!(
            record_offset
                .checked_add(56)
                .is_some_and(|end| end <= locator_offset),
            "invalid ZIP64 directory record location"
        );
        file.seek(SeekFrom::Start(record_offset))?;
        let mut record = [0u8; 56];
        file.read_exact(&mut record)
            .context("truncated ZIP64 directory record")?;
        let record_size = u64::from_le_bytes(record[4..12].try_into()?);
        ensure!(
            record.starts_with(b"PK\x06\x06")
                && (44..=MAX_METADATA_BYTES).contains(&record_size)
                && record_offset.checked_add(12 + record_size) == Some(locator_offset)
                && record[16..24] == [0; 8]
                && u64::from_le_bytes(record[24..32].try_into()?) == MEMBER_PATHS.len() as u64
                && u64::from_le_bytes(record[32..40].try_into()?) == MEMBER_PATHS.len() as u64,
            "invalid or unbounded ZIP64 directory metadata"
        );
        ensure!(
            u64::from_le_bytes(record[40..48].try_into()?) <= MAX_METADATA_BYTES,
            "ZIP64 central directory exceeds 16 MiB"
        );
    } else {
        ensure!(
            directory_offset != u32::MAX,
            "missing ZIP64 directory locator"
        );
        ensure!(
            directory_bytes as u64 <= MAX_METADATA_BYTES,
            "ZIP central directory exceeds 16 MiB"
        );
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(())
}

fn small_member<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Result<Vec<u8>> {
    let mut entry = archive.by_name(name)?;
    ensure!(
        entry.size() <= MAX_METADATA_BYTES,
        "archive metadata exceeds 16 MiB"
    );
    let length = entry.size();
    let mut bytes = Vec::new();
    entry
        .by_ref()
        .take(MAX_METADATA_BYTES + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 == length && length <= MAX_METADATA_BYTES,
        "archive metadata length mismatch"
    );
    Ok(bytes)
}

fn parse_index(bytes: &[u8]) -> Result<Vec<IndexedRecord>> {
    let text = std::str::from_utf8(bytes).context("CDXJ is not UTF-8")?;
    ensure!(text.ends_with('\n'), "CDXJ lacks a terminating newline");
    let mut records = Vec::new();
    let mut previous = "";
    for line in text.lines() {
        ensure!(
            !line.is_empty() && line >= previous && !line.contains('\r'),
            "CDXJ rows are unsorted or malformed"
        );
        previous = line;
        let mut parts = line.splitn(3, ' ');
        let key = parts.next().context("missing CDXJ key")?;
        let timestamp = parts.next().context("missing CDXJ timestamp")?;
        ensure!(
            !key.is_empty()
                && key.bytes().all(|byte| byte.is_ascii_graphic())
                && timestamp.len() == 14
                && timestamp.bytes().all(|byte| byte.is_ascii_digit()),
            "invalid CDXJ key/timestamp token"
        );
        let entry: CdxEntry = serde_json::from_str(parts.next().context("missing CDXJ JSON")?)
            .context("invalid CDXJ entry")?;
        ensure!(
            entry.filename == "data.warc.gz",
            "CDXJ filename is outside the archive profile"
        );
        decimal(&entry.offset)?;
        decimal(&entry.length)?;
        parse_url(&entry.url)?;
        records.push(IndexedRecord {
            key: key.to_string(),
            timestamp: timestamp.to_string(),
            entry,
        });
    }
    Ok(records)
}

fn decimal(value: &str) -> Result<u64> {
    ensure!(
        !value.is_empty()
            && value.bytes().all(|byte| byte.is_ascii_digit())
            && (value == "0" || !value.starts_with('0')),
        "invalid decimal archive offset/length"
    );
    value.parse().context("archive offset/length overflow")
}

fn parse_pages(bytes: &[u8]) -> Result<Vec<ArchivePage>> {
    let text = std::str::from_utf8(bytes).context("pages JSONL is not UTF-8")?;
    ensure!(
        text.ends_with('\n'),
        "pages JSONL lacks a terminating newline"
    );
    let mut lines = text.lines();
    let header: serde_json::Value =
        serde_json::from_str(lines.next().context("missing pages JSONL header")?)?;
    ensure!(
        header["format"] == "json-pages-1.0" && header["id"] == "pages",
        "invalid pages JSONL header"
    );
    lines
        .map(|line| serde_json::from_str(line).context("invalid page entry"))
        .collect()
}

struct ParsedRecord {
    kind: String,
    url: String,
    content_type: String,
    size: u64,
    sha256: String,
    warc_record_id: String,
    status: u16,
    captured_at: DateTime<Utc>,
    http_version: String,
}

fn validate_source_records(
    source: &mut (impl Read + Seek),
    records: &[ArchivedRecord],
) -> Result<()> {
    let source_length = source.seek(SeekFrom::End(0))?;
    source.seek(SeekFrom::Start(0))?;
    let mut offset = 0u64;
    let mut ids = BTreeSet::new();
    for expected in records {
        ensure!(
            expected.offset == offset && expected.length > 0,
            "source WARC record ranges are inconsistent"
        );
        offset = offset
            .checked_add(expected.length)
            .context("source WARC range overflow")?;
        ensure!(
            offset <= source_length,
            "source WARC range exceeds file length"
        );
        let parsed = read_record(&mut *source, expected.length)?;
        ensure!(
            parsed.kind == expected.kind
                && parsed.url == expected.url
                && parsed.content_type == expected.content_type
                && parsed.size == expected.size
                && parsed.sha256 == expected.sha256
                && parsed.warc_record_id == expected.warc_record_id
                && parsed.status == expected.status
                && parsed.captured_at == expected.captured_at
                && parsed.http_version == expected.http_version,
            "source WARC metadata mismatch"
        );
        ensure!(
            ids.insert(parsed.warc_record_id),
            "source WARC has duplicate record identifiers"
        );
    }
    ensure!(
        offset == source_length,
        "source WARC contains unindexed bytes"
    );
    Ok(())
}

fn read_header(input: &mut impl Read) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut byte = [0u8; 1];
    while !bytes.ends_with(b"\r\n\r\n") {
        ensure!(
            bytes.len() < MAX_HEADER_BYTES,
            "archive header exceeds 128 KiB"
        );
        input
            .read_exact(&mut byte)
            .context("truncated archive header")?;
        bytes.push(byte[0]);
    }
    Ok(bytes)
}

type ParsedHeader = (String, Vec<(String, Vec<u8>)>);

fn parse_header(bytes: &[u8]) -> Result<ParsedHeader> {
    ensure!(
        bytes.ends_with(b"\r\n\r\n"),
        "invalid archive header terminator"
    );
    for (index, byte) in bytes.iter().enumerate() {
        ensure!(
            (*byte != b'\n' || index > 0 && bytes[index - 1] == b'\r')
                && (*byte != b'\r' || bytes.get(index + 1) == Some(&b'\n')),
            "archive header has a non-CRLF line ending"
        );
    }
    let mut lines = bytes[..bytes.len() - 4].split(|byte| *byte == b'\n');
    let first = lines.next().context("missing archive header line")?;
    let first = first.strip_suffix(b"\r").unwrap_or(first);
    let first = std::str::from_utf8(first)
        .context("invalid archive status line")?
        .to_string();
    ensure!(
        first
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ' || byte == b'\t'),
        "invalid archive status line"
    );
    let mut fields = Vec::new();
    for line in lines {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let separator = line
            .iter()
            .position(|byte| *byte == b':')
            .context("invalid archive header field")?;
        let name = std::str::from_utf8(&line[..separator])
            .context("invalid archive header name")?
            .to_string();
        let value = line[separator + 1..].trim_ascii().to_vec();
        fields.push((name, value));
    }
    validate_headers(&fields)?;
    Ok((first, fields))
}

fn read_record(input: impl Read, length: u64) -> Result<ParsedRecord> {
    let mut gzip = GzDecoder::new(BufReader::new(input.take(length)));
    let (version, raw_fields) = parse_header(&read_header(&mut gzip)?)?;
    ensure!(version == "WARC/1.1", "unsupported WARC version");
    let mut fields = BTreeMap::new();
    for (name, value) in raw_fields {
        ensure!(
            fields
                .insert(name.to_ascii_lowercase(), String::from_utf8(value)?)
                .is_none(),
            "duplicate WARC header field"
        );
    }
    let required = |name: &str| {
        fields
            .get(name)
            .map(String::as_str)
            .with_context(|| format!("missing WARC {name}"))
    };
    ensure!(
        required("warc-type")? == "response"
            && required("content-type")? == "application/http;msgtype=response",
        "unsupported WARC record type"
    );
    let url = required("warc-target-uri")?.to_string();
    parse_url(&url)?;
    let captured_at = DateTime::parse_from_rfc3339(required("warc-date")?)
        .context("invalid WARC timestamp")?
        .with_timezone(&Utc);
    validate_timestamp(captured_at)?;
    let record_id = required("warc-record-id")?.to_string();
    validate_record_id(&record_id)?;
    let kind = required("warc-fyi-kind")?.to_string();
    validate_kind(&kind)?;
    let http_version = required("warc-fyi-source-http-version")?.to_string();
    ensure!(
        ["HTTP/1.0", "HTTP/1.1", "HTTP/2", "HTTP/3"].contains(&http_version.as_str()),
        "invalid WARC source HTTP version"
    );
    let block_length = decimal(required("content-length")?)?;
    let http_header = read_header(&mut gzip)?;
    let payload_length = block_length
        .checked_sub(http_header.len() as u64)
        .context("WARC HTTP header exceeds content block")?;
    let (status_line, headers) = parse_header(&http_header)?;
    ensure!(
        status_line.starts_with("HTTP/1.1 "),
        "invalid reconstructed HTTP status line"
    );
    let status: u16 = status_line
        .split(' ')
        .nth(1)
        .context("missing HTTP status")?
        .parse()
        .context("invalid HTTP status")?;
    ensure!((100..=599).contains(&status), "invalid HTTP status range");
    ensure!(
        !(status < 200 || status == 204 || status == 304) || payload_length == 0,
        "bodyless response has payload bytes"
    );
    let lengths: Vec<&Vec<u8>> = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map(|(_, value)| value)
        .collect();
    ensure!(
        lengths.len() == 1 && decimal(std::str::from_utf8(lengths[0])?)? == payload_length,
        "HTTP entity length mismatch"
    );
    ensure!(
        headers
            .iter()
            .all(|(name, _)| name.eq_ignore_ascii_case("content-length")
                || !FRAMING_HEADERS.contains(&name.to_ascii_lowercase().as_str())),
        "reconstructed HTTP message contains transfer framing"
    );
    let content_type = content_type(&headers)?;
    let mut block_digest = Sha256::new();
    block_digest.update(&http_header);
    let mut payload_digest = Sha256::new();
    let mut remaining = payload_length;
    let mut buffer = [0u8; 64 * 1024];
    while remaining > 0 {
        let count = remaining.min(buffer.len() as u64) as usize;
        gzip.read_exact(&mut buffer[..count])
            .context("truncated WARC entity payload")?;
        block_digest.update(&buffer[..count]);
        payload_digest.update(&buffer[..count]);
        remaining -= count as u64;
    }
    let mut trailer = [0u8; 4];
    gzip.read_exact(&mut trailer)
        .context("truncated WARC record terminator")?;
    ensure!(trailer == *b"\r\n\r\n", "invalid WARC record terminator");
    let mut extra = [0u8; 1];
    ensure!(
        gzip.read(&mut extra)? == 0,
        "WARC gzip member contains extra records or bytes"
    );
    let buffered = gzip.into_inner();
    ensure!(
        buffered.buffer().is_empty() && buffered.get_ref().limit() == 0,
        "WARC range contains unused compressed bytes"
    );
    let payload_digest = payload_digest.finalize();
    ensure!(
        required("warc-block-digest")?
            == format!("sha256:{}", BASE32.encode(&block_digest.finalize()))
            && required("warc-payload-digest")?
                == format!("sha256:{}", BASE32.encode(&payload_digest)),
        "WARC block/payload digest mismatch"
    );
    Ok(ParsedRecord {
        kind,
        url,
        content_type,
        size: payload_length,
        sha256: hex(&payload_digest),
        warc_record_id: record_id,
        status,
        captured_at,
        http_version,
    })
}

fn validate_record_id(value: &str) -> Result<()> {
    let uuid = value
        .strip_prefix("<urn:uuid:")
        .and_then(|text| text.strip_suffix('>'))
        .context("invalid WARC record identifier")?;
    ensure!(
        uuid.len() == 36
            && uuid
                .bytes()
                .enumerate()
                .all(|(index, byte)| if [8, 13, 18, 23].contains(&index) {
                    byte == b'-'
                } else {
                    byte.is_ascii_hexdigit()
                }),
        "invalid WARC UUID record identifier"
    );
    Ok(())
}
