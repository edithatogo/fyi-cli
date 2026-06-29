"""Tests for faithful archive capture."""

from __future__ import annotations

import gzip
import zipfile
from typing import TYPE_CHECKING

import httpx
import pytest
from warcio.archiveiterator import ArchiveIterator

from fyi_system.archive_capture import CaptureCaps, CapturedResource, capture_request, package_wacz
from fyi_system.cli import build_parser

if TYPE_CHECKING:
    from pathlib import Path


def capture_transport() -> httpx.MockTransport:
    """Build a mocked FYI request with one attachment."""

    def handler(request: httpx.Request) -> httpx.Response:
        path = request.url.path
        if path == "/request/123.json":
            return httpx.Response(
                200,
                json={
                    "id": 123,
                    "url_title": "example_request",
                    "title": "Example request",
                    "authority": "Agency",
                    "attachments": [
                        {
                            "name": "file.txt",
                            "url": "https://fyi.example/attachments/file.txt",
                            "content_type": "text/plain",
                        },
                    ],
                },
                headers={"content-type": "application/json"},
                request=request,
            )
        if path == "/request/example_request":
            return httpx.Response(
                200,
                content=b"<html>Example</html>",
                headers={"content-type": "text/html"},
                request=request,
            )
        if path == "/attachments/file.txt":
            return httpx.Response(
                200,
                content=b"attachment bytes",
                headers={"content-type": "text/plain"},
                request=request,
            )
        return httpx.Response(404, request=request)

    return httpx.MockTransport(handler)


def test_capture_request_writes_warc_wacz_and_derived_store(tmp_path: Path) -> None:
    summary = capture_request(
        request_ref="123",
        base_url="https://fyi.example",
        data_dir=tmp_path / "data",
        dist_dir=tmp_path / "dist",
        transport=capture_transport(),
    )

    warc_path = tmp_path / summary["warc_path"]
    wacz_path = tmp_path / summary["wacz_path"]
    derived_path = tmp_path / summary["derived_path"]

    assert warc_path.exists()
    assert wacz_path.exists()
    assert (derived_path / "request.json").exists()
    assert (derived_path / "page.html").read_bytes() == b"<html>Example</html>"
    assert len(summary["resources"]) == 3

    with gzip.open(warc_path, "rb") as stream:
        records = list(ArchiveIterator(stream))
    assert [record.rec_type for record in records] == ["response", "response", "response"]
    assert all(record.rec_headers.get_header("WARC-Payload-Digest") for record in records)

    with zipfile.ZipFile(wacz_path) as archive:
        names = set(archive.namelist())
    assert "datapackage.json" in names
    assert "indexes/index.cdxj" in names


def test_capture_request_dedupes_attachment_payloads(tmp_path: Path) -> None:
    kwargs = {
        "base_url": "https://fyi.example",
        "data_dir": tmp_path / "data",
        "dist_dir": tmp_path / "dist",
        "transport": capture_transport(),
    }

    capture_request(request_ref="123", **kwargs)
    capture_request(request_ref="123", **kwargs)

    attachment_files = [
        path for path in (tmp_path / "data" / "attachments").rglob("*") if path.is_file()
    ]
    assert len(attachment_files) == 1


def test_package_wacz_appends_warc_segments(tmp_path: Path) -> None:
    wacz_path = tmp_path / "snapshots" / "20260629.wacz"
    first_warc = tmp_path / "first.warc.gz"
    second_warc = tmp_path / "second.warc.gz"
    first_warc.write_bytes(b"first")
    second_warc.write_bytes(b"second")

    package_wacz(
        warc_path=first_warc,
        output_path=wacz_path,
        resources=[
            CapturedResource(
                kind="html",
                url="https://fyi.example/request/one",
                content_type="text/html",
                size=5,
                sha256="a" * 64,
            ),
        ],
    )
    package_wacz(
        warc_path=second_warc,
        output_path=wacz_path,
        resources=[
            CapturedResource(
                kind="json",
                url="https://fyi.example/request/two.json",
                content_type="application/json",
                size=6,
                sha256="b" * 64,
            ),
        ],
    )

    with zipfile.ZipFile(wacz_path) as archive:
        names = set(archive.namelist())
        datapackage = archive.read("datapackage.json")

    assert {"archive/first.warc.gz", "archive/second.warc.gz"} <= names
    assert datapackage.count(b'"url"') == 2


def test_capture_request_respects_max_bytes(tmp_path: Path) -> None:
    with pytest.raises(RuntimeError, match="max_bytes"):
        capture_request(
            request_ref="123",
            base_url="https://fyi.example",
            data_dir=tmp_path / "data",
            dist_dir=tmp_path / "dist",
            caps=CaptureCaps(max_bytes=1),
            transport=capture_transport(),
        )


def test_capture_cli_parses() -> None:
    args = build_parser().parse_args(["capture", "123", "--max-bytes", "10"])

    assert args.cmd == "capture"
    assert args.request_ref == "123"
    assert args.max_bytes == 10
