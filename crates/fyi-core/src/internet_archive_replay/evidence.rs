//! Evidence-producing archive replay. All transport is limited to approved rows.

use super::archive::{preflight_archive, ReplayArchive, ReplayArchivePaths};
use super::{
    absolute_path, reject_symlink_ancestors, replay_approved_rows, validate_selection_bytes,
    ApprovedSelection, ReplayConfig, ReplayFetchError, ReplayObservation, ReplayResponse,
    ReplayResult, MAX_CHECKPOINT_BYTES, MAX_SELECTION_BYTES, REPLAY_ADAPTER_ID, REPLAY_ORIGIN,
};
use crate::acquisition_receipts::{
    canonical_json_bytes, verify_receipt, verify_receipt_result_bytes, write_evidence_bytes_atomic,
    write_receipt_atomic,
};
use crate::provenance::sha256_hex;
use crate::sync::WaybackTransport;
use crate::time_util::now_rfc3339_millis_z;
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::cell::Cell;
use std::fs;
use std::future::Future;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// All evidence locations are supplied explicitly and must not overlap.
pub struct ReplayEvidencePaths<'a> {
    pub selection: &'a Path,
    pub output_dir: &'a Path,
    pub responses_dir: &'a Path,
    pub checkpoint: &'a Path,
    pub result: &'a Path,
    pub receipt: &'a Path,
}

/// Fetch only self-digested approved rows and persist response bodies and receipt.
///
/// Each redirect hop is captured separately. An exchange that does not yield a
/// complete response cannot be represented by receipt v1: the operation fails
/// without publishing a misleading receipt, leaving any prior raw bodies intact.
pub async fn replay_archive_with_evidence(
    paths: &ReplayEvidencePaths<'_>,
    config: &ReplayConfig,
) -> Result<ReplayResult> {
    let selection = prepare(paths, config)?;
    let transport = WaybackTransport::new()?;
    replay_evidenced_with_archive(paths, config, &selection, None, |url, timeout, limit| {
        let transport = &transport;
        async move {
            transport
                .fetch(&url, timeout, limit)
                .await
                .map_err(|_| ReplayFetchError::Fatal(anyhow::anyhow!("archive transport failed")))
        }
    })
    .await
}

/// Replay approved rows and publish a WACZ and receipt linkage for the observed responses.
pub async fn replay_archive_with_archive(
    paths: &ReplayEvidencePaths<'_>,
    config: &ReplayConfig,
    archive: &ReplayArchivePaths<'_>,
) -> Result<ReplayResult> {
    config.validate()?;
    preflight_archive(paths, archive)?;
    let selection = prepare(paths, config)?;
    let mut capture = ReplayArchive::new(archive)?;
    let transport = WaybackTransport::new()?;
    replay_evidenced_with_archive(
        paths,
        config,
        &selection,
        Some(&mut capture),
        |url, timeout, limit| {
            let transport = &transport;
            async move {
                transport.fetch(&url, timeout, limit).await.map_err(|_| {
                    ReplayFetchError::Fatal(anyhow::anyhow!("archive transport failed"))
                })
            }
        },
    )
    .await
}

pub(super) fn read_regular_bounded(path: &Path, cap: usize, label: &str) -> Result<Vec<u8>> {
    reject_symlink_ancestors(path)?;
    let metadata = fs::symlink_metadata(path).with_context(|| format!("{label} is missing"))?;
    ensure!(
        metadata.file_type().is_file(),
        "{label} must be a regular file"
    );
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= cap, "{label} exceeds byte limit");
    Ok(bytes)
}

