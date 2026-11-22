//! WACZ publication and offline linkage to a replay acquisition receipt.

use super::evidence::{read_regular_bounded, require_absent, sanitized_response_url};
use super::{absolute_path, reject_symlink_ancestors, ReplayEvidencePaths, ReplayResponse};
use crate::acquisition_receipts::{canonical_json_bytes, integer, verify_receipt};
use crate::archive_format::{
    verify_wacz_records, write_wacz, ArchivedRecord, HttpResource, WarcWriter,
};
use crate::provenance::sha256_hex;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

const LINK_SCHEMA: &str = "urn:fyi-cli:wayback-wacz-link:1";
const MAX_LINK_BYTES: usize = 16 * 1024 * 1024;

/// Explicit fresh output paths for the archive and its receipt linkage.
pub struct ReplayArchivePaths<'a> {
    pub wacz: &'a Path,
    pub link: &'a Path,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchiveLink {
    schema: String,
    receipt_sha256: String,
    wacz_sha256: String,
    wacz_bytes: u64,
    records: Vec<RecordLink>,
    link_sha256: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordLink {
    response_index: usize,
    record_id: String,
    offset: u64,
    length: u64,
}

fn link_digest(link: &ArchiveLink) -> Result<String> {
    let mut value = serde_json::to_value(link)?;
    value
        .as_object_mut()
        .context("invalid archive link")?
        .remove("link_sha256");
    Ok(sha256_hex(&canonical_json_bytes(&value)?))
}

fn check_records(receipt: &Value, records: &[ArchivedRecord]) -> Result<()> {
    verify_receipt(receipt)?;
    ensure!(
        receipt["command"] == "internet-archive-replay"
            && receipt["adapter"]["id"] == super::REPLAY_ADAPTER_ID,
        "receipt is not for Wayback replay"
    );
    let responses = receipt["responses"]
        .as_array()
        .context("receipt has no responses")?;
    ensure!(
        !responses.is_empty(),
        "a replay archive requires at least one response"
    );
    ensure!(
        responses.len() == records.len(),
        "archive response count differs from receipt"
    );
    for (response, record) in responses.iter().zip(records) {
        ensure!(
            record.kind == "wayback-replay"
                && response["url"].as_str() == Some(record.url.as_str())
                && integer(response, "status")? == u64::from(record.status)
                && integer(response, "bytes")? == record.size
                && response["sha256"].as_str() == Some(record.sha256.as_str()),
            "archive response differs from receipt"
        );
    }
    Ok(())
}

/// Verify WACZ fixity, the linkage and every ordered WARC response against the receipt.
pub fn verify_replay_archive(receipt: &Value, wacz: &Path, link_path: &Path) -> Result<()> {
    verify_receipt(receipt)?;
    reject_symlink_ancestors(wacz)?;
    let metadata = fs::symlink_metadata(wacz).context("replay WACZ is missing")?;
    ensure!(
        metadata.file_type().is_file(),
        "replay WACZ must be a regular file"
    );
    let bytes = read_regular_bounded(link_path, MAX_LINK_BYTES, "replay archive link")?;
    let link: ArchiveLink =
        serde_json::from_slice(&bytes).context("invalid replay archive link")?;
    ensure!(
        link.schema == LINK_SCHEMA,
        "unsupported replay archive link"
    );
    ensure!(
        link.link_sha256 == link_digest(&link)?,
        "archive link digest mismatch"
    );
    ensure!(
        receipt["receipt_sha256"].as_str() == Some(link.receipt_sha256.as_str()),
        "archive link receipt mismatch"
    );
    let (summary, records) = verify_wacz_records(wacz)?;
    ensure!(
        summary.bytes == link.wacz_bytes && summary.sha256 == link.wacz_sha256,
        "archive link WACZ mismatch"
    );
    check_records(receipt, &records)?;
    ensure!(
        link.records.len() == records.len(),
        "archive link record count mismatch"
    );
    for (index, (linked, record)) in link.records.iter().zip(&records).enumerate() {
        ensure!(
            linked.response_index == index
                && linked.record_id == record.warc_record_id
                && linked.offset == record.offset
                && linked.length == record.length,
            "archive link record mismatch"
        );
    }
    Ok(())
}

