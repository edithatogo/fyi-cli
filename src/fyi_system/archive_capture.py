"""Faithful read-only capture for FYI/Alaveteli request resources."""

from __future__ import annotations

import gzip
import hashlib
import json
import shutil
import time
import zipfile
from dataclasses import asdict, dataclass
from datetime import UTC, datetime
from io import BytesIO
from pathlib import Path
from typing import TYPE_CHECKING, Any
from urllib.parse import urljoin

from warcio.statusandheaders import StatusAndHeaders
from warcio.warcwriter import WARCWriter

from fyi_system.discovery import authority_name, client
from fyi_system.fetch import extract_request_artifacts, normalize_request_payload

if TYPE_CHECKING:
    import httpx


@dataclass(frozen=True)
class CaptureCaps:
    """Hard limits for a capture run."""

    max_bytes: int | None = None
    max_runtime_minutes: float | None = None
    max_disk_gb: float | None = None


DEFAULT_CAPTURE_CAPS = CaptureCaps()


@dataclass(frozen=True)
class CapturedResource:
    """Captured HTTP resource metadata."""

    kind: str
    url: str
    content_type: str
    size: int
    sha256: str
    path: str | None = None
    warc_record_id: str | None = None


def utc_now_compact() -> str:
    """Return compact UTC timestamp for artifact names."""
    return datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")


def sha256_bytes(payload: bytes) -> str:
    """Return SHA-256 hex digest for bytes."""
    return hashlib.sha256(payload).hexdigest()


def ensure_disk_budget(path: Path, max_disk_gb: float | None) -> None:
    """Fail if free disk space is lower than the configured minimum."""
    if max_disk_gb is None:
        return
    path.mkdir(parents=True, exist_ok=True)
    free_bytes = shutil.disk_usage(path).free
    required = int(max_disk_gb * 1024**3)
    if free_bytes < required:
        msg = f"Free disk space {free_bytes} bytes is below required budget {required} bytes"
        raise RuntimeError(msg)


def cap_exceeded(*, started_at: float, bytes_written: int, caps: CaptureCaps) -> str | None:
    """Return first exceeded cap name."""
    if caps.max_bytes is not None and bytes_written > caps.max_bytes:
        return "max_bytes"
    if caps.max_runtime_minutes is not None:
        elapsed_minutes = (time.monotonic() - started_at) / 60
        if elapsed_minutes > caps.max_runtime_minutes:
            return "max_runtime_minutes"
    return None


def response_content_type(response: httpx.Response) -> str:
    """Return response content type without parameters."""
    return response.headers.get("content-type", "application/octet-stream").split(";")[0]


def write_attachment_payload(attachments_dir: Path, payload: bytes) -> Path:
    """Write content-addressed attachment bytes and return the path."""
    digest = sha256_bytes(payload)
    output_path = attachments_dir / digest[:8] / digest
    output_path.parent.mkdir(parents=True, exist_ok=True)
    if not output_path.exists():
        output_path.write_bytes(payload)
    return output_path


def warc_http_headers(response: httpx.Response) -> StatusAndHeaders:
    """Build WARC HTTP response headers."""
    headers = [(key, value) for key, value in response.headers.items()]
    return StatusAndHeaders(
        f"{response.status_code} {response.reason_phrase}",
        headers,
        protocol="HTTP/1.1",
    )


def write_warc_record(
    *,
    writer: WARCWriter,
    url: str,
    payload: bytes,
    response: httpx.Response,
    digest: str,
) -> str:
    """Write one response record and return its WARC record id."""
    record = writer.create_warc_record(
        url,
        "response",
        payload=BytesIO(payload),
        http_headers=warc_http_headers(response),
        warc_headers_dict={"WARC-Payload-Digest": f"sha256:{digest}"},
    )
    writer.write_record(record)
    return str(record.rec_headers.get_header("WARC-Record-ID"))


