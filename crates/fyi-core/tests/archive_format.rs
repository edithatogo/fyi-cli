use chrono::{DateTime, TimeZone, Utc};
use data_encoding::BASE32;
use flate2::read::GzDecoder;
use fyi_core::archive_format::{
    verify_wacz, verify_wacz_with_checkpoint, write_wacz, write_wacz_with_checkpoint, ArchivePage,
    ArchivedRecord, HttpResource, WarcWriter,
};
use reqwest::{Url, Version};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Cursor, Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const HTML: &[u8] = include_bytes!("../../../conformance/archive/page.html");
const BINARY: &[u8] = b"%PDF-synthetic\0\xff\r\nentity bytes\n";

fn timestamp() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 14, 5, 6, 7)
        .single()
        .unwrap()
}

fn resource<'a>(
    url: &'a Url,
    payload: &'a [u8],
    headers: &'a [(String, Vec<u8>)],
) -> HttpResource<'a> {
    HttpResource {
        kind: "attachment",
        url,
        status: 200,
        headers,
        payload,
        captured_at: timestamp(),
        http_version: Version::HTTP_2,
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn labelled_digest(bytes: &[u8]) -> String {
    format!("sha256:{}", BASE32.encode(&Sha256::digest(bytes)))
}

fn split_header(bytes: &[u8]) -> (&[u8], &[u8]) {
    let end = bytes
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .unwrap();
    (&bytes[..end], &bytes[end + 4..])
}

fn header_fields(bytes: &[u8]) -> BTreeMap<String, String> {
    std::str::from_utf8(bytes)
        .unwrap()
        .split("\r\n")
        .skip(1)
        .map(|line| {
            let (key, value) = line.split_once(':').unwrap();
            (key.to_ascii_lowercase(), value.trim_start().to_string())
        })
        .collect()
}

fn decoded_member(bytes: &[u8], record: &ArchivedRecord) -> Vec<u8> {
    let start = usize::try_from(record.offset).unwrap();
    let end = usize::try_from(record.offset + record.length).unwrap();
    let mut decoder = GzDecoder::new(&bytes[start..end]);
    let mut decoded = Vec::new();
    decoder.read_to_end(&mut decoded).unwrap();
    decoded
}

fn write_fixture(directory: &Path) -> (std::path::PathBuf, Vec<ArchivedRecord>, Vec<ArchivePage>) {
    let warc_path = directory.join("capture.warc.gz");
    let page_url = Url::parse("https://example.invalid/request/42").unwrap();
    let document_url = Url::parse("https://example.invalid/attachment/7/document.pdf").unwrap();
    let html_headers = vec![("Content-Type".into(), b"text/html; charset=utf-8".to_vec())];
    let pdf_headers = vec![("Content-Type".into(), b"application/pdf".to_vec())];
    let mut writer = WarcWriter::new(File::create(&warc_path).unwrap());
    let mut page_resource = resource(&page_url, HTML, &html_headers);
    page_resource.kind = "html";
    let first = writer.append(&page_resource).unwrap();
    let second = writer
        .append(&resource(&document_url, BINARY, &pdf_headers))
        .unwrap();
    writer.finish().unwrap();
    let page = ArchivePage {
        url: page_url.to_string(),
        ts: timestamp(),
        title: Some("Synthetic request archive".into()),
        id: Some("request-42".into()),
    };
    (warc_path, vec![first, second], vec![page])
}

fn zip_member(archive: &mut ZipArchive<File>, name: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

#[test]
fn warc_response_records_preserve_entity_bytes_digests_and_independent_gzip_ranges() {
    let url = Url::parse("https://example.invalid/attachment/7/document.pdf").unwrap();
    let headers = vec![
        ("Content-Type".into(), b"application/pdf".to_vec()),
        ("Content-Length".into(), b"9000".to_vec()),
        ("Transfer-Encoding".into(), b"chunked".to_vec()),
        ("Connection".into(), b"X-Hop, keep-alive".to_vec()),
        ("X-Hop".into(), b"transient".to_vec()),
        ("ETag".into(), b"\"synthetic-v1\"".to_vec()),
        ("Set-Cookie".into(), b"synthetic-one=value".to_vec()),
        ("sEt-CoOkIe".into(), b"synthetic-two=value".to_vec()),
        ("X-Session".into(), b"synthetic-private-value".to_vec()),
    ];
    let mut writer = WarcWriter::new(Vec::new());
    let first = writer.append(&resource(&url, BINARY, &headers)).unwrap();
    let second = writer.append(&resource(&url, HTML, &[])).unwrap();
    let bytes = writer.finish().unwrap();
    assert_eq!(first.offset, 0);
    assert_eq!(first.length, second.offset);
    assert_eq!(second.offset + second.length, bytes.len() as u64);
    assert_eq!(first.kind, "attachment");
    assert_eq!(first.size, BINARY.len() as u64);
    assert_eq!(first.sha256, sha256(BINARY));
    assert_eq!(first.http_version, "HTTP/2");
    assert_ne!(first.warc_record_id, second.warc_record_id);
    assert!(first.warc_record_id.starts_with("<urn:uuid:"));
    assert!(first.warc_record_id.ends_with('>'));

    for (record, payload) in [(&first, BINARY), (&second, HTML)] {
        let decoded = decoded_member(&bytes, record);
        let (header, rest) = split_header(&decoded);
        assert!(header.starts_with(b"WARC/1.1\r\n"));
        let fields = header_fields(header);
        assert_eq!(fields["warc-type"], "response");
        assert_eq!(
            fields["warc-fyi-response-header-policy"],
            "public-allowlist-v1"
        );
        assert_eq!(fields["warc-target-uri"], url.as_str());
        assert_eq!(fields["warc-date"], "2026-11-14T05:06:07Z");
        assert_eq!(fields["warc-record-id"], record.warc_record_id);
        assert_eq!(fields["content-type"], "application/http;msgtype=response");
        let block_length: usize = fields["content-length"].parse().unwrap();
        let block = &rest[..block_length];
        assert_eq!(&rest[block_length..], b"\r\n\r\n");
        assert_eq!(fields["warc-block-digest"], labelled_digest(block));
        assert_eq!(fields["warc-payload-digest"], labelled_digest(payload));
        let (http, body) = split_header(block);
        assert!(http.starts_with(b"HTTP/1.1 200 OK\r\n"));
        assert_eq!(body, payload);
        assert_eq!(
            header_fields(http)["content-length"],
            payload.len().to_string()
        );
    }
    let decoded = decoded_member(&bytes, &first);
    let (_, block) = split_header(&decoded);
    let (http, _) = split_header(block);
    let http = std::str::from_utf8(http).unwrap().to_ascii_lowercase();
    assert!(!http.contains("transfer-encoding:"));
    assert!(!http.contains("connection:"));
    assert!(!http.contains("x-hop:"));
    assert!(http.contains("etag: \"synthetic-v1\""));
    assert_eq!(http.matches("set-cookie:").count(), 0);
    assert!(!http.contains("synthetic-one=value"));
    assert!(!http.contains("synthetic-two=value"));
    assert!(!http.contains("x-session:"));
    assert!(!http.contains("synthetic-private-value"));
}

#[test]
fn warc_preserves_content_encoded_entity_and_non_success_status() {
    let url = Url::parse("https://example.invalid/encoded").unwrap();
    let headers = vec![("Content-Encoding".into(), b"gzip".to_vec())];
    let mut input = resource(&url, BINARY, &headers);
    input.status = 404;
    input.http_version = Version::HTTP_11;
    let mut writer = WarcWriter::new(Vec::new());
    let record = writer.append(&input).unwrap();
    let bytes = writer.finish().unwrap();
    let decoded = decoded_member(&bytes, &record);
    let (_, block) = split_header(&decoded);
    let (http, _) = split_header(block);
    assert!(http.starts_with(b"HTTP/1.1 404 Not Found\r\n"));
    assert_eq!(header_fields(http)["content-encoding"], "gzip");
    assert_eq!(record.status, 404);
}

#[test]
fn invalid_resource_inputs_fail_before_writing() {
    let ordinary_url = Url::parse("https://example.invalid/request/42").unwrap();
    let bad_urls = [
        "file:///private/synthetic",
        "https://synthetic:secret@example.invalid/",
        "https://example.invalid/page#fragment",
    ];
    let mut writer = WarcWriter::new(Vec::new());
    for text in bad_urls {
        let url = Url::parse(text).unwrap();
        assert!(writer.append(&resource(&url, HTML, &[])).is_err());
    }
    for headers in [
        vec![("X-Test\r\nInjected".into(), b"value".to_vec())],
        vec![("X-Test".into(), b"value\r\nInjected: yes".to_vec())],
        vec![("Content-Type".into(), b"text/html\0".to_vec())],
    ] {
        assert!(writer
            .append(&resource(&ordinary_url, HTML, &headers))
            .is_err());
    }
    let mut invalid_status = resource(&ordinary_url, HTML, &[]);
    invalid_status.status = 999;
    assert!(writer.append(&invalid_status).is_err());
    assert!(writer.finish().unwrap().is_empty());
}

struct FailingWriter {
    remaining: usize,
}

impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::Error::other("synthetic write failure"));
        }
        let count = bytes.len().min(self.remaining);
        self.remaining -= count;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn partial_writer_failure_poisoning_is_visible() {
    let url = Url::parse("https://example.invalid/request/42").unwrap();
    let mut writer = WarcWriter::new(FailingWriter { remaining: 24 });
    assert!(writer.append(&resource(&url, HTML, &[])).is_err());
    assert!(writer.append(&resource(&url, HTML, &[])).is_err());
    assert!(writer.finish().is_err());
}

