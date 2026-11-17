//! Bounded replay of self-pinned approvals through a caller-owned HTTP boundary.

mod evidence;
pub use evidence::{
    replay_archive_with_evidence, verify_replay_response_files, ReplayEvidencePaths,
};

use crate::acquisition_receipts::canonical_json_bytes;
use crate::provenance::sha256_hex;
use anyhow::{bail, ensure, Context, Result};
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, value::RawValue, Number, Value};
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fs;
use std::future::Future;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

/// A redirect-free response supplied by the caller's guarded HTTP boundary.
#[derive(Debug, Clone)]
pub struct ReplayResponse {
    pub url: reqwest::Url,
    pub status: reqwest::StatusCode,
    pub version: reqwest::Version,
    pub headers: reqwest::header::HeaderMap,
    pub body: Vec<u8>,
    pub captured_at: chrono::DateTime<chrono::Utc>,
}

impl ReplayResponse {
    /// Decode an entity without allowing any layer to exceed the byte cap.
    fn decoded_entity(&self, max_bytes: u64) -> Result<Vec<u8>> {
        let mut encodings = Vec::new();
        for value in self.headers.get_all(reqwest::header::CONTENT_ENCODING) {
            encodings.extend(
                value
                    .to_str()
                    .context("invalid HTTP content encoding")?
                    .split(','),
            );
        }
        ensure!(
            encodings.len() <= 4,
            "too many HTTP content encoding layers"
        );
        let mut body = Cow::Borrowed(self.body.as_slice());
        for encoding in encodings.into_iter().rev() {
            let decoded = match encoding.trim().to_ascii_lowercase().as_str() {
                "" | "identity" => continue,
                "gzip" | "x-gzip" => {
                    read_entity_bytes(flate2::read::MultiGzDecoder::new(body.as_ref()), max_bytes)
                }
                "deflate" => {
                    read_entity_bytes(flate2::read::ZlibDecoder::new(body.as_ref()), max_bytes)
                        .or_else(|_| {
                            read_entity_bytes(
                                flate2::read::DeflateDecoder::new(body.as_ref()),
                                max_bytes,
                            )
                        })
                }
                _ => bail!("unsupported HTTP content encoding"),
            }
            .context("failed to decode HTTP entity")?;
            ensure!(
                decoded.len() as u64 <= max_bytes,
                "decoded HTTP entity exceeds byte limit"
            );
            body = Cow::Owned(decoded);
        }
        ensure!(
            body.len() as u64 <= max_bytes,
            "decoded HTTP entity exceeds byte limit"
        );
        Ok(body.into_owned())
    }
}

fn read_entity_bytes(reader: impl Read, max_bytes: u64) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// A transport failure. Replay does not retry either variant automatically.
#[derive(Debug, thiserror::Error)]
pub enum ReplayFetchError {
    #[error("retryable transport error: {0}")]
    Retryable(anyhow::Error),
    #[error("fatal transport error: {0}")]
    Fatal(anyhow::Error),
}

/// Monotonic time boundary used by replay and deterministic offline tests.
pub trait ReplayRuntime: Send {
    fn now(&self) -> Duration;
    fn sleep(&mut self, duration: Duration) -> impl Future<Output = ()> + Send;
}

pub const REPLAY_ORIGIN: &str = "https://web.archive.org";
pub const REPLAY_ADAPTER_ID: &str = "internet-archive-replay";
pub const SELECTION_SCHEMA: &str = "urn:fyi-cli:internet-archive-replay-selection:1";
pub const CHECKPOINT_SCHEMA: &str = "urn:fyi-cli:internet-archive-replay-checkpoint:1";
pub const RESULT_SCHEMA: &str = "urn:fyi-cli:internet-archive-replay-result:1";
pub const MAX_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;
const MAX_SELECTION_BYTES: usize = 16 * 1024 * 1024;
const MAX_CHECKPOINT_BYTES: usize = 16 * 1024 * 1024;
// Object names contain the decimal index, a separator, a SHA-256 and `.raw`.
const MAX_OBJECT_INDEX_DIGITS: usize = 255 - 1 - 64 - 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayConfig {
    pub allowed_target_host: String,
    pub max_rows: usize,
    pub max_payload_bytes: usize,
    pub max_redirects: usize,
    pub max_runtime_seconds: f64,
    pub request_timeout_seconds: f64,
}