fn checkpoint_digest(path: &Path) -> Result<Option<String>> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(Some(sha256_hex(&read_regular_bounded(
            path,
            MAX_CHECKPOINT_BYTES,
            "replay checkpoint",
        )?))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn require_absent(path: &Path, label: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => anyhow::bail!("{label} already exists; choose a fresh evidence path"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn prepare(paths: &ReplayEvidencePaths<'_>, config: &ReplayConfig) -> Result<ApprovedSelection> {
    config.validate()?;
    let raw_paths = [
        paths.selection,
        paths.output_dir,
        paths.responses_dir,
        paths.checkpoint,
        paths.result,
        paths.receipt,
    ];
    for path in raw_paths {
        reject_symlink_ancestors(path)?;
    }
    let normalized: Vec<PathBuf> = raw_paths
        .iter()
        .map(|path| absolute_path(path))
        .collect::<Result<_>>()?;
    for (index, path) in normalized.iter().enumerate() {
        for other in normalized.iter().skip(index + 1) {
            ensure!(
                path != other,
                "replay input and evidence paths must be distinct"
            );
        }
    }
    for directory in [&normalized[1], &normalized[2]] {
        for path in &normalized {
            if path != directory {
                ensure!(
                    !path.starts_with(directory),
                    "replay evidence paths must be outside payload and response directories"
                );
            }
        }
    }
    let selection = validate_selection_bytes(&read_regular_bounded(
        paths.selection,
        MAX_SELECTION_BYTES,
        "replay selection",
    )?)?;
    // Every redirect can produce another recorded URL. Bound its worst-case
    // JSON escaping and fixed response metadata before the first request so a
    // successful run cannot produce an unreadably large v1 receipt.
    let mut receipt_bound = 4096_usize;
    for row in &selection.0.rows {
        let per_response = row
            .original
            .len()
            .checked_add(100)
            .and_then(|length| length.checked_mul(6))
            .and_then(|length| length.checked_add(1024))
            .context("replay receipt bound overflow")?;
        let row_bound = per_response
            .checked_mul(config.max_redirects + 1)
            .context("replay receipt bound overflow")?;
        receipt_bound = receipt_bound
            .checked_add(row_bound)
            .context("replay receipt bound overflow")?;
    }
    ensure!(
        receipt_bound <= crate::receipt_wire::MAX_RECEIPT_BYTES,
        "approved selection can exceed the receipt byte limit"
    );
    checkpoint_digest(paths.checkpoint)?;
    require_absent(paths.result, "replay result")?;
    require_absent(paths.receipt, "replay receipt")?;
    match fs::symlink_metadata(paths.responses_dir) {
        Ok(metadata) => {
            ensure!(
                metadata.file_type().is_dir(),
                "response path must be a directory"
            );
            ensure!(
                fs::read_dir(paths.responses_dir)?.next().is_none(),
                "response directory must be empty for each replay run"
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    for path in [paths.result, paths.receipt, paths.checkpoint] {
        ensure!(
            path.file_name().is_some(),
            "replay evidence path must name a file"
        );
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        reject_symlink_ancestors(path)?;
    }
    fs::create_dir_all(paths.responses_dir)?;
    reject_symlink_ancestors(paths.responses_dir)?;
    Ok(selection)
}

pub(super) fn sanitized_response_url(response: &ReplayResponse) -> Result<String> {
    let mut url = response.url.clone();
    ensure!(
        url.username().is_empty() && url.password().is_none(),
        "response URL contains credentials"
    );
    if url.query().is_some() {
        url.set_query(Some("REDACTED"));
    }
    url.set_fragment(None);
    Ok(url.to_string())
}

/// Verify the ordered raw response files belonging to a replay receipt.
pub fn verify_replay_response_files(receipt: &Value, directory: &Path) -> Result<()> {
    verify_receipt(receipt)?;
    ensure!(
        receipt["command"] == "internet-archive-replay"
            && receipt["adapter"]["id"] == REPLAY_ADAPTER_ID,
        "receipt is not for Wayback replay"
    );
    reject_symlink_ancestors(directory)?;
    ensure!(
        fs::symlink_metadata(directory)?.file_type().is_dir(),
        "replay response path must be a directory"
    );
    let responses = receipt["responses"]
        .as_array()
        .context("receipt has no response list")?;
    let mut count = 0;
    for entry in fs::read_dir(directory)? {
        entry?;
        count += 1;
        ensure!(
            count <= responses.len(),
            "replay response file count mismatch"
        );
    }
    ensure!(
        count == responses.len(),
        "replay response file count mismatch"
    );
    for (index, response) in responses.iter().enumerate() {
        let digest = response["sha256"]
            .as_str()
            .context("response digest missing")?;
        let size = response["bytes"]
            .as_u64()
            .or_else(|| {
                let value = response["bytes"].as_f64()?;
                (value.is_finite()
                    && (0.0..=super::MAX_PAYLOAD_BYTES as f64).contains(&value)
                    && value.fract() == 0.0)
                    .then_some(value as u64)
            })
            .context("response byte count is not an unsigned integer")?;
        ensure!(
            size <= super::MAX_PAYLOAD_BYTES as u64,
            "response byte count exceeds the replay limit"
        );
        let file = directory.join(format!("{index:06}-{digest}.raw"));
        let bytes = read_regular_bounded(&file, super::MAX_PAYLOAD_BYTES, "replay response")?;
        ensure!(
            bytes.len() as u64 == size,
            "replay response byte count mismatch"
        );
        ensure!(
            sha256_hex(&bytes) == digest,
            "replay response digest mismatch"
        );
    }
    Ok(())
}

fn observe_response(
    observation: &ReplayObservation<'_>,
    responses_dir: &Path,
    responses: &mut Vec<Value>,
    incomplete: &mut bool,
    max_bytes: usize,
) -> Result<()> {
    *incomplete = true;
    ensure!(
        observation.attempts == 1 && observation.retry_delays_seconds.is_empty(),
        "replay observation has unsupported retry evidence"
    );
    let Some(response) = observation.response else {
        // Receipt v1 cannot encode a physical request with no response status.
        return Ok(());
    };
    ensure!(
        response.body.len() <= max_bytes,
        "response exceeds the byte limit"
    );
    let digest = sha256_hex(&response.body);
    let name = format!("{:06}-{digest}.raw", responses.len());
    let path = responses_dir.join(name);
    reject_symlink_ancestors(&path)?;
    write_evidence_bytes_atomic(&path, &response.body)?;
    responses.push(json!({
        "url": sanitized_response_url(response)?,
        "status": response.status.as_u16(),
        "bytes": response.body.len(),
        "sha256": digest,
        "attempts": 1,
        "retry_delays_seconds": [],
    }));
    *incomplete = false;
    Ok(())
}

#[cfg(test)]
async fn replay_evidenced<F, Fut>(
    paths: &ReplayEvidencePaths<'_>,
    config: &ReplayConfig,
    selection: &ApprovedSelection,
    fetch: F,
) -> Result<ReplayResult>
where
    F: FnMut(String, Duration, usize) -> Fut,
    Fut: Future<Output = Result<ReplayResponse, ReplayFetchError>>,
{
    replay_evidenced_with_archive(paths, config, selection, None, fetch).await
}

async fn replay_evidenced_with_archive<F, Fut>(
    paths: &ReplayEvidencePaths<'_>,
    config: &ReplayConfig,
    selection: &ApprovedSelection,
    mut archive: Option<&mut ReplayArchive>,
    mut fetch: F,
) -> Result<ReplayResult>
where
    F: FnMut(String, Duration, usize) -> Fut,
    Fut: Future<Output = Result<ReplayResponse, ReplayFetchError>>,
{
    let before = checkpoint_digest(paths.checkpoint)?;
    let started = now_rfc3339_millis_z();
    let mut responses = Vec::new();
    let mut incomplete = false;
    let attempted = Cell::new(0_usize);
    let observed = Cell::new(0_usize);
    let outcome = replay_approved_rows(
        selection,
        config,
        paths.output_dir,
        paths.checkpoint,
        |url, timeout, max_bytes| {
            attempted.set(attempted.get() + 1);
            fetch(url, timeout, max_bytes)
        },
        |observation| {
            observed.set(observed.get() + 1);
            observe_response(
                observation,
                paths.responses_dir,
                &mut responses,
                &mut incomplete,
                config.max_payload_bytes,
            )?;
            if let (Some(capture), Some(response)) = (archive.as_mut(), observation.response) {
                capture.append(response)?;
            }
            Ok(())
        },
    )
    .await;
    if incomplete || attempted.get() != observed.get() {
        return Err(outcome
            .err()
            .unwrap_or_else(|| anyhow::anyhow!("incomplete exchange")))
        .context("replay evidence is incomplete; no receipt was published");
    }
    let result_bytes = match &outcome {
        Ok(result) => result.canonical_bytes()?,
        Err(_) => canonical_json_bytes(&json!({
            "schema": "urn:fyi-cli:internet-archive-replay-failure:1",
            "status": "failed",
        }))?,
    };
    let mut total_bytes = 0_u64;
    for response in &responses {
        total_bytes = total_bytes
            .checked_add(
                response["bytes"]
                    .as_u64()
                    .context("invalid response length")?,
            )
            .context("response byte count overflow")?;
    }
    let after = checkpoint_digest(paths.checkpoint)?;
    let mut receipt = json!({
        "schema": "urn:fyi-cli:acquisition-receipt",
        "schema_version": "1.0.0",
        "command": "internet-archive-replay",
        "adapter": {"id": REPLAY_ADAPTER_ID, "version": env!("CARGO_PKG_VERSION")},
        "source": {"url": format!("{REPLAY_ORIGIN}/")},
        "request_bounds": config.request_bounds(selection),
        "started_at": started,
        "completed_at": now_rfc3339_millis_z(),
        "status": if outcome.is_ok() { "succeeded" } else { "failed" },
        "responses": responses,
        "totals": {"requests": responses.len(), "bytes": total_bytes, "retries": 0},
        "rate_limit": {"name": null, "minimum_interval_seconds": null},
        "checkpoint": {"before_sha256": before, "after_sha256": after},
        "result": {
            "representation": "canonical_result_projection", "media_type": "application/json",
            "bytes": result_bytes.len(), "sha256": sha256_hex(&result_bytes),
        },
    });
    if outcome.is_err() {
        receipt["failure"] = json!({"type": "ReplayFailed"});
    }
    receipt["receipt_sha256"] = Value::String(sha256_hex(&canonical_json_bytes(&receipt)?));
    verify_receipt(&receipt)?;
    verify_receipt_result_bytes(&receipt, &result_bytes)?;
    let link = archive
        .as_mut()
        .map(|capture| capture.finish(&receipt))
        .transpose();
    write_evidence_bytes_atomic(paths.result, &result_bytes)?;
    write_receipt_atomic(paths.receipt, &receipt)?;
    let link = link?;
    if let (Some(capture), Some(link)) = (archive, link) {
        reject_symlink_ancestors(capture.link_path())?;
        write_evidence_bytes_atomic(capture.link_path(), &link)?;
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::{header, StatusCode, Url, Version};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    const SELECTION: &[u8] = include_bytes!(
        "../../../../tests/fixtures/internet_archive_replay/approved-selection.json"
    );
    const PAYLOAD: &[u8] =
        include_bytes!("../../../../tests/fixtures/internet_archive_replay/payload.json");

    struct Harness {
        _temp: tempfile::TempDir,
        root: PathBuf,
        selection: PathBuf,
        output_dir: PathBuf,
        responses_dir: PathBuf,
        checkpoint: PathBuf,
        result: PathBuf,
        receipt: PathBuf,
    }

    impl Harness {
        fn new() -> Self {
            let parent = fs::canonicalize(std::env::temp_dir()).unwrap();
            let temp = tempfile::tempdir_in(parent).unwrap();
            let root = temp.path().to_path_buf();
            fs::write(root.join("selection.json"), SELECTION).unwrap();
            Self {
                selection: root.join("selection.json"),
                output_dir: root.join("payloads"),
                responses_dir: root.join("responses"),
                checkpoint: root.join("checkpoint.json"),
                result: root.join("result.json"),
                receipt: root.join("receipt.json"),
                _temp: temp,
                root,
            }
        }
        fn config() -> ReplayConfig {
            ReplayConfig {
                allowed_target_host: "example.test".into(),
                max_rows: 5,
                max_payload_bytes: 1024,
                max_redirects: 3,
                max_runtime_seconds: 20.0,
                request_timeout_seconds: 5.0,
            }
        }
        fn paths(&self) -> ReplayEvidencePaths<'_> {
            ReplayEvidencePaths {
                selection: &self.selection,
                output_dir: &self.output_dir,
                responses_dir: &self.responses_dir,
                checkpoint: &self.checkpoint,
                result: &self.result,
                receipt: &self.receipt,
            }
        }
    }

    fn response(url: &str, status: StatusCode, body: &[u8]) -> ReplayResponse {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
        ReplayResponse {
            url: Url::parse(url).unwrap(),
            status,
            version: Version::HTTP_11,
            headers,
            body: body.to_vec(),
            captured_at: time::OffsetDateTime::now_utc(),
        }
    }

    fn fixture_receipt(h: &Harness) -> Value {
        let receipt: Value =
            serde_json::from_slice(&fs::read(h.root.join("receipt.json")).unwrap()).unwrap();
        verify_receipt(&receipt).unwrap();
        verify_receipt_result_bytes(&receipt, &fs::read(h.root.join("result.json")).unwrap())
            .unwrap();
        receipt
    }

    #[tokio::test]
    async fn redirect_hops_and_result_are_bound_to_the_receipt() {
        let h = Harness::new();
        let paths = h.paths();
        let config = Harness::config();
        let archive_paths = ReplayArchivePaths {
            wacz: &h.root.join("redirect.wacz"),
            link: &h.root.join("redirect-link.json"),
        };
        preflight_archive(&paths, &archive_paths).unwrap();
        let selection = prepare(&paths, &config).unwrap();
        let mut capture = ReplayArchive::new(&archive_paths).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let fetch_calls = calls.clone();
        let result = replay_evidenced_with_archive(&paths, &config, &selection, Some(&mut capture), move |url, _, _| {
            let hop = fetch_calls.fetch_add(1, Ordering::SeqCst);
            async move {
                if hop == 0 {
                    let mut redirect = response(&url, StatusCode::FOUND, b"");
                    redirect.headers.insert(header::LOCATION, "https://web.archive.org/web/20250202000000id_/https://example.test/request/1.json".parse().unwrap());
                    Ok(redirect)
                } else {
                    Ok(response(&url, StatusCode::OK, PAYLOAD))
                }
            }
        }).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let receipt = fixture_receipt(&h);
        assert_eq!(receipt["status"], "succeeded");
        assert_eq!(
            receipt["totals"],
            json!({"requests": 2, "bytes": PAYLOAD.len(), "retries": 0})
        );
        assert_eq!(receipt["responses"][0]["status"], 302);
        assert_eq!(receipt["responses"][1]["sha256"], sha256_hex(PAYLOAD));
        assert_eq!(receipt["checkpoint"]["before_sha256"], Value::Null);
        assert_eq!(
            receipt["checkpoint"]["after_sha256"],
            sha256_hex(&fs::read(paths.checkpoint).unwrap())
        );
        assert_eq!(
            fs::read(paths.result).unwrap(),
            result.canonical_bytes().unwrap()
        );
        super::super::verify_replay_archive(&receipt, archive_paths.wacz, archive_paths.link)
            .unwrap();
        assert_eq!(
            fs::read(
                paths
                    .responses_dir
                    .join(format!("000000-{}.raw", sha256_hex(b"")))
            )
            .unwrap(),
            b""
        );
        assert_eq!(
            fs::read(
                paths
                    .responses_dir
                    .join(format!("000001-{}.raw", sha256_hex(PAYLOAD)))
            )
            .unwrap(),
            PAYLOAD
        );
        verify_replay_response_files(&receipt, paths.responses_dir).unwrap();
        let extra = paths.responses_dir.join("unexpected.raw");
        fs::write(&extra, b"extra").unwrap();
        assert!(verify_replay_response_files(&receipt, paths.responses_dir).is_err());
        fs::remove_file(extra).unwrap();
        let second = paths
            .responses_dir
            .join(format!("000001-{}.raw", sha256_hex(PAYLOAD)));
        fs::write(&second, b"tampered").unwrap();
        assert!(verify_replay_response_files(&receipt, paths.responses_dir).is_err());
        #[cfg(unix)]
        {
            fs::remove_file(&second).unwrap();
            let outside = h.root.join("outside.raw");
            fs::write(&outside, PAYLOAD).unwrap();
            std::os::unix::fs::symlink(outside, second).unwrap();
            assert!(verify_replay_response_files(&receipt, paths.responses_dir).is_err());
        }
    }

    #[tokio::test]
    async fn validation_failure_records_observed_bytes_without_claiming_success() {
        let h = Harness::new();
        let paths = h.paths();
        let config = Harness::config();
        let archive_paths = ReplayArchivePaths {
            wacz: &h.root.join("failed.wacz"),
            link: &h.root.join("failed-link.json"),
        };
        preflight_archive(&paths, &archive_paths).unwrap();
        let selection = prepare(&paths, &config).unwrap();
        let mut capture = ReplayArchive::new(&archive_paths).unwrap();
        let result = replay_evidenced_with_archive(
            &paths,
            &config,
            &selection,
            Some(&mut capture),
            |url, _, _| async move { Ok(response(&url, StatusCode::OK, b"incorrect")) },
        )
        .await;
        assert!(result.is_err());
        let receipt = fixture_receipt(&h);
        assert_eq!(receipt["status"], "failed");
        assert_eq!(receipt["failure"]["type"], "ReplayFailed");
        assert_eq!(receipt["totals"]["requests"], 1);
        assert_eq!(receipt["responses"][0]["sha256"], sha256_hex(b"incorrect"));
        assert!(!paths.checkpoint.exists());
        super::super::verify_replay_archive(&receipt, archive_paths.wacz, archive_paths.link)
            .unwrap();
    }

    #[tokio::test]
    async fn compressed_http_bytes_remain_distinct_from_the_verified_payload() {
        use std::io::Write;

        let h = Harness::new();
        let paths = h.paths();
        let config = Harness::config();
        let selection = prepare(&paths, &config).unwrap();
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(PAYLOAD).unwrap();
        let encoded = encoder.finish().unwrap();
        let supplied = encoded.clone();
        let result = replay_evidenced(&paths, &config, &selection, move |url, _, _| {
            let body = supplied.clone();
            async move {
                let mut captured = response(&url, StatusCode::OK, &body);
                captured
                    .headers
                    .insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
                Ok(captured)
            }
        })
        .await
        .unwrap();
        let receipt = fixture_receipt(&h);
        assert_eq!(receipt["responses"][0]["sha256"], sha256_hex(&encoded));
        assert_ne!(sha256_hex(&encoded), sha256_hex(PAYLOAD));
        assert_eq!(
            fs::read(
                paths
                    .responses_dir
                    .join(format!("000000-{}.raw", sha256_hex(&encoded)))
            )
            .unwrap(),
            encoded
        );
        let projection: Value = serde_json::from_slice(&result.canonical_bytes().unwrap()).unwrap();
        assert_eq!(projection["objects"][0]["sha256"], sha256_hex(PAYLOAD));
    }

    #[tokio::test]
    async fn compressed_fixture_links_receipt_to_wacz_and_rejects_tampering() {
        use crate::internet_archive_replay::verify_replay_archive;
        use std::io::Read;

        let h = Harness::new();
        let paths = h.paths();
        let config = Harness::config();
        let archive_paths = ReplayArchivePaths {
            wacz: &h.root.join("replay.wacz"),
            link: &h.root.join("archive-link.json"),
        };
        preflight_archive(&paths, &archive_paths).unwrap();
        let selection = prepare(&paths, &config).unwrap();
        let mut capture = ReplayArchive::new(&archive_paths).unwrap();
        // The exact gzip response from the retained offline Wayback fixture.
        let encoded = data_encoding::BASE64
            .decode(b"H4sIAAAAAAAA/wEyAM3/eyJyZXF1ZXN0Ijoic3ludGhldGljLTAwMSIsInN0YXR1cyI6InN1Y2Nlc3NmdWwifQpWg5kbMgAAAA==")
            .unwrap();
        assert_eq!(
            sha256_hex(&encoded),
            "ddf6ca0e623a8d84c8e6828c9eb3f0a9239c9dd9d25676c5a3e271817f064f20"
        );
        let supplied = encoded.clone();
        replay_evidenced_with_archive(
            &paths,
            &config,
            &selection,
            Some(&mut capture),
            move |url, _, _| {
                let body = supplied.clone();
                async move {
                    let mut captured = response(&url, StatusCode::OK, &body);
                    captured
                        .headers
                        .insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
                    captured
                        .headers
                        .insert(header::SET_COOKIE, "private=secret".parse().unwrap());
                    Ok(captured)
                }
            },
        )
        .await
        .unwrap();

        let receipt = fixture_receipt(&h);
        verify_replay_archive(&receipt, archive_paths.wacz, archive_paths.link).unwrap();
        let file = fs::File::open(archive_paths.wacz).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut warc = Vec::new();
        flate2::read::GzDecoder::new(zip.by_name("archive/data.warc.gz").unwrap())
            .read_to_end(&mut warc)
            .unwrap();
        let headers_end = warc
            .windows(4)
            .position(|part| part == b"\r\n\r\n")
            .unwrap()
            + 4;
        let http_end = warc[headers_end..]
            .windows(4)
            .position(|part| part == b"\r\n\r\n")
            .unwrap()
            + headers_end
            + 4;
        assert_eq!(&warc[http_end..http_end + encoded.len()], encoded);
        assert!(String::from_utf8_lossy(&warc[..http_end]).contains("content-encoding: gzip"));
        assert!(!warc
            .windows(b"private=secret".len())
            .any(|window| window == b"private=secret"));

        let link_bytes = fs::read(archive_paths.link).unwrap();
        let mut link: Value = serde_json::from_slice(&link_bytes).unwrap();
        link["records"][0]["record_id"] = json!("<urn:uuid:00000000-0000-0000-0000-000000000000>");
        let mut unsigned = link.as_object().unwrap().clone();
        unsigned.remove("link_sha256");
        link["link_sha256"] = json!(sha256_hex(
            &canonical_json_bytes(&Value::Object(unsigned)).unwrap()
        ));
        fs::write(archive_paths.link, canonical_json_bytes(&link).unwrap()).unwrap();
        assert!(
            verify_replay_archive(&receipt, archive_paths.wacz, archive_paths.link)
                .unwrap_err()
                .to_string()
                .contains("record mismatch")
        );
        fs::write(archive_paths.link, &link_bytes).unwrap();
        let mut altered_receipt = receipt.clone();
        altered_receipt["responses"][0]["sha256"] = json!("0".repeat(64));
        let mut unsigned = altered_receipt.as_object().unwrap().clone();
        unsigned.remove("receipt_sha256");
        altered_receipt["receipt_sha256"] = json!(sha256_hex(
            &canonical_json_bytes(&Value::Object(unsigned)).unwrap()
        ));
        assert!(
            verify_replay_archive(&altered_receipt, archive_paths.wacz, archive_paths.link)
                .unwrap_err()
                .to_string()
                .contains("receipt mismatch")
        );
        let mut rebound_link: Value = serde_json::from_slice(&link_bytes).unwrap();
        rebound_link["receipt_sha256"] = altered_receipt["receipt_sha256"].clone();
        let mut unsigned = rebound_link.as_object().unwrap().clone();
        unsigned.remove("link_sha256");
        rebound_link["link_sha256"] = json!(sha256_hex(
            &canonical_json_bytes(&Value::Object(unsigned)).unwrap()
        ));
        fs::write(
            archive_paths.link,
            canonical_json_bytes(&rebound_link).unwrap(),
        )
        .unwrap();
        assert!(
            verify_replay_archive(&altered_receipt, archive_paths.wacz, archive_paths.link)
                .unwrap_err()
                .to_string()
                .contains("archive response differs from receipt")
        );
        fs::write(archive_paths.link, &link_bytes).unwrap();
        let mut wacz = fs::read(archive_paths.wacz).unwrap();
        let index = wacz.len() / 2;
        wacz[index] ^= 1;
        fs::write(archive_paths.wacz, wacz).unwrap();
        assert!(verify_replay_archive(&receipt, archive_paths.wacz, archive_paths.link).is_err());
    }

    #[tokio::test]
    async fn archive_paths_reject_collisions_before_replay() {
        let h = Harness::new();
        let paths = h.paths();
        let config = Harness::config();
        let original = fs::read(&h.selection).unwrap();
        let wacz = h.root.join("capture.wacz");
        let archives = [
            ReplayArchivePaths {
                wacz: &wacz,
                link: &h.selection,
            },
            ReplayArchivePaths {
                wacz: &wacz,
                link: &h.responses_dir.join("link.json"),
            },
            ReplayArchivePaths {
                wacz: &wacz,
                link: &wacz,
            },
        ];
        for archive in &archives {
            assert!(replay_archive_with_archive(&paths, &config, archive)
                .await
                .is_err());
            assert_eq!(fs::read(&h.selection).unwrap(), original);
            assert!(!h.responses_dir.exists());
            assert!(!h.receipt.exists());
            assert!(!wacz.exists());
        }
    }

    #[tokio::test]
    async fn concurrent_wacz_creation_preserves_existing_file_and_acquisition_receipt() {
        let h = Harness::new();
        let paths = h.paths();
        let config = Harness::config();
        let wacz = h.root.join("capture.wacz");
        let link = h.root.join("link.json");
        let archive_paths = ReplayArchivePaths {
            wacz: &wacz,
            link: &link,
        };
        preflight_archive(&paths, &archive_paths).unwrap();
        let selection = prepare(&paths, &config).unwrap();
        let mut capture = ReplayArchive::new(&archive_paths).unwrap();
        let concurrent = wacz.clone();
        let error = replay_evidenced_with_archive(
            &paths,
            &config,
            &selection,
            Some(&mut capture),
            move |url, _, _| {
                let concurrent = concurrent.clone();
                async move {
                    fs::write(concurrent, b"existing").unwrap();
                    Ok(response(&url, StatusCode::OK, PAYLOAD))
                }
            },
        )
        .await
        .unwrap_err();
        assert!(format!("{error:#}").contains("publishing replay WACZ"));
        assert_eq!(fs::read(wacz).unwrap(), b"existing");
        assert!(!link.exists());
        assert_eq!(fixture_receipt(&h)["status"], "succeeded");
    }

    #[tokio::test]
    async fn incomplete_exchange_never_produces_a_false_receipt() {
        let h = Harness::new();
        let paths = h.paths();
        let config = Harness::config();
        let selection = prepare(&paths, &config).unwrap();
        let result = replay_evidenced(&paths, &config, &selection, |_, _, _| async move {
            Err(ReplayFetchError::Fatal(anyhow::anyhow!(
                "synthetic failure"
            )))
        })
        .await;
        assert!(format!("{:#}", result.unwrap_err()).contains("no receipt was published"));
        assert!(!paths.receipt.exists());
        assert!(!paths.result.exists());
    }

    #[tokio::test]
    async fn rejected_transport_response_never_becomes_a_zero_request_receipt() {
        let h = Harness::new();
        let paths = h.paths();
        let config = Harness::config();
        let selection = prepare(&paths, &config).unwrap();
        let error = replay_evidenced(&paths, &config, &selection, |_, _, _| async move {
            Ok(response(
                "https://web.archive.org/wrong",
                StatusCode::OK,
                PAYLOAD,
            ))
        })
        .await
        .unwrap_err();
        assert!(format!("{error:#}").contains("no receipt was published"));
        assert!(!paths.receipt.exists());
        assert!(!paths.result.exists());
    }

    #[tokio::test]
    async fn checkpoint_resume_records_zero_requests_and_preserves_lineage() {
        let h = Harness::new();
        let paths = h.paths();
        let config = Harness::config();
        let selection = prepare(&paths, &config).unwrap();
        replay_evidenced(&paths, &config, &selection, |url, _, _| async move {
            Ok(response(&url, StatusCode::OK, PAYLOAD))
        })
        .await
        .unwrap();
        let before = sha256_hex(&fs::read(paths.checkpoint).unwrap());
        let result2 = h.root.join("result-2.json");
        let receipt2 = h.root.join("receipt-2.json");
        let responses2 = h.root.join("responses-2");
        let resumed = ReplayEvidencePaths {
            result: &result2,
            receipt: &receipt2,
            responses_dir: &responses2,
            ..paths
        };
        let selection = prepare(&resumed, &config).unwrap();
        replay_evidenced(&resumed, &config, &selection, |_, _, _| async move {
            panic!("completed checkpoint must not make a request");
            #[allow(unreachable_code)]
            Ok(response("https://web.archive.org/", StatusCode::OK, b""))
        })
        .await
        .unwrap();
        let receipt: Value = serde_json::from_slice(&fs::read(receipt2).unwrap()).unwrap();
        verify_receipt(&receipt).unwrap();
        assert_eq!(receipt["totals"]["requests"], 0);
        assert_eq!(receipt["checkpoint"]["before_sha256"], before);
        assert_eq!(receipt["checkpoint"]["after_sha256"], before);

        let result3 = h.root.join("result-3.json");
        let receipt3 = h.root.join("receipt-3.json");
        let responses3 = h.root.join("responses-3");
        let wacz = h.root.join("empty.wacz");
        let link = h.root.join("empty-link.json");
        let empty = ReplayEvidencePaths {
            result: &result3,
            receipt: &receipt3,
            responses_dir: &responses3,
            ..h.paths()
        };
        let archive_paths = ReplayArchivePaths {
            wacz: &wacz,
            link: &link,
        };
        preflight_archive(&empty, &archive_paths).unwrap();
        let selection = prepare(&empty, &config).unwrap();
        let mut capture = ReplayArchive::new(&archive_paths).unwrap();
        let error = replay_evidenced_with_archive(
            &empty,
            &config,
            &selection,
            Some(&mut capture),
            |_, _, _| async move {
                panic!("completed checkpoint must not make a request");
                #[allow(unreachable_code)]
                Ok(response("https://web.archive.org/", StatusCode::OK, b""))
            },
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("requires at least one response"));
        let zero_receipt: Value = serde_json::from_slice(&fs::read(&receipt3).unwrap()).unwrap();
        verify_receipt_result_bytes(&zero_receipt, &fs::read(result3).unwrap()).unwrap();
        assert_eq!(zero_receipt["totals"]["requests"], 0);
        assert!(!wacz.exists());
        assert!(!link.exists());
        assert_eq!(sha256_hex(&fs::read(paths.checkpoint).unwrap()), before);
    }

    #[test]
    fn preflight_rejects_aliases_existing_outputs_and_unsafe_input() {
        let h = Harness::new();
        let paths = h.paths();
        let config = Harness::config();
        let aliased = ReplayEvidencePaths {
            receipt: paths.selection,
            ..paths
        };
        assert!(prepare(&aliased, &config).is_err());
        assert_eq!(fs::read(aliased.selection).unwrap(), SELECTION);
        let paths = h.paths();
        fs::write(paths.result, b"preserve").unwrap();
        assert!(prepare(&paths, &config).is_err());
        assert_eq!(fs::read(paths.result).unwrap(), b"preserve");
        assert!(!paths.responses_dir.exists());
    }

    #[test]
    fn response_queries_are_redacted_in_receipt_metadata() {
        let value = response(
            "https://web.archive.org/web/a?secret=synthetic",
            StatusCode::OK,
            b"",
        );
        assert_eq!(
            sanitized_response_url(&value).unwrap(),
            "https://web.archive.org/web/a?REDACTED"
        );
    }

    #[test]
    fn receipt_size_is_bounded_before_any_request() {
        let h = Harness::new();
        let mut selection: Value = serde_json::from_slice(SELECTION).unwrap();
        selection["rows"][0]["original"] =
            json!(format!("https://example.test/{}", "a".repeat(500_000)));
        let mut unsigned = selection.as_object().unwrap().clone();
        unsigned.remove("selection_sha256");
        selection["selection_sha256"] = json!(sha256_hex(
            &canonical_json_bytes(&Value::Object(unsigned)).unwrap()
        ));
        fs::write(&h.selection, canonical_json_bytes(&selection).unwrap()).unwrap();
        let mut config = Harness::config();
        config.max_redirects = 5;
        let error = prepare(&h.paths(), &config).unwrap_err();
        assert!(error.to_string().contains("receipt byte limit"));
        assert!(!h.responses_dir.exists());
    }
}