#[test]
fn wacz_has_complete_stored_resources_cdxj_pages_and_fixity() {
    let directory = tempfile::tempdir().unwrap();
    let (warc, records, pages) = write_fixture(directory.path());
    let output = directory.path().join("capture.wacz");
    let summary = write_wacz(&output, &warc, &records, &pages).unwrap();
    assert_eq!(summary.record_count, 2);
    assert_eq!(summary.bytes, fs::metadata(&output).unwrap().len());
    assert_eq!(summary.sha256, sha256(&fs::read(&output).unwrap()));
    let verified = verify_wacz(&output).unwrap();
    assert_eq!(verified.sha256, summary.sha256);
    assert_eq!(verified.record_count, 2);

    let mut archive = ZipArchive::new(File::open(&output).unwrap()).unwrap();
    assert_eq!(archive.len(), 5);
    let warc_bytes = fs::read(&warc).unwrap();
    assert_eq!(zip_member(&mut archive, "archive/data.warc.gz"), warc_bytes);
    assert_eq!(
        archive
            .by_name("archive/data.warc.gz")
            .unwrap()
            .compression(),
        CompressionMethod::Stored
    );
    let manifest_bytes = zip_member(&mut archive, "datapackage.json");
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(manifest["profile"], "data-package");
    assert_eq!(manifest["wacz_version"], "1.1.1");
    assert_eq!(manifest["resources"].as_array().unwrap().len(), 3);
    for entry in manifest["resources"].as_array().unwrap() {
        let member = zip_member(&mut archive, entry["path"].as_str().unwrap());
        assert_eq!(entry["bytes"], member.len() as u64);
        assert_eq!(entry["hash"], format!("sha256:{}", sha256(&member)));
    }
    let digest: Value =
        serde_json::from_slice(&zip_member(&mut archive, "datapackage-digest.json")).unwrap();
    assert_eq!(digest["path"], "datapackage.json");
    assert_eq!(
        digest["hash"],
        format!("sha256:{}", sha256(&manifest_bytes))
    );

    let index = String::from_utf8(zip_member(&mut archive, "indexes/index.cdxj")).unwrap();
    let mut found = Vec::new();
    for line in index.lines() {
        let mut fields = line.splitn(3, ' ');
        let key = fields.next().unwrap();
        assert!(key.starts_with("invalid,example)/"));
        assert_eq!(fields.next().unwrap(), "20261114050607");
        let entry: Value = serde_json::from_str(fields.next().unwrap()).unwrap();
        let record = records
            .iter()
            .find(|record| record.url == entry["url"])
            .unwrap();
        assert_eq!(entry["filename"], "data.warc.gz");
        assert_eq!(entry["offset"], record.offset.to_string());
        assert_eq!(entry["length"], record.length.to_string());
        assert_eq!(entry["status"], "200");
        assert_eq!(entry["digest"], format!("sha256:{}", record.sha256));
        found.push(record.url.clone());
    }
    assert_eq!(found.len(), 2);
    assert!(index.lines().is_sorted());
    let page_bytes = zip_member(&mut archive, "pages/pages.jsonl");
    let page_text = std::str::from_utf8(&page_bytes).unwrap();
    let page_lines: Vec<Value> = page_text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(page_lines[0]["format"], "json-pages-1.0");
    assert_eq!(page_lines[1]["url"], pages[0].url);
    assert_eq!(page_lines[1]["ts"], "2026-11-14T05:06:07Z");
    assert_eq!(page_lines[1]["title"], "Synthetic request archive");
    if let Some(destination) = std::env::var_os("FYI_ARCHIVE_ORACLE_DIR") {
        let destination = Path::new(&destination);
        fs::create_dir(destination).unwrap();
        fs::copy(&warc, destination.join("capture.warc.gz")).unwrap();
        fs::copy(&output, destination.join("capture.wacz")).unwrap();
        fs::write(
            destination.join("records.json"),
            serde_json::to_vec_pretty(&records).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn package_creation_preserves_existing_outputs_and_sources() {
    let directory = tempfile::tempdir().unwrap();
    let (warc, records, pages) = write_fixture(directory.path());
    let warc_bytes = fs::read(&warc).unwrap();
    let output = directory.path().join("capture.wacz");
    fs::write(&output, b"prior capture").unwrap();
    assert!(write_wacz(&output, &warc, &records, &pages).is_err());
    assert_eq!(fs::read(&output).unwrap(), b"prior capture");
    assert_eq!(fs::read(&warc).unwrap(), warc_bytes);
}

#[test]
fn packaging_rejects_tampered_truncated_and_mismatched_warc_ranges() {
    let directory = tempfile::tempdir().unwrap();
    let (warc, mut records, pages) = write_fixture(directory.path());
    let original = fs::read(&warc).unwrap();
    let output = directory.path().join("capture.wacz");
    records[1].offset += 1;
    assert!(write_wacz(&output, &warc, &records, &pages).is_err());
    assert!(!output.exists());
    records[1].offset -= 1;
    for bytes in [&original[..original.len() - 1], &original[..24]] {
        fs::write(&warc, bytes).unwrap();
        assert!(write_wacz(&output, &warc, &records, &pages).is_err());
        assert!(!output.exists());
    }
    let mut tampered = original;
    tampered[20] ^= 0x80;
    fs::write(&warc, &tampered).unwrap();
    assert!(write_wacz(&output, &warc, &records, &pages).is_err());
    assert!(!output.exists());
}

fn rewrite_member(input: &Path, output: &Path, name: &str, replacement: &[u8]) {
    let mut archive = ZipArchive::new(File::open(input).unwrap()).unwrap();
    let mut writer = ZipWriter::new(File::create(output).unwrap());
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let entry_name = entry.name().unwrap().into_owned();
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        writer.start_file(&entry_name, options).unwrap();
        if entry_name == name {
            writer.write_all(replacement).unwrap();
        } else {
            io::copy(&mut entry, &mut writer).unwrap();
        }
    }
    writer.finish().unwrap();
}

#[test]
fn verification_rejects_wacz_tampering_and_truncation() {
    let directory = tempfile::tempdir().unwrap();
    let (warc, records, pages) = write_fixture(directory.path());
    let output = directory.path().join("capture.wacz");
    write_wacz(&output, &warc, &records, &pages).unwrap();
    let tampered = directory.path().join("tampered.wacz");
    rewrite_member(
        &output,
        &tampered,
        "pages/pages.jsonl",
        b"synthetic tamper\n",
    );
    assert!(verify_wacz(&tampered).is_err());
    rewrite_member(&output, &tampered, "datapackage.json", b"{}\n");
    assert!(verify_wacz(&tampered).is_err());
    let bytes = fs::read(&output).unwrap();
    fs::write(&tampered, &bytes[..bytes.len() - 15]).unwrap();
    assert!(verify_wacz(&tampered).is_err());
}

#[test]
fn unsafe_member_paths_and_unbounded_metadata_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("unsafe.wacz");
    let mut writer = ZipWriter::new(File::create(&output).unwrap());
    writer
        .start_file("../outside", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"synthetic").unwrap();
    for name in [
        "archive/data.warc.gz",
        "indexes/index.cdxj",
        "pages/pages.jsonl",
        "datapackage.json",
    ] {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"synthetic").unwrap();
    }
    writer.finish().unwrap();
    assert!(verify_wacz(&output)
        .unwrap_err()
        .to_string()
        .contains("unsafe"));
    assert!(!directory.path().join("outside").exists());

    let mut writer = ZipWriter::new(File::create(&output).unwrap());
    writer
        .start_file("datapackage.json", SimpleFileOptions::default())
        .unwrap();
    io::copy(&mut io::repeat(b' ').take(17 * 1024 * 1024), &mut writer).unwrap();
    for name in [
        "archive/data.warc.gz",
        "indexes/index.cdxj",
        "pages/pages.jsonl",
        "datapackage-digest.json",
    ] {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"synthetic").unwrap();
    }
    writer.finish().unwrap();
    assert!(verify_wacz(&output)
        .unwrap_err()
        .to_string()
        .contains("exceeds 16 MiB"));
}