impl ReplayConfig {
    pub fn validate(&self) -> Result<()> {
        let host = &self.allowed_target_host;
        ensure!(
            !host.is_empty()
                && host.len() <= 253
                && host != "localhost"
                && host.bytes().all(|byte| !byte.is_ascii_uppercase())
                && host.split('.').all(|label| {
                    !label.is_empty()
                        && label.len() <= 63
                        && label.as_bytes()[0].is_ascii_alphanumeric()
                        && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                        && label
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                }),
            "allowed_target_host must be a normalized public DNS hostname"
        );
        ensure!(
            (1..=10_000).contains(&self.max_rows),
            "max_rows must be between 1 and 10000"
        );
        ensure!(
            (1..=MAX_PAYLOAD_BYTES).contains(&self.max_payload_bytes),
            "max_payload_bytes must be between 1 and {MAX_PAYLOAD_BYTES}"
        );
        ensure!(
            self.max_redirects <= 5,
            "max_redirects must be between 0 and 5"
        );
        positive_duration(self.max_runtime_seconds, "max_runtime_seconds")?;
        positive_duration(self.request_timeout_seconds, "request_timeout_seconds")?;
        Ok(())
    }

    pub fn identity(&self, selection: &ApprovedSelection) -> Value {
        json!({
            "replay_origin": REPLAY_ORIGIN,
            "allowed_target_host": self.allowed_target_host,
            "selection_sha256": selection.selection_sha256(),
            "source_cdx_sha256": selection.source_cdx_sha256(),
            "max_payload_bytes": self.max_payload_bytes
        })
    }

    pub fn request_bounds(&self, selection: &ApprovedSelection) -> Value {
        let mut value = self.identity(selection);
        value["max_rows"] = json!(self.max_rows);
        value["max_redirects"] = json!(self.max_redirects);
        value["max_runtime_seconds"] = json!(self.max_runtime_seconds);
        value["request_timeout_seconds"] = json!(self.request_timeout_seconds);
        value
    }
}

/// A schema-valid, canonical, self-pinned approval. Construct through validation.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct ApprovedSelection(SelectionDocument);

impl ApprovedSelection {
    /// Preserve unbounded JSON integer tokens in the exact canonical projection.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        canonical(&self.0)
    }

    pub fn selection_sha256(&self) -> &str {
        &self.0.selection_sha256
    }

    pub fn source_cdx_sha256(&self) -> &str {
        &self.0.source_cdx_sha256
    }

    pub fn len(&self) -> usize {
        self.0.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.rows.is_empty()
    }
}

// Fields follow canonical key order, including the omitted self-digest form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectionDocument {
    rows: Vec<ApprovedRow>,
    schema: String,
    selection_sha256: String,
    source_cdx_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovedRow {
    cdx_digest: String,
    cdx_length: ExactInteger,
    cdx_statuscode: ExactInteger,
    #[serde(
        default,
        deserialize_with = "optional_string",
        skip_serializing_if = "Option::is_none"
    )]
    expected_media_type: Option<String>,
    expected_payload_bytes: ExactInteger,
    expected_payload_sha256: String,
    expected_status: ExactInteger,
    original: String,
    row_index: ExactInteger,
    timestamp: String,
}

fn optional_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    String::deserialize(deserializer).map(Some)
}

/// The selection schema leaves integers unbounded. Keep their JSON identity
/// separate from decimal digits used for ordering, bounds and payload names.
#[derive(Debug, Clone)]
struct ExactInteger {
    json: Box<RawValue>,
    digits: String,
}

