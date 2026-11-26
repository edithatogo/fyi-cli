//! Verify the link between a source acquisition receipt and its archived HTTP response.

use crate::acquisition_receipts::{canonical_json_bytes, integer, verify_receipt};
use crate::archive_format::{verify_wacz_records, ArchivedRecord};
use crate::provenance::sha256_hex;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

const LINK_SCHEMA: &str = "urn:fyi-cli:source-wacz-link:1";
const MAX_LINK_BYTES: u64 = 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceLink {
    schema: String,
    receipt_sha256: String,
    wacz_sha256: String,
    wacz_bytes: u64,
    record_id: String,
    offset: u64,
    length: u64,
    link_sha256: String,
}

fn link_digest(link: &SourceLink) -> Result<String> {
    let mut value = serde_json::to_value(link)?;
    value
        .as_object_mut()
        .context("invalid archive link")?
        .remove("link_sha256");
    Ok(sha256_hex(&canonical_json_bytes(&value)?))
}

fn reject_symlinks(path: &Path) -> Result<()> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    // Inspect before lexical normalization: alias/../file still traverses alias.
    for ancestor in absolute.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => ensure!(
                !metadata.file_type().is_symlink(),
                "archive paths must not contain symlinks"
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn check_record(receipt: &Value, record: &ArchivedRecord) -> Result<()> {
    let command = receipt["command"].as_str().unwrap_or("");
    let adapter = receipt["adapter"]["id"].as_str().unwrap_or("");
    ensure!(
        (command == "fetch-request"
            && ["alaveteli-request-json", "alaveteli-authority-request-json"].contains(&adapter))
            || (command == "sync-pull" && adapter == "alaveteli-sync")
            || (command == "fetch-source-resource" && adapter == "alaveteli-source-resource"),
        "receipt is not an eligible source response"
    );
    let responses = receipt["responses"]
        .as_array()
        .context("receipt has no responses")?;
    ensure!(
        responses.len() == 1,
        "source archive requires exactly one response"
    );
    let response = &responses[0];
    ensure!(
        record.kind == "source-response"
            && response["url"].as_str() == Some(record.url.as_str())
            && integer(response, "status")? == u64::from(record.status)
            && integer(response, "bytes")? == record.size
            && response["sha256"].as_str() == Some(record.sha256.as_str()),
        "archive response differs from receipt"
    );
    Ok(())
}

/// Verify the archive bytes, indexed WARC record and receipt as one offline unit.
pub fn verify_source_archive(receipt: &Value, wacz: &Path, link_path: &Path) -> Result<()> {
    verify_receipt(receipt)?;
    reject_symlinks(wacz)?;
    reject_symlinks(link_path)?;
    ensure!(
        fs::symlink_metadata(wacz)?.file_type().is_file(),
        "WACZ must be a regular file"
    );
    ensure!(
        fs::symlink_metadata(link_path)?.file_type().is_file(),
        "archive link must be a regular file"
    );
    let file = File::open(link_path)?;
    let mut bytes = Vec::new();
    file.take(MAX_LINK_BYTES + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_LINK_BYTES,
        "archive link exceeds byte limit"
    );
    let link: SourceLink = serde_json::from_slice(&bytes).context("invalid source archive link")?;
    ensure!(
        link.schema == LINK_SCHEMA && link.link_sha256 == link_digest(&link)?,
        "source archive link invalid"
    );
    ensure!(
        receipt["receipt_sha256"].as_str() == Some(link.receipt_sha256.as_str()),
        "archive receipt mismatch"
    );
    let (summary, records) = verify_wacz_records(wacz)?;
    ensure!(
        summary.bytes == link.wacz_bytes
            && summary.sha256 == link.wacz_sha256
            && records.len() == 1,
        "source WACZ mismatch"
    );
    let record = &records[0];
    check_record(receipt, record)?;
    ensure!(
        record.warc_record_id == link.record_id
            && record.offset == link.offset
            && record.length == link.length,
        "source archive record mismatch"
    );
    Ok(())
}