#[test]
fn cdxj_uses_canonical_surt_keys_and_rejects_key_injection() {
    let corpus: Value =
        serde_json::from_str(include_str!("../../../conformance/archive/surt-cases.json")).unwrap();
    for case in corpus.as_array().unwrap() {
        let directory = tempfile::tempdir().unwrap();
        let warc = directory.path().join("capture.warc.gz");
        let output = directory.path().join("capture.wacz");
        let url = Url::parse(case["url"].as_str().unwrap()).unwrap();
        let mut writer = WarcWriter::new(File::create(&warc).unwrap());
        let record = writer.append(&resource(&url, HTML, &[])).unwrap();
        writer.finish().unwrap();
        let pages = [ArchivePage {
            url: url.to_string(),
            ts: timestamp(),
            title: None,
            id: None,
        }];
        write_wacz(&output, &warc, &[record], &pages).unwrap();
        let mut archive = ZipArchive::new(File::open(&output).unwrap()).unwrap();
        let index = String::from_utf8(zip_member(&mut archive, "indexes/index.cdxj")).unwrap();
        assert_eq!(index.lines().count(), 1);
        assert_eq!(
            index.split_once(' ').unwrap().0,
            case["key"].as_str().unwrap()
        );
    }
}

#[test]
fn duplicate_zip_member_names_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("duplicate.wacz");
    fs::write(
        &output,
        include_bytes!("../../../conformance/archive/duplicate-members.zip"),
    )
    .unwrap();
    assert!(verify_wacz(&output)
        .unwrap_err()
        .to_string()
        .contains("duplicate"));
}