impl ExactInteger {
    fn parse(token: &str) -> Result<Self> {
        let unsigned = token.strip_prefix('-').unwrap_or(token);
        let (token, digits) = if unsigned.bytes().all(|byte| byte.is_ascii_digit()) {
            ensure!(
                !token.starts_with('-') || unsigned == "0",
                "integer must be nonnegative"
            );
            let digits = unsigned.to_string();
            (digits.clone(), digits)
        } else {
            // JSON Schema also recognizes finite, integral floating-point values.
            let value: f64 = serde_json::from_str(token).context("expected an integer")?;
            ensure!(
                value.is_finite() && value >= 0.0 && value.fract() == 0.0,
                "expected a nonnegative integer"
            );
            let token = format!("{value:?}");
            let token = if let Some((mantissa, exponent)) = token.split_once('e') {
                let exponent: i32 = exponent.parse()?;
                format!("{mantissa}e{exponent:+03}")
            } else {
                token
            };
            (
                token,
                format!("{value:.0}").trim_start_matches('-').to_string(),
            )
        };
        Ok(Self {
            json: RawValue::from_string(token)?,
            digits,
        })
    }

    fn bounded(&self, maximum: usize, label: &str) -> Result<usize> {
        let value = self
            .digits
            .parse::<usize>()
            .with_context(|| format!("{label} exceeds its bound"))?;
        ensure!(value <= maximum, "{label} exceeds its bound");
        Ok(value)
    }

    fn same(&self, other: &Self) -> bool {
        self.json.get() == other.json.get()
    }
}

impl Serialize for ExactInteger {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.json.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ExactInteger {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Box::<RawValue>::deserialize(deserializer)?;
        Self::parse(raw.get()).map_err(serde::de::Error::custom)
    }
}

#[derive(Serialize)]
struct UnsignedSelection<'a> {
    rows: &'a [ApprovedRow],
    schema: &'a str,
    source_cdx_sha256: &'a str,
}

pub fn validate_selection_bytes(bytes: &[u8]) -> Result<ApprovedSelection> {
    ensure!(
        bytes.len() <= MAX_SELECTION_BYTES,
        "replay selection exceeds byte limit"
    );
    let selection: SelectionDocument =
        serde_json::from_slice(bytes).context("invalid replay selection")?;
    ensure!(
        selection.schema == SELECTION_SCHEMA,
        "invalid replay selection schema"
    );
    ensure!(
        !selection.rows.is_empty(),
        "approved replay selection must contain rows"
    );
    validate_digest(&selection.selection_sha256)?;
    validate_digest(&selection.source_cdx_sha256)?;
    let unsigned = UnsignedSelection {
        rows: &selection.rows,
        schema: &selection.schema,
        source_cdx_sha256: &selection.source_cdx_sha256,
    };
    ensure!(
        selection.selection_sha256 == sha256_hex(&canonical(&unsigned)?),
        "invalid replay selection: selection_sha256 mismatch"
    );
    let mut identities = BTreeSet::new();
    let mut previous: Option<&ExactInteger> = None;
    for row in &selection.rows {
        canonical_target_url(&row.original, None)?;
        ensure!(
            row.timestamp.len() == 14 && row.timestamp.bytes().all(|byte| byte.is_ascii_digit()),
            "approved timestamp must contain 14 ASCII digits"
        );
        ensure!(
            (1..=256).contains(&row.cdx_digest.chars().count()),
            "invalid approved CDX digest"
        );
        for (value, label) in [
            (&row.cdx_statuscode, "cdx_statuscode"),
            (&row.expected_status, "expected_status"),
        ] {
            ensure!(
                value.bounded(599, label)? >= 100,
                "{label} must be between 100 and 599"
            );
        }
        validate_digest(&row.expected_payload_sha256)?;
        if let Some(media_type) = &row.expected_media_type {
            ensure!(valid_media_type(media_type), "invalid approved media type");
        }
        ensure!(
            identities.insert((&row.original, &row.timestamp)),
            "approved replay rows must have unique URL/timestamp identities"
        );
        if let Some(previous) = previous {
            ensure!(
                (previous.digits.len(), &previous.digits)
                    < (row.row_index.digits.len(), &row.row_index.digits),
                "approved replay row indexes must be unique and ordered"
            );
        }
        previous = Some(&row.row_index);
    }
    Ok(ApprovedSelection(selection))
}

