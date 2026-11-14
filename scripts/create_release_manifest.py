#!/usr/bin/env python3
"""Create a deterministic release manifest and SHA-256 inventory."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import sys
import tempfile
from contextlib import contextmanager
from pathlib import Path
from typing import TYPE_CHECKING, TypedDict

if TYPE_CHECKING:
    from collections.abc import Iterator
    from typing import BinaryIO

MANIFEST_NAME = "release-manifest.json"
SUMS_NAME = "SHA256SUMS"
GENERATED_NAMES = frozenset({MANIFEST_NAME, SUMS_NAME})
MAX_ASSETS = 4096
MAX_EVIDENCE_BYTES = 4 * 1024 * 1024
HASH_CHUNK_BYTES = 1024 * 1024
SEMVER = re.compile(
    r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)"
    r"(?:-(?P<pre>[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?",
)
SAFE_NAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9._+-]{0,254}")
WINDOWS_DEVICES = frozenset(
    {"con", "prn", "aux", "nul"}
    | {f"com{number}" for number in range(1, 10)}
    | {f"lpt{number}" for number in range(1, 10)},
)


class AssetRecord(TypedDict):
    """One complete regular release file."""

    name: str
    sha256: str
    size: int


class ReleaseManifest(TypedDict):
    """Deterministic byte inventory, without a provenance assertion."""

    schema_version: int
    version: str
    commit: str
    files: list[AssetRecord]


def _validate_identity(version: str, commit: str) -> None:
    match = SEMVER.fullmatch(version) if isinstance(version, str) else None
    if match is None or any(
        part.isdigit() and len(part) > 1 and part.startswith("0")
        for part in (match.group("pre") or "").split(".")
    ):
        message = "version must be a SemVer value without a leading v"
        raise ValueError(message)
    if not isinstance(commit, str) or re.fullmatch(r"[0-9A-Fa-f]{40}", commit) is None:
        message = "commit must be a full 40-character hexadecimal Git object ID"
        raise ValueError(message)


def _validate_name(name: str) -> None:
    aliases = {generated.casefold() for generated in GENERATED_NAMES}
    if (
        SAFE_NAME.fullmatch(name) is None
        or name.endswith(".")
        or name.split(".", 1)[0].casefold() in WINDOWS_DEVICES
        or (name.casefold() in aliases and name not in GENERATED_NAMES)
    ):
        message = f"unsafe or nonportable asset name: {name!r}"
        raise ValueError(message)


def _fingerprint(metadata: os.stat_result) -> tuple[int, ...]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_ctime_ns,
        metadata.st_mode,
    )


@contextmanager
def _open_asset(path: Path) -> Iterator[BinaryIO]:
    before = path.lstat()
    if not stat.S_ISREG(before.st_mode):
        message = f"asset must be a regular file, not a symlink or directory: {path.name}"
        raise ValueError(message)
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0)
    flags |= getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    descriptor = os.open(path, flags)
    with os.fdopen(descriptor, "rb") as stream:
        if _fingerprint(before) != _fingerprint(os.fstat(stream.fileno())):
            message = f"asset changed before reading: {path.name}"
            raise ValueError(message)
        yield stream
        if _fingerprint(before) != _fingerprint(os.fstat(stream.fileno())) or _fingerprint(
            before,
        ) != _fingerprint(path.lstat()):
            message = f"asset changed while reading: {path.name}"
            raise ValueError(message)


def _inventory(assets: Path) -> list[Path]:
    if not stat.S_ISDIR(assets.lstat().st_mode):
        message = "assets must be a real directory, not a symlink"
        raise ValueError(message)
    paths: list[Path] = []
    names: set[str] = set()
    for path in assets.iterdir():
        _validate_name(path.name)
        folded = path.name.casefold()
        if folded in names:
            message = f"case-insensitive asset name collision: {path.name}"
            raise ValueError(message)
        names.add(folded)
        if not stat.S_ISREG(path.lstat().st_mode):
            message = f"asset must be a regular file, not a symlink or directory: {path.name}"
            raise ValueError(message)
        if path.name not in GENERATED_NAMES:
            paths.append(path)
            if len(paths) > MAX_ASSETS:
                message = f"release directory exceeds {MAX_ASSETS} payload files"
                raise ValueError(message)
    if not paths:
        message = "release directory contains no payload files"
        raise ValueError(message)
    return sorted(paths)


def create_manifest(assets: Path, version: str, commit: str) -> ReleaseManifest:
    """Inventory payload bytes without writing or authenticating their origin."""
    _validate_identity(version, commit)
    files: list[AssetRecord] = []
    for path in _inventory(assets):
        digest = hashlib.sha256()
        size = 0
        with _open_asset(path) as stream:
            while chunk := stream.read(HASH_CHUNK_BYTES):
                digest.update(chunk)
                size += len(chunk)
        files.append({"name": path.name, "sha256": digest.hexdigest(), "size": size})
    return {
        "schema_version": 1,
        "version": version,
        "commit": commit.lower(),
        "files": files,
    }


def _manifest_bytes(manifest: ReleaseManifest) -> bytes:
    return (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode("utf-8")


def _checksums(manifest: ReleaseManifest, manifest_bytes: bytes) -> bytes:
    hashes = {entry["name"]: entry["sha256"] for entry in manifest["files"]}
    hashes[MANIFEST_NAME] = hashlib.sha256(manifest_bytes).hexdigest()
    return "".join(f"{hashes[name]}  {name}\n" for name in sorted(hashes)).encode("ascii")


def _replace_evidence(path: Path, content: bytes) -> None:
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            dir=path.parent,
            prefix=".release-",
            delete=False,
        ) as stream:
            temporary = Path(stream.name)
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        temporary.replace(path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def write_manifest(assets: Path, version: str, commit: str) -> ReleaseManifest:
    """Generate evidence in an exclusively owned staging directory; never publish."""
    manifest = create_manifest(assets, version, commit)
    encoded = _manifest_bytes(manifest)
    _replace_evidence(assets / MANIFEST_NAME, encoded)
    _replace_evidence(assets / SUMS_NAME, _checksums(manifest, encoded))
    return manifest


def _read_evidence(path: Path) -> bytes:
    with _open_asset(path) as stream:
        data = stream.read(MAX_EVIDENCE_BYTES + 1)
    if len(data) > MAX_EVIDENCE_BYTES:
        message = f"integrity file exceeds {MAX_EVIDENCE_BYTES} bytes: {path.name}"
        raise ValueError(message)
    return data


def verify_manifest(assets: Path, expected_version: str, expected_commit: str) -> list[str]:
    """Check exact local inventory and evidence; return errors without changing files."""
    try:
        expected = create_manifest(assets, expected_version, expected_commit)
        manifest_bytes = _read_evidence(assets / MANIFEST_NAME)
        checksum_bytes = _read_evidence(assets / SUMS_NAME)
    except (OSError, ValueError) as error:
        return [str(error)]
    encoded = _manifest_bytes(expected)
    errors = []
    if manifest_bytes != encoded:
        errors.append(
            "release-manifest.json does not match the expected identity and payload bytes",
        )
    if checksum_bytes != _checksums(expected, encoded):
        errors.append("SHA256SUMS does not exactly bind all payloads and release-manifest.json")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("assets", type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument(
        "--verify",
        action="store_true",
        help="Verify local evidence without writing",
    )
    args = parser.parse_args()
    if args.verify:
        errors = verify_manifest(args.assets, args.version, args.commit)
        for error in errors:
            sys.stderr.write(f"error: {error}\n")
        return 1 if errors else 0
    try:
        write_manifest(args.assets, args.version, args.commit)
    except (OSError, ValueError) as error:
        parser.error(str(error))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