#[test]
fn zip64_directory_and_stored_member_sizes_remain_readable() {
    let directory = tempfile::tempdir().unwrap();
    let (warc, records, pages) = write_fixture(directory.path());
    let original = directory.path().join("capture.wacz");
    write_wacz(&original, &warc, &records, &pages).unwrap();
    let output = directory.path().join("zip64.wacz");
    let mut reader = ZipArchive::new(File::open(&original).unwrap()).unwrap();
    let mut writer = ZipWriter::new(File::create(&output).unwrap());
    writer.set_raw_zip64_extensible_data_sector(Vec::new().into_boxed_slice());
    for index in 0..reader.len() {
        let mut member = reader.by_index(index).unwrap();
        let name = member.name().unwrap().into_owned();
        writer
            .start_file(name, SimpleFileOptions::default().large_file(true))
            .unwrap();
        io::copy(&mut member, &mut writer).unwrap();
    }
    writer.finish().unwrap();
    assert_eq!(verify_wacz(&output).unwrap().record_count, 2);
}

#[test]
fn digest_tampering_is_rejected_with_valid_gzip_crc_and_matching_payload_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let url = Url::parse("https://example.invalid/request/42").unwrap();
    let mut writer = WarcWriter::new(Vec::new());
    let mut record = writer.append(&resource(&url, HTML, &[])).unwrap();
    let original = writer.finish().unwrap();
    let mut decoded = decoded_member(&original, &record);
    let body_start = decoded
        .windows(HTML.len())
        .position(|part| part == HTML)
        .unwrap();
    decoded[body_start] ^= 1;
    let payload = &decoded[body_start..body_start + HTML.len()];
    record.sha256 = sha256(payload);
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&decoded).unwrap();
    let recompressed = encoder.finish().unwrap();
    record.length = recompressed.len() as u64;
    let warc = directory.path().join("tampered.warc.gz");
    fs::write(&warc, &recompressed).unwrap();
    let output = directory.path().join("capture.wacz");
    let error = write_wacz(&output, &warc, &[record], &[]).unwrap_err();
    assert!(error.to_string().contains("digest mismatch"));
    assert!(!output.exists());
    assert_eq!(fs::read(&warc).unwrap(), recompressed);
}