fn valid_media_type(value: &str) -> bool {
    static MEDIA_TYPE: OnceLock<Regex> = OnceLock::new();
    MEDIA_TYPE
        .get_or_init(|| Regex::new(r"\A[a-z0-9.+-]+/[a-z0-9.+-]+\z").unwrap())
        .is_match(value)
}

fn validate_digest(value: &str) -> Result<()> {
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "invalid lowercase SHA-256 digest"
    );
    Ok(())
}

fn canonical(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn positive_duration(seconds: f64, label: &str) -> Result<Duration> {
    let duration = Duration::try_from_secs_f64(seconds)
        .with_context(|| format!("{label} must be finite and positive"))?;
    ensure!(!duration.is_zero(), "{label} must be positive");
    Ok(duration)
}

fn canonical_target_url(value: &str, allowed_host: Option<&str>) -> Result<()> {
    ensure!(
        value.starts_with("https://")
            && !value.contains(['\\', '#'])
            && !value
                .chars()
                .any(|character| character.is_whitespace() || character < '\u{20}'),
        "original must be a canonical HTTPS URL"
    );
    let parsed = reqwest::Url::parse(value).context("original must be a canonical HTTPS URL")?;
    let (authority, suffix) = value[8..]
        .split_once('/')
        .context("original must include a canonical HTTPS path")?;
    ensure!(
        !authority.is_empty()
            && !authority.contains(['@', '?'])
            && parsed.host_str().is_some()
            && parsed.port_or_known_default() == Some(443),
        "original must be a canonical HTTPS URL"
    );
    let (path, query) = suffix
        .split_once('?')
        .map_or((suffix, None), |(path, query)| (path, Some(query)));
    ensure!(
        !path.split('/').any(|segment| matches!(segment, "." | "..")) && query != Some(""),
        "original must be a canonical HTTPS URL"
    );
    let raw_host = if authority.starts_with('[') {
        authority
            .split_once(']')
            .context("invalid target hostname")?
            .0
            .trim_start_matches('[')
    } else {
        authority.split(':').next().unwrap_or("")
    };
    if let Some(allowed_host) = allowed_host {
        ensure!(
            raw_host.to_lowercase() == allowed_host,
            "approved row target host does not match allowed target host"
        );
    }
    Ok(())
}

fn validate_replay_url(value: &str, original: &str) -> Result<()> {
    let parsed = reqwest::Url::parse(value).context("Wayback redirect URL is invalid")?;
    ensure!(
        value.starts_with("https://")
            && parsed.host_str() == Some("web.archive.org")
            && parsed.port_or_known_default() == Some(443)
            && parsed.username().is_empty()
            && parsed.password().is_none()
            && parsed.fragment().is_none(),
        "Wayback redirect escaped the archive host allowlist"
    );
    let (authority, path) = value[8..]
        .split_once('/')
        .context("Wayback redirect does not identify a raw replay target")?;
    ensure!(
        !authority.contains('@'),
        "Wayback redirect escaped the archive host allowlist"
    );
    static REPLAY_PATH: OnceLock<Regex> = OnceLock::new();
    let captures = REPLAY_PATH
        .get_or_init(|| Regex::new(r"\Aweb/(\d{14})id_/(https://.+)\z").unwrap())
        .captures(path)
        .context("Wayback redirect does not identify a raw replay target")?;
    ensure!(
        captures.get(2).unwrap().as_str() == original,
        "Wayback redirect changed the approved target"
    );
    ensure!(
        parsed.as_str() == value,
        "Wayback URL changes under transport normalization"
    );
    Ok(())
}

/// One physical exchange. Its body and headers retain the raw encoded entity.
/// A failed transport is observed once with no response and is never retried.
pub struct ReplayObservation<'a> {
    pub requested_url: &'a str,
    pub response: Option<&'a ReplayResponse>,
    pub attempts: usize,
    pub retry_delays_seconds: &'a [Number],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayObject {
    bytes: usize,
    cdx_digest: String,
    final_url: String,
    media_type: String,
    original: String,
    path: String,
    row_index: ExactInteger,
    sha256: String,
    status: u16,
    timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayResult {
    objects: Vec<ReplayObject>,
    schema: &'static str,
    selection_sha256: String,
    source_cdx_sha256: String,
}

impl ReplayResult {
    /// Exact result bytes, including approvals with integer identifiers above u64.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        canonical(self)
    }
}

