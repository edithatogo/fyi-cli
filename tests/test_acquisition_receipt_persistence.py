"""Offline regressions for immutable receipt publication and filesystem failures."""

from __future__ import annotations

import errno
import json
import os
import stat
from concurrent.futures import ThreadPoolExecutor
from threading import Barrier
from typing import TYPE_CHECKING, Any

import pytest

from fyi_system.acquisition_receipts import (
    AcquisitionRecorder,
    canonical_json_bytes,
    validate_receipt,
    write_receipt_atomic,
)

if TYPE_CHECKING:
    from pathlib import Path


def _receipt(command: str = "discover") -> dict[str, Any]:
    return AcquisitionRecorder(
        command=command,
        adapter_id="alaveteli-search-feed",
        adapter_version="test",
        source_url="https://example.test/search",
        request_bounds={"max_pages": 1},
        started_at="2026-08-13T00:00:00Z",
    ).build(
        result_projection=b"[]\n",
        result_media_type="application/json",
        completed_at="2026-08-13T00:00:01Z",
    )


def _assert_clean(directory: Path) -> None:
    assert not list(directory.glob(".receipt.json.*.tmp"))


@pytest.mark.parametrize("existing", [b"not-json", b"{}", b"null", b"[]", b"\xff"])
def test_corrupt_existing_evidence_is_preserved(tmp_path: Path, existing: bytes) -> None:
    destination = tmp_path / "receipt.json"
    destination.write_bytes(existing)

    with pytest.raises(FileExistsError, match="existing acquisition receipt is invalid"):
        write_receipt_atomic(destination, _receipt())

    assert destination.read_bytes() == existing
    _assert_clean(tmp_path)


def test_failure_receipt_cannot_replace_successful_evidence(tmp_path: Path) -> None:
    destination = tmp_path / "receipt.json"
    original = _receipt()
    write_receipt_atomic(destination, original)
    recorder = AcquisitionRecorder(
        command="discover",
        adapter_id="alaveteli-search-feed",
        source_url="https://example.test/search",
        request_bounds={"max_pages": 1},
    )

    with pytest.raises(FileExistsError, match="different acquisition receipt"):
        recorder.write_failure(destination, failure_type="TimeoutError")

    assert destination.read_bytes() == canonical_json_bytes(original)
    _assert_clean(tmp_path)


def test_dangling_symlink_is_not_replaced(tmp_path: Path) -> None:
    destination = tmp_path / "receipt.json"
    missing_target = tmp_path / "missing.json"
    destination.symlink_to(missing_target)

    with pytest.raises(FileExistsError, match="not a regular file"):
        write_receipt_atomic(destination, _receipt())

    assert destination.is_symlink()
    assert not missing_target.exists()
    _assert_clean(tmp_path)


def test_directory_destination_is_not_replaced(tmp_path: Path) -> None:
    destination = tmp_path / "receipt.json"
    destination.mkdir()

    with pytest.raises(FileExistsError, match="not a regular file"):
        write_receipt_atomic(destination, _receipt())

    assert destination.is_dir()
    _assert_clean(tmp_path)


@pytest.mark.parametrize("identical", [True, False])
def test_concurrent_publication_is_idempotent_or_has_one_winner(
    tmp_path: Path,
    *,
    identical: bool,
) -> None:
    destination = tmp_path / "receipt.json"
    first = _receipt()
    second = first if identical else _receipt("capture")
    barrier = Barrier(2)

    def publish(receipt: dict[str, Any]) -> bool:
        barrier.wait(timeout=10)
        try:
            write_receipt_atomic(destination, receipt)
        except FileExistsError:
            return False
        return True

    with ThreadPoolExecutor(max_workers=2) as pool:
        futures = [pool.submit(publish, receipt) for receipt in (first, second)]
        outcomes = [future.result(timeout=15) for future in futures]

    assert sum(outcomes) == (2 if identical else 1)
    stored = destination.read_bytes()
    assert stored in (canonical_json_bytes(first), canonical_json_bytes(second))
    validate_receipt(json.loads(stored))
    _assert_clean(tmp_path)


@pytest.mark.parametrize("error_number", [errno.EXDEV, errno.EPERM])
def test_unsupported_hard_link_fails_without_overwrite_fallback(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    error_number: int,
) -> None:
    destination = tmp_path / "receipt.json"

    def fail_link(_source: Path, _destination: Path) -> None:
        message = "injected hard-link failure"
        raise OSError(error_number, message)

    monkeypatch.setattr("fyi_system.acquisition_receipts.os.link", fail_link)
    with pytest.raises(OSError, match="injected hard-link failure") as failure:
        write_receipt_atomic(destination, _receipt())

    assert failure.value.errno == error_number
    assert not destination.exists()
    _assert_clean(tmp_path)


def test_file_flush_failure_does_not_publish_partial_receipt(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    destination = tmp_path / "receipt.json"

    def fail_flush(_descriptor: int) -> None:
        message = "injected file flush failure"
        raise OSError(errno.EIO, message)

    monkeypatch.setattr("fyi_system.acquisition_receipts.os.fsync", fail_flush)
    with pytest.raises(OSError, match="injected file flush failure"):
        write_receipt_atomic(destination, _receipt())

    assert not destination.exists()
    _assert_clean(tmp_path)


@pytest.mark.skipif(os.name != "posix", reason="directory fsync is POSIX-only")
def test_directory_flush_failure_preserves_complete_published_evidence(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    destination = tmp_path / "receipt.json"
    receipt = _receipt()
    real_fsync = os.fsync

    def fail_directory_flush(descriptor: int) -> None:
        if stat.S_ISDIR(os.fstat(descriptor).st_mode):
            message = "injected directory flush failure"
            raise OSError(errno.EIO, message)
        real_fsync(descriptor)

    with monkeypatch.context() as patch:
        patch.setattr("fyi_system.acquisition_receipts.os.fsync", fail_directory_flush)
        with pytest.raises(OSError, match="injected directory flush failure"):
            write_receipt_atomic(destination, receipt)

    assert destination.read_bytes() == canonical_json_bytes(receipt)
    validate_receipt(json.loads(destination.read_bytes()))
    write_receipt_atomic(destination, receipt)
    assert destination.read_bytes() == canonical_json_bytes(receipt)
    _assert_clean(tmp_path)


@pytest.mark.skipif(os.name != "posix", reason="POSIX permission bits")
def test_new_receipt_is_private(tmp_path: Path) -> None:
    destination = tmp_path / "receipt.json"
    write_receipt_atomic(destination, _receipt())
    assert stat.S_IMODE(destination.stat().st_mode) == 0o600
    _assert_clean(tmp_path)