#[test]
fn pages_require_matching_response_timestamps_and_unique_identifiers() {
    let directory = tempfile::tempdir().unwrap();
    let (warc, records, mut pages) = write_fixture(directory.path());
    let output = directory.path().join("capture.wacz");
    pages[0].ts += chrono::Duration::seconds(1);
    assert!(write_wacz(&output, &warc, &records, &pages).is_err());
    pages[0].ts -= chrono::Duration::seconds(1);
    pages.push(pages[0].clone());
    assert!(write_wacz(&output, &warc, &records, &pages).is_err());
    assert!(!output.exists());
}

#[test]
fn caller_supplied_metadata_must_match_the_source_warc() {
    let directory = tempfile::tempdir().unwrap();
    let (warc, mut records, pages) = write_fixture(directory.path());
    let output = directory.path().join("capture.wacz");
    records[0].sha256 = "0".repeat(64);
    assert!(write_wacz(&output, &warc, &records, &pages).is_err());
    assert!(!output.exists());
    assert!(warc.exists());
}

#[test]
fn manifest_cannot_authorise_an_index_into_unrelated_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let (warc, records, pages) = write_fixture(directory.path());
    let output = directory.path().join("capture.wacz");
    write_wacz(&output, &warc, &records, &pages).unwrap();
    let mut archive = ZipArchive::new(File::open(&output).unwrap()).unwrap();
    let index_bytes = zip_member(&mut archive, "indexes/index.cdxj");
    let index = String::from_utf8(index_bytes).unwrap();
    let corrupt = index.replace("\"offset\":\"0\"", "\"offset\":\"1\"");
    assert_ne!(index, corrupt);
    let manifest_bytes = zip_member(&mut archive, "datapackage.json");
    let mut manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    for entry in manifest["resources"].as_array_mut().unwrap() {
        if entry["path"] == "indexes/index.cdxj" {
            entry["bytes"] = json!(corrupt.len());
            entry["hash"] = json!(format!("sha256:{}", sha256(corrupt.as_bytes())));
        }
    }
    let manifest = serde_json::to_vec(&manifest).unwrap();
    let digest = serde_json::to_vec(
        &json!({"path":"datapackage.json","hash":format!("sha256:{}", sha256(&manifest))}),
    )
    .unwrap();
    let mut replacements = BTreeMap::new();
    replacements.insert("indexes/index.cdxj", corrupt.as_bytes());
    replacements.insert("datapackage.json", manifest.as_slice());
    replacements.insert("datapackage-digest.json", digest.as_slice());
    let tampered = directory.path().join("ranges.wacz");
    let mut writer = ZipWriter::new(File::create(&tampered).unwrap());
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let entry_name = entry.name().unwrap().into_owned();
        writer
            .start_file(&entry_name, SimpleFileOptions::default())
            .unwrap();
        if let Some(bytes) = replacements.get(entry_name.as_str()) {
            writer.write_all(bytes).unwrap();
        } else {
            io::copy(&mut entry, &mut writer).unwrap();
        }
    }
    writer.finish().unwrap();
    assert!(verify_wacz(&tampered).is_err());
}