pub fn write_replay_result(path: &Path, result: &ReplayResult) -> Result<()> {
    replace_atomic(path, &result.canonical_bytes()?)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    checkpoint_sha256: String,
    completed: Vec<ReplayObject>,
    config: Value,
    config_sha256: String,
    next_position: usize,
    schema: String,
}

#[derive(Serialize)]
struct UnsignedCheckpoint<'a> {
    completed: &'a [ReplayObject],
    config: &'a Value,
    config_sha256: &'a str,
    next_position: usize,
    schema: &'a str,
}

impl Checkpoint {
    fn unsigned(&self) -> UnsignedCheckpoint<'_> {
        UnsignedCheckpoint {
            completed: &self.completed,
            config: &self.config,
            config_sha256: &self.config_sha256,
            next_position: self.next_position,
            schema: &self.schema,
        }
    }
}

fn object_name(row: &ApprovedRow) -> String {
    format!(
        "{:0>6}-{}.raw",
        row.row_index.digits, row.expected_payload_sha256
    )
}

fn load_checkpoint(
    path: &Path,
    config: &ReplayConfig,
    selection: &ApprovedSelection,
    output: &Path,
) -> Result<Vec<ReplayObject>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "replay checkpoint must be a regular file"
    );
    let mut checkpoint_bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_CHECKPOINT_BYTES as u64 + 1)
        .read_to_end(&mut checkpoint_bytes)?;
    ensure!(
        checkpoint_bytes.len() <= MAX_CHECKPOINT_BYTES,
        "replay checkpoint exceeds byte limit"
    );
    let checkpoint: Checkpoint =
        serde_json::from_slice(&checkpoint_bytes).context("replay checkpoint is unreadable")?;
    ensure!(
        checkpoint.checkpoint_sha256 == sha256_hex(&canonical(&checkpoint.unsigned())?),
        "replay checkpoint self-digest mismatch"
    );
    let identity = config.identity(selection);
    ensure!(
        checkpoint.schema == CHECKPOINT_SCHEMA && checkpoint.config == identity,
        "replay checkpoint configuration mismatch"
    );
    ensure!(
        checkpoint.config_sha256 == sha256_hex(&canonical_json_bytes(&identity)?),
        "replay checkpoint configuration digest mismatch"
    );
    ensure!(
        checkpoint.next_position == checkpoint.completed.len()
            && checkpoint.next_position <= selection.len(),
        "replay checkpoint progress is invalid"
    );
    for (item, row) in checkpoint.completed.iter().zip(&selection.0.rows) {
        ensure!(
            item.row_index.same(&row.row_index) && item.path == object_name(row),
            "replay checkpoint ordering or object path changed"
        );
        ensure!(
            item.original == row.original
                && item.timestamp == row.timestamp
                && item.cdx_digest == row.cdx_digest
                && usize::from(item.status)
                    == row.expected_status.bounded(599, "expected_status")?
                && item.bytes
                    == row
                        .expected_payload_bytes
                        .bounded(config.max_payload_bytes, "approved payload length")?
                && item.sha256 == row.expected_payload_sha256,
            "replay checkpoint object does not match approval"
        );
        ensure!(
            item.media_type == item.media_type.trim().to_ascii_lowercase()
                && !item.media_type.contains(';')
                && row
                    .expected_media_type
                    .as_ref()
                    .is_none_or(|expected| expected == &item.media_type),
            "replay checkpoint media type does not match approval"
        );
        validate_replay_url(&item.final_url, &row.original)?;
        let payload = read_regular_payload(&output.join(&item.path), config.max_payload_bytes)?;
        ensure!(
            payload.len() == item.bytes && sha256_hex(&payload) == item.sha256,
            "replay checkpoint object integrity mismatch"
        );
    }
    Ok(checkpoint.completed)
}