def package_wacz(*, warc_path: Path, output_path: Path, resources: list[CapturedResource]) -> None:
    """Create or extend a minimal WACZ-style package containing WARC segments."""
    output_path.parent.mkdir(parents=True, exist_ok=True)
    existing_resources: list[dict[str, Any]] = []
    existing_entries: dict[str, bytes] = {}
    if output_path.exists():
        with zipfile.ZipFile(output_path) as existing:
            for name in existing.namelist():
                if name == "datapackage.json":
                    payload = json.loads(existing.read(name))
                    existing_resources = list(payload.get("resources") or [])
                elif name != "indexes/index.cdxj":
                    existing_entries[name] = existing.read(name)

    archive_name = f"archive/{warc_path.name}"
    existing_entries[archive_name] = warc_path.read_bytes()
    datapackage = {
        "profile": "data-package",
        "name": "fyi-request-capture",
        "resources": [*existing_resources, *[asdict(resource) for resource in resources]],
    }
    tmp_path = output_path.with_suffix(f"{output_path.suffix}.tmp")
    with zipfile.ZipFile(tmp_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, payload in sorted(existing_entries.items()):
            archive.writestr(name, payload)
        archive.writestr("datapackage.json", json.dumps(datapackage, indent=2, sort_keys=True))
        archive.writestr("indexes/index.cdxj", "")
    tmp_path.replace(output_path)


def write_derived_store(
    *,
    derived_dir: Path,
    authority: str,
    request_id: int,
    request_json: dict[str, Any],
    html: bytes,
    resources: list[CapturedResource],
) -> Path:
    """Write derived request view for dataset/export consumers."""
    output_dir = derived_dir / (authority or "unknown") / str(request_id)
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "request.json").write_text(
        json.dumps(request_json, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    (output_dir / "page.html").write_bytes(html)
    (output_dir / "attachments.json").write_text(
        json.dumps(
            [asdict(resource) for resource in resources if resource.kind == "attachment"],
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    (output_dir / "snapshot_meta.json").write_text(
        json.dumps(
            {"resources": [asdict(resource) for resource in resources]},
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    return output_dir


def capture_request(
    *,
    request_ref: str,
    base_url: str = "https://fyi.org.nz",
    data_dir: Path = Path("data"),
    dist_dir: Path = Path("dist"),
    caps: CaptureCaps | None = None,
    transport: httpx.BaseTransport | None = None,
) -> dict[str, Any]:
    """Capture request JSON, rendered HTML, attachments, WARC, WACZ, and derived view."""
    caps = caps or DEFAULT_CAPTURE_CAPS
    ensure_disk_budget(data_dir, caps.max_disk_gb)
    started_at = time.monotonic()
    run_id = utc_now_compact()
    warc_path = data_dir / "warc" / f"{run_id}-{request_ref}.warc.gz"
    wacz_path = dist_dir / "site_snapshots" / f"{run_id[:8]}.wacz"
    attachments_dir = data_dir / "attachments"
    derived_dir = data_dir / "raw" / "requests"
    resources: list[CapturedResource] = []
    bytes_written = 0

    with client(base_url, transport=transport) as http:
        json_response = http.get(f"/request/{request_ref}.json")
        if json_response.status_code == 404:
            msg = f"Request {request_ref} was not found"
            raise FileNotFoundError(msg)
        json_response.raise_for_status()
        request_json = json_response.json()
        request = normalize_request_payload(request_json)
        request_id = int(request["id"] or request_ref)
        url_title = str(request["url_title"] or request_ref)
        authority = authority_name(request_json.get("authority") or request_json.get("public_body"))
        if not authority:
            authority = "unknown"

        html_response = http.get(f"/request/{url_title}")
        html_response.raise_for_status()

        warc_path.parent.mkdir(parents=True, exist_ok=True)
        with gzip.open(warc_path, "wb") as stream:
            writer = WARCWriter(stream, gzip=False)
            for kind, response, payload in (
                ("json", json_response, json_response.content),
                ("html", html_response, html_response.content),
            ):
                digest = sha256_bytes(payload)
                record_id = write_warc_record(
                    writer=writer,
                    url=str(response.url),
                    payload=payload,
                    response=response,
                    digest=digest,
                )
                resources.append(
                    CapturedResource(
                        kind=kind,
                        url=str(response.url),
                        content_type=response_content_type(response),
                        size=len(payload),
                        sha256=digest,
                        warc_record_id=record_id,
                    ),
                )
                bytes_written += len(payload)

            for attachment in extract_request_artifacts(request_json)["attachments"]:
                attachment_url = urljoin(base_url.rstrip("/") + "/", str(attachment["url"]))
                response = http.get(attachment_url)
                if response.status_code == 404:
                    continue
                response.raise_for_status()
                payload = response.content
                digest = sha256_bytes(payload)
                attachment_path = write_attachment_payload(attachments_dir, payload)
                record_id = write_warc_record(
                    writer=writer,
                    url=str(response.url),
                    payload=payload,
                    response=response,
                    digest=digest,
                )
                resources.append(
                    CapturedResource(
                        kind="attachment",
                        url=str(response.url),
                        content_type=response_content_type(response),
                        size=len(payload),
                        sha256=digest,
                        path=attachment_path.as_posix(),
                        warc_record_id=record_id,
                    ),
                )
                bytes_written += len(payload)
                exceeded = cap_exceeded(
                    started_at=started_at,
                    bytes_written=bytes_written,
                    caps=caps,
                )
                if exceeded is not None:
                    msg = f"Capture aborted because {exceeded} was exceeded"
                    raise RuntimeError(msg)

    derived_path = write_derived_store(
        derived_dir=derived_dir,
        authority=authority,
        request_id=request_id,
        request_json=request_json,
        html=html_response.content,
        resources=resources,
    )
    package_wacz(warc_path=warc_path, output_path=wacz_path, resources=resources)
    return {
        "request_id": request_id,
        "url_title": url_title,
        "warc_path": warc_path.as_posix(),
        "wacz_path": wacz_path.as_posix(),
        "derived_path": derived_path.as_posix(),
        "resources": [asdict(resource) for resource in resources],
        "bytes_written": bytes_written,
    }