#[test]
fn cursor_writer_handles_short_writes_without_changing_record_payload() {
    struct ShortWriter(Cursor<Vec<u8>>);
    impl Write for ShortWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.write(&bytes[..bytes.len().min(7)])
        }
        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }
    let url = Url::parse("https://example.invalid/short").unwrap();
    let mut writer = WarcWriter::new(ShortWriter(Cursor::new(Vec::new())));
    let record = writer.append(&resource(&url, BINARY, &[])).unwrap();
    let bytes = writer.finish().unwrap().0.into_inner();
    let decoded = decoded_member(&bytes, &record);
    assert!(decoded.windows(BINARY.len()).any(|part| part == BINARY));
}

#[test]
fn package_checkpoint_cancellation_removes_only_owned_partial_output() {
    let directory = tempfile::tempdir().unwrap();
    let (warc, records, pages) = write_fixture(directory.path());
    let source_bytes = fs::read(&warc).unwrap();
    let prior = directory.path().join("prior.wacz");
    fs::write(&prior, b"prior capture bytes").unwrap();
    let output = directory.path().join("cancelled.wacz");
    let mut calls = 0usize;
    let error = write_wacz_with_checkpoint(&output, &warc, &records, &pages, || {
        calls += 1;
        if fs::metadata(&output).is_ok_and(|metadata| metadata.len() > 0) {
            anyhow::bail!("synthetic packaging cancellation");
        }
        Ok(())
    })
    .unwrap_err();
    assert!(format!("{error:#}").contains("synthetic packaging cancellation"));
    assert!(calls > 1);
    assert!(!output.exists());
    assert_eq!(fs::read(&warc).unwrap(), source_bytes);
    assert_eq!(fs::read(prior).unwrap(), b"prior capture bytes");
}