fn checkpoint_bytes(
    config: &ReplayConfig,
    selection: &ApprovedSelection,
    completed: &[ReplayObject],
) -> Result<Vec<u8>> {
    let identity = config.identity(selection);
    let mut checkpoint = Checkpoint {
        checkpoint_sha256: String::new(),
        completed: completed.to_vec(),
        config_sha256: sha256_hex(&canonical_json_bytes(&identity)?),
        config: identity,
        next_position: completed.len(),
        schema: CHECKPOINT_SCHEMA.to_string(),
    };
    checkpoint.checkpoint_sha256 = sha256_hex(&canonical(&checkpoint.unsigned())?);
    let bytes = canonical(&checkpoint)?;
    ensure!(
        bytes.len() <= MAX_CHECKPOINT_BYTES,
        "replay checkpoint exceeds byte limit"
    );
    Ok(bytes)
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized)
}

fn reject_symlink_ancestors(path: &Path) -> Result<()> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    for ancestor in absolute.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => ensure!(
                !metadata.file_type().is_symlink(),
                "refusing symlink in replay path"
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn replace_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    reject_symlink_ancestors(path)?;
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    reject_symlink_ancestors(path)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn read_regular_payload(path: &Path, cap: usize) -> Result<Vec<u8>> {
    reject_symlink_ancestors(path)?;
    let metadata = fs::symlink_metadata(path).context("replay object is missing")?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "replay object path is not a regular file"
    );
    let mut payload = Vec::new();
    fs::File::open(path)?
        .take(cap as u64 + 1)
        .read_to_end(&mut payload)?;
    ensure!(payload.len() <= cap, "replay object exceeds the byte cap");
    Ok(payload)
}

fn persist_payload(path: &Path, payload: &[u8], cap: usize) -> Result<()> {
    reject_symlink_ancestors(path)?;
    if fs::symlink_metadata(path).is_ok() {
        ensure!(
            read_regular_payload(path, cap)? == payload,
            "replay object collision or corruption"
        );
        return Ok(());
    }
    let mut temporary =
        tempfile::NamedTempFile::new_in(path.parent().context("replay object has no directory")?)?;
    temporary.write_all(payload)?;
    temporary.as_file().sync_all()?;
    if let Err(error) = temporary.persist_noclobber(path) {
        ensure!(
            error.error.kind() == std::io::ErrorKind::AlreadyExists,
            "failed to publish replay object: {}",
            error.error
        );
        ensure!(
            read_regular_payload(path, cap)? == payload,
            "replay object collision or corruption"
        );
    }
    Ok(())
}

struct TokioRuntime(tokio::time::Instant);

impl ReplayRuntime for TokioRuntime {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
    fn sleep(&mut self, duration: Duration) -> impl Future<Output = ()> + Send {
        tokio::time::sleep(duration)
    }
}