pub(super) fn preflight_archive(
    paths: &ReplayEvidencePaths<'_>,
    archive: &ReplayArchivePaths<'_>,
) -> Result<()> {
    ensure!(
        archive.wacz.extension().is_some_and(|ext| ext == "wacz"),
        "replay archive output must have a .wacz extension"
    );
    let files = [
        paths.selection,
        paths.checkpoint,
        paths.result,
        paths.receipt,
        archive.wacz,
        archive.link,
    ];
    let normalized = files
        .iter()
        .map(|path| {
            reject_symlink_ancestors(path)?;
            absolute_path(path)
        })
        .collect::<Result<Vec<_>>>()?;
    for (relative_index, output) in normalized[4..].iter().enumerate() {
        ensure!(
            output.file_name().is_some(),
            "replay archive output must name a file"
        );
        for (other_index, other) in normalized.iter().enumerate() {
            if relative_index + 4 != other_index {
                ensure!(
                    !output.starts_with(other) && !other.starts_with(output),
                    "replay archive paths must not overlap other evidence paths"
                );
            }
        }
        for directory in [paths.output_dir, paths.responses_dir] {
            let directory = absolute_path(directory)?;
            ensure!(
                !output.starts_with(&directory) && !directory.starts_with(output),
                "replay archive paths must be outside payload and response directories"
            );
        }
    }
    require_absent(archive.wacz, "replay WACZ")?;
    require_absent(archive.link, "replay archive link")?;
    Ok(())
}

pub(super) struct ReplayArchive {
    directory: tempfile::TempDir,
    writer: Option<WarcWriter<File>>,
    records: Vec<ArchivedRecord>,
    wacz: PathBuf,
    link: PathBuf,
}

impl ReplayArchive {
    pub(super) fn new(paths: &ReplayArchivePaths<'_>) -> Result<Self> {
        let parent = paths
            .wacz
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let link_parent = paths
            .link
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        fs::create_dir_all(link_parent)?;
        reject_symlink_ancestors(paths.wacz)?;
        reject_symlink_ancestors(paths.link)?;
        let directory = tempfile::Builder::new()
            .prefix(".fyi-replay-")
            .tempdir_in(parent)?;
        let writer = WarcWriter::new(File::create(directory.path().join("capture.warc.gz"))?);
        Ok(Self {
            directory,
            writer: Some(writer),
            records: Vec::new(),
            wacz: paths.wacz.to_path_buf(),
            link: paths.link.to_path_buf(),
        })
    }

    pub(super) fn append(&mut self, response: &ReplayResponse) -> Result<()> {
        let url = reqwest::Url::parse(&sanitized_response_url(response)?)?;
        let headers = response
            .headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.as_bytes().to_vec()))
            .collect::<Vec<_>>();
        let record = self
            .writer
            .as_mut()
            .context("archive writer is closed")?
            .append(&HttpResource {
                kind: "wayback-replay",
                url: &url,
                status: response.status.as_u16(),
                headers: &headers,
                payload: &response.body,
                captured_at: response.captured_at,
                http_version: response.version,
            })?;
        self.records.push(record);
        Ok(())
    }

    pub(super) fn finish(&mut self, receipt: &Value) -> Result<Vec<u8>> {
        check_records(receipt, &self.records)?;
        self.writer
            .take()
            .context("archive writer is closed")?
            .finish()?
            .sync_all()?;
        let temporary = self.directory.path().join("capture.wacz");
        let summary = write_wacz(
            &temporary,
            &self.directory.path().join("capture.warc.gz"),
            &self.records,
            &[],
        )?;
        let (verified, records) = verify_wacz_records(&temporary)?;
        ensure!(
            summary == verified && records == self.records,
            "replay archive readback mismatch"
        );
        let mut link = ArchiveLink {
            schema: LINK_SCHEMA.to_string(),
            receipt_sha256: receipt["receipt_sha256"]
                .as_str()
                .context("missing receipt digest")?
                .to_string(),
            wacz_sha256: summary.sha256,
            wacz_bytes: summary.bytes,
            records: records
                .iter()
                .enumerate()
                .map(|(index, record)| RecordLink {
                    response_index: index,
                    record_id: record.warc_record_id.clone(),
                    offset: record.offset,
                    length: record.length,
                })
                .collect(),
            link_sha256: String::new(),
        };
        link.link_sha256 = link_digest(&link)?;
        let bytes = canonical_json_bytes(&serde_json::to_value(link)?)?;
        ensure!(
            bytes.len() <= MAX_LINK_BYTES,
            "replay archive link exceeds byte limit"
        );
        reject_symlink_ancestors(&self.wacz)?;
        fs::hard_link(&temporary, &self.wacz)
            .context("publishing replay WACZ without overwrite")?;
        #[cfg(unix)]
        {
            let parent = self
                .wacz
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            File::open(parent)?.sync_all()?;
        }
        Ok(bytes)
    }

    pub(super) fn link_path(&self) -> &Path {
        &self.link
    }
}