#[test]
fn verification_checkpoint_cancellation_preserves_the_source_package() {
    let directory = tempfile::tempdir().unwrap();
    let (warc, records, pages) = write_fixture(directory.path());
    let package = directory.path().join("capture.wacz");
    write_wacz(&package, &warc, &records, &pages).unwrap();
    let source_bytes = fs::read(&package).unwrap();
    let mut calls = 0usize;
    let error = verify_wacz_with_checkpoint(&package, || {
        calls += 1;
        if calls == 12 {
            anyhow::bail!("synthetic verification cancellation");
        }
        Ok(())
    })
    .unwrap_err();
    assert!(format!("{error:#}").contains("synthetic verification cancellation"));
    assert_eq!(calls, 12);
    assert_eq!(fs::read(package).unwrap(), source_bytes);
}

#[test]
fn public_wacz_does_not_serialise_synthetic_sensitive_headers() {
    let directory = tempfile::tempdir().unwrap();
    let url = Url::parse("https://example.invalid/request/public-safe").unwrap();
    let headers = vec![
        ("Content-Type".into(), b"text/html".to_vec()),
        ("ETag".into(), b"\"synthetic-tag\"".to_vec()),
        ("Set-Cookie".into(), b"session=fabricated-a".to_vec()),
        ("sEt-CoOkIe".into(), b"session=fabricated-b".to_vec()),
        (
            "WWW-Authenticate".into(),
            b"Bearer realm=\"fabricated\"".to_vec(),
        ),
        ("X-Session-Token".into(), b"fabricated-private".to_vec()),
    ];
    let warc_path = directory.path().join("capture.warc.gz");
    let mut writer = WarcWriter::new(File::create(&warc_path).unwrap());
    let mut response = resource(&url, HTML, &headers);
    response.kind = "html";
    let record = writer.append(&response).unwrap();
    writer.finish().unwrap();

    let page = ArchivePage {
        url: url.to_string(),
        ts: timestamp(),
        title: Some("Synthetic".into()),
        id: None,
    };
    let package_path = directory.path().join("public.wacz");
    write_wacz(
        &package_path,
        &warc_path,
        std::slice::from_ref(&record),
        &[page],
    )
    .unwrap();
    verify_wacz(&package_path).unwrap();

    let mut package = ZipArchive::new(File::open(package_path).unwrap()).unwrap();
    let warc_bytes = zip_member(&mut package, "archive/data.warc.gz");
    let decoded = decoded_member(&warc_bytes, &record);
    let (warc_header, record_bytes) = split_header(&decoded);
    let fields = header_fields(warc_header);
    assert_eq!(
        fields["warc-fyi-response-header-policy"],
        "public-allowlist-v1"
    );
    assert_eq!(
        fields["warc-block-digest"],
        labelled_digest(&record_bytes[..record_bytes.len() - 4])
    );
    let (http, payload) = split_header(&record_bytes[..record_bytes.len() - 4]);
    let text = std::str::from_utf8(http).unwrap().to_ascii_lowercase();
    assert_eq!(payload, HTML);
    assert!(text.contains("content-type: text/html"));
    assert!(text.contains("etag: \"synthetic-tag\""));
    for disallowed in [
        "set-cookie:",
        "www-authenticate:",
        "x-session-token:",
        "fabricated-a",
        "fabricated-b",
        "fabricated-private",
    ] {
        assert!(!text.contains(disallowed), "{disallowed} leaked into WARC");
    }
}