/// Replay each approved row with zero retries and one observed exchange per hop.
/// The caller bounds encoded receive bytes and supplies fresh redirect-free GETs.
pub async fn replay_approved_rows<F, Fut, O>(
    selection: &ApprovedSelection,
    config: &ReplayConfig,
    output_dir: &Path,
    checkpoint_path: &Path,
    fetch: F,
    observer: O,
) -> Result<ReplayResult>
where
    F: FnMut(String, Duration, usize) -> Fut,
    Fut: Future<Output = Result<ReplayResponse, ReplayFetchError>>,
    O: FnMut(&ReplayObservation<'_>) -> Result<()>,
{
    let mut runtime = TokioRuntime(tokio::time::Instant::now());
    replay_approved_rows_with_runtime(
        selection,
        config,
        output_dir,
        checkpoint_path,
        fetch,
        observer,
        &mut runtime,
    )
    .await
}

pub async fn replay_approved_rows_with_runtime<F, Fut, O, R>(
    selection: &ApprovedSelection,
    config: &ReplayConfig,
    output_dir: &Path,
    checkpoint_path: &Path,
    fetch: F,
    observer: O,
    runtime: &mut R,
) -> Result<ReplayResult>
where
    F: FnMut(String, Duration, usize) -> Fut,
    Fut: Future<Output = Result<ReplayResponse, ReplayFetchError>>,
    O: FnMut(&ReplayObservation<'_>) -> Result<()>,
    R: ReplayRuntime,
{
    config.validate()?;
    ensure!(
        selection.len() <= config.max_rows,
        "approved replay selection exceeds max_rows"
    );
    for row in &selection.0.rows {
        canonical_target_url(&row.original, Some(&config.allowed_target_host))?;
        ensure!(
            row.row_index.digits.len() <= MAX_OBJECT_INDEX_DIGITS,
            "replay object name exceeds limit"
        );
        row.expected_payload_bytes
            .bounded(config.max_payload_bytes, "approved payload length")?;
    }
    reject_symlink_ancestors(output_dir)?;
    reject_symlink_ancestors(checkpoint_path)?;
    ensure!(
        !absolute_path(checkpoint_path)?.starts_with(absolute_path(output_dir)?),
        "replay checkpoint must be outside the payload directory"
    );
    fs::create_dir_all(output_dir)?;
    ensure!(
        fs::metadata(output_dir)?.is_dir(),
        "replay output must be a plain directory"
    );
    let mut completed = load_checkpoint(checkpoint_path, config, selection, output_dir)?;
    let deadline = runtime
        .now()
        .checked_add(positive_duration(
            config.max_runtime_seconds,
            "max_runtime_seconds",
        )?)
        .context("replay runtime deadline exceeds clock range")?;
    let mut runner = Runner {
        fetch,
        observer,
        runtime,
        deadline,
        config,
    };
    for row in &selection.0.rows[completed.len()..] {
        let (response, payload, final_url) = runner.fetch_row(row).await?;
        let expected_status = row.expected_status.bounded(599, "expected_status")?;
        ensure!(
            usize::from(response.status.as_u16()) == expected_status,
            "Wayback response status does not match approval"
        );
        ensure!(
            payload.len()
                == row
                    .expected_payload_bytes
                    .bounded(config.max_payload_bytes, "approved payload length")?,
            "Wayback payload length does not match approval"
        );
        let digest = sha256_hex(&payload);
        ensure!(
            digest == row.expected_payload_sha256,
            "Wayback payload digest does not match approval"
        );
        let media_type = response
            .headers
            .get(reqwest::header::CONTENT_TYPE)
            .map(|header| header.to_str())
            .transpose()
            .context("invalid Wayback media type header")?
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        ensure!(
            row.expected_media_type
                .as_ref()
                .is_none_or(|expected| expected == &media_type),
            "Wayback media type does not match approval"
        );
        let name = object_name(row);
        let path = output_dir.join(&name);
        completed.push(ReplayObject {
            bytes: payload.len(),
            cdx_digest: row.cdx_digest.clone(),
            final_url,
            media_type,
            original: row.original.clone(),
            path: name,
            row_index: row.row_index.clone(),
            sha256: digest,
            status: response.status.as_u16(),
            timestamp: row.timestamp.clone(),
        });
        // Validate the complete next checkpoint before publishing either file.
        // Unbounded response metadata must not replace resumable progress with
        // a checkpoint that the reader cannot reopen under its byte limit.
        let checkpoint = checkpoint_bytes(config, selection, &completed)?;
        persist_payload(&path, &payload, config.max_payload_bytes)?;
        replace_atomic(checkpoint_path, &checkpoint)?;
    }
    Ok(ReplayResult {
        objects: completed,
        schema: RESULT_SCHEMA,
        selection_sha256: selection.selection_sha256().to_string(),
        source_cdx_sha256: selection.source_cdx_sha256().to_string(),
    })
}

struct Runner<'a, F, O, R> {
    fetch: F,
    observer: O,
    runtime: &'a mut R,
    deadline: Duration,
    config: &'a ReplayConfig,
}

impl<F, Fut, O, R> Runner<'_, F, O, R>
where
    F: FnMut(String, Duration, usize) -> Fut,
    Fut: Future<Output = Result<ReplayResponse, ReplayFetchError>>,
    O: FnMut(&ReplayObservation<'_>) -> Result<()>,
    R: ReplayRuntime,
{
    async fn fetch_row(&mut self, row: &ApprovedRow) -> Result<(ReplayResponse, Vec<u8>, String)> {
        let mut url = format!("{REPLAY_ORIGIN}/web/{}id_/{}", row.timestamp, row.original);
        validate_replay_url(&url, &row.original)?;
        for redirects in 0..=self.config.max_redirects {
            let remaining = self
                .deadline
                .checked_sub(self.runtime.now())
                .filter(|duration| !duration.is_zero())
                .context("Wayback replay exceeded its runtime deadline")?;
            let timeout = positive_duration(
                self.config.request_timeout_seconds,
                "request_timeout_seconds",
            )?
            .min(remaining);
            let fetched = tokio::time::timeout(
                timeout,
                (self.fetch)(url.clone(), timeout, self.config.max_payload_bytes),
            )
            .await;
            let response = match fetched {
                Ok(Ok(response)) => response,
                failed => {
                    (self.observer)(&ReplayObservation {
                        requested_url: &url,
                        response: None,
                        attempts: 1,
                        retry_delays_seconds: &[],
                    })?;
                    match failed {
                        Ok(Err(error)) => return Err(error.into()),
                        Err(error) => {
                            return Err(error).context("Wayback replay exchange timed out")
                        }
                        Ok(Ok(_)) => unreachable!(),
                    }
                }
            };
            ensure!(
                response.body.len() <= self.config.max_payload_bytes,
                "Wayback response exceeded the byte cap"
            );
            ensure!(
                response.url.as_str() == url,
                "Wayback transport response URL differs from its request"
            );
            (self.observer)(&ReplayObservation {
                requested_url: &url,
                response: Some(&response),
                attempts: 1,
                retry_delays_seconds: &[],
            })?;
            ensure!(
                self.runtime.now() < self.deadline,
                "Wayback replay exceeded its runtime deadline"
            );
            let lengths = response
                .headers
                .get_all(reqwest::header::CONTENT_LENGTH)
                .iter()
                .collect::<Vec<_>>();
            ensure!(lengths.len() <= 1, "Wayback declared length is ambiguous");
            if let Some(declared) = lengths.first() {
                let declared = declared
                    .to_str()
                    .context("Wayback declared length is invalid")?
                    .trim()
                    .parse::<usize>()
                    .context("Wayback declared length is invalid")?;
                ensure!(
                    declared == response.body.len(),
                    "Wayback declared length does not match encoded entity"
                );
            }
            let payload = response.decoded_entity(self.config.max_payload_bytes as u64)?;
            if !matches!(response.status.as_u16(), 301 | 302 | 303 | 307 | 308) {
                return Ok((response, payload, url));
            }
            ensure!(
                redirects < self.config.max_redirects,
                "Wayback redirect limit exceeded"
            );
            let location = response
                .headers
                .get(reqwest::header::LOCATION)
                .context("Wayback redirect must be an absolute approved URL")?
                .to_str()
                .context("invalid Wayback redirect header")?;
            ensure!(
                location.starts_with("https://"),
                "Wayback redirect must be an absolute approved URL"
            );
            validate_replay_url(location, &row.original)?;
            url = location.to_string();
        }
        bail!("Wayback redirect limit exceeded")
    }
}
