import hashlib
import json
from pathlib import Path

import pytest

from scripts import create_release_manifest as release_manifest

COMMIT = "a" * 40
VERSION = "0.1.2"


def payloads(assets: Path) -> None:
    (assets / "z.bin").write_bytes(b"z")
    (assets / "a.bin").write_bytes(b"a")


def test_release_manifest_is_sorted_and_content_addressed(tmp_path: Path):
    payloads(tmp_path)
    manifest = release_manifest.create_manifest(tmp_path, VERSION, COMMIT)
    assert [item["name"] for item in manifest["files"]] == ["a.bin", "z.bin"]
    assert manifest["files"][0]["sha256"] == hashlib.sha256(b"a").hexdigest()
    assert manifest["files"][0]["size"] == 1
    assert manifest["version"] == VERSION
    assert manifest["commit"] == COMMIT
    assert json.dumps(manifest, sort_keys=True)


def test_release_manifest_hashes_payloads_without_read_bytes(tmp_path: Path, monkeypatch):
    payloads(tmp_path)

    def forbidden_read_bytes(_self):
        pytest.fail("payload hashing must stream rather than read the complete file")

    monkeypatch.setattr(Path, "read_bytes", forbidden_read_bytes)
    assert release_manifest.create_manifest(tmp_path, VERSION, COMMIT)["files"]


def test_release_manifest_rejects_missing_assets_directory(tmp_path: Path):
    with pytest.raises(FileNotFoundError):
        release_manifest.create_manifest(tmp_path / "missing", VERSION, COMMIT)


def test_release_manifest_rejects_empty_assets_directory(tmp_path: Path):
    with pytest.raises(ValueError, match="payload"):
        release_manifest.create_manifest(tmp_path, VERSION, COMMIT)


@pytest.mark.parametrize(
    "version",
    ["v0.1.2", "0.1", "01.2.3", "1.2.3-01", "1.2.3\n", "", "latest"],
)
def test_release_manifest_rejects_invalid_version(tmp_path: Path, version: str):
    payloads(tmp_path)
    with pytest.raises(ValueError, match="SemVer"):
        release_manifest.create_manifest(tmp_path, version, COMMIT)


@pytest.mark.parametrize("version", ["0.1.2", "1.2.3-rc.1+build.42", "2.0.0+sha.a12"])
def test_release_manifest_accepts_semver(tmp_path: Path, version: str):
    payloads(tmp_path)
    assert release_manifest.create_manifest(tmp_path, version, COMMIT)["version"] == version


@pytest.mark.parametrize("commit", ["abc123", "g" * 40, "a" * 39, "a" * 41, "a" * 40 + "\n"])
def test_release_manifest_rejects_invalid_commit(tmp_path: Path, commit: str):
    payloads(tmp_path)
    with pytest.raises(ValueError, match=r"40.*hex"):
        release_manifest.create_manifest(tmp_path, VERSION, commit)


@pytest.mark.parametrize(
    "name",
    [
        "bad name.bin",
        "-option.bin",
        "CON.zip",
        "nul",
        "payload.",
        "sha256sums",
        "Release-Manifest.json",
    ],
)
def test_release_manifest_rejects_nonportable_names(tmp_path: Path, name: str):
    try:
        (tmp_path / name).write_bytes(b"data")
    except OSError:
        pytest.skip("filesystem already rejects this unsafe filename")
    with pytest.raises(ValueError, match="name"):
        release_manifest.create_manifest(tmp_path, VERSION, COMMIT)


def test_release_manifest_rejects_subdirectories(tmp_path: Path):
    payloads(tmp_path)
    (tmp_path / "nested").mkdir()
    with pytest.raises(ValueError, match="regular file"):
        release_manifest.create_manifest(tmp_path, VERSION, COMMIT)


def test_release_manifest_rejects_case_collisions(tmp_path: Path):
    (tmp_path / "Asset.bin").write_bytes(b"one")
    (tmp_path / "asset.bin").write_bytes(b"two")
    if len(list(tmp_path.iterdir())) < 2:
        pytest.skip("case-insensitive filesystem cannot represent this fixture")
    with pytest.raises(ValueError, match="case"):
        release_manifest.create_manifest(tmp_path, VERSION, COMMIT)


@pytest.mark.parametrize("link_name", ["linked.bin", "SHA256SUMS", "release-manifest.json"])
def test_release_manifest_rejects_symlinks(tmp_path: Path, link_name: str):
    payloads(tmp_path)
    try:
        (tmp_path / link_name).symlink_to(tmp_path / "a.bin")
    except OSError:
        pytest.skip("symlink creation unavailable")
    with pytest.raises(ValueError, match="regular file"):
        release_manifest.create_manifest(tmp_path, VERSION, COMMIT)


def test_release_manifest_rejects_symlink_directory(tmp_path: Path):
    actual = tmp_path / "actual"
    actual.mkdir()
    payloads(actual)
    alias = tmp_path / "alias"
    try:
        alias.symlink_to(actual, target_is_directory=True)
    except OSError:
        pytest.skip("symlink creation unavailable")
    with pytest.raises(ValueError, match="directory"):
        release_manifest.create_manifest(alias, VERSION, COMMIT)


def test_generation_binds_manifest_and_is_idempotent(tmp_path: Path):
    payloads(tmp_path)
    manifest = release_manifest.write_manifest(tmp_path, VERSION, COMMIT)
    original = {path.name: path.read_bytes() for path in tmp_path.iterdir()}
    assert [item["name"] for item in manifest["files"]] == ["a.bin", "z.bin"]
    expected_names = ["a.bin", "release-manifest.json", "z.bin"]
    expected_sums = "".join(
        f"{hashlib.sha256(original[name]).hexdigest()}  {name}\n" for name in expected_names
    )
    assert original["SHA256SUMS"].decode() == expected_sums
    assert release_manifest.verify_manifest(tmp_path, VERSION, COMMIT) == []
    release_manifest.write_manifest(tmp_path, VERSION, COMMIT)
    assert original == {path.name: path.read_bytes() for path in tmp_path.iterdir()}


def test_asset_count_limit_excludes_generated_evidence(tmp_path: Path, monkeypatch):
    payloads(tmp_path)
    monkeypatch.setattr(release_manifest, "MAX_ASSETS", 2)
    release_manifest.write_manifest(tmp_path, VERSION, COMMIT)
    assert release_manifest.verify_manifest(tmp_path, VERSION, COMMIT) == []
    (tmp_path / "extra.bin").write_bytes(b"extra")
    assert release_manifest.verify_manifest(tmp_path, VERSION, COMMIT)


def test_verification_rejects_unsafe_extra_directory(tmp_path: Path):
    payloads(tmp_path)
    release_manifest.write_manifest(tmp_path, VERSION, COMMIT)
    (tmp_path / "extra").mkdir()
    assert release_manifest.verify_manifest(tmp_path, VERSION, COMMIT)


def test_verification_rejects_oversized_evidence(tmp_path: Path, monkeypatch):
    payloads(tmp_path)
    release_manifest.write_manifest(tmp_path, VERSION, COMMIT)
    monkeypatch.setattr(release_manifest, "MAX_EVIDENCE_BYTES", 8)
    assert "exceeds" in release_manifest.verify_manifest(tmp_path, VERSION, COMMIT)[0]


@pytest.mark.parametrize(
    "mutation",
    [
        "tampered",
        "extra",
        "missing",
        "manifest",
        "checksums",
        "missing-manifest",
        "missing-checksums",
    ],
)
def test_verification_rejects_missing_extra_or_tampered_assets(tmp_path: Path, mutation: str):
    payloads(tmp_path)
    release_manifest.write_manifest(tmp_path, VERSION, COMMIT)
    if mutation == "tampered":
        (tmp_path / "a.bin").write_bytes(b"altered")
    elif mutation == "extra":
        (tmp_path / "extra.bin").write_bytes(b"extra")
    elif mutation == "missing":
        (tmp_path / "a.bin").unlink()
    elif mutation == "manifest":
        (tmp_path / "release-manifest.json").write_text("{}\n", encoding="utf-8")
    elif mutation == "checksums":
        sums = tmp_path / "SHA256SUMS"
        sums.write_text(
            sums.read_text(encoding="utf-8") + "0" * 64 + "  extra.bin\n",
            encoding="utf-8",
        )
    elif mutation == "missing-manifest":
        (tmp_path / "release-manifest.json").unlink()
    else:
        (tmp_path / "SHA256SUMS").unlink()
    assert release_manifest.verify_manifest(tmp_path, VERSION, COMMIT)


def test_verification_uses_expected_release_identity(tmp_path: Path):
    payloads(tmp_path)
    release_manifest.write_manifest(tmp_path, VERSION, COMMIT)
    assert release_manifest.verify_manifest(tmp_path, "0.1.3", COMMIT)
    assert release_manifest.verify_manifest(tmp_path, VERSION, "b" * 40)
    assert release_manifest.verify_manifest(tmp_path, VERSION, "short")


@pytest.mark.parametrize(
    "manifest_text",
    ['{"schema_version":1,"schema_version":1}', "[]", "null", "{"],
)
def test_verification_rejects_malformed_manifest(tmp_path: Path, manifest_text: str):
    payloads(tmp_path)
    release_manifest.write_manifest(tmp_path, VERSION, COMMIT)
    (tmp_path / "release-manifest.json").write_text(manifest_text, encoding="utf-8")
    assert release_manifest.verify_manifest(tmp_path, VERSION, COMMIT)


def test_cli_verify_never_repairs_tampering(tmp_path: Path, monkeypatch, capsys):
    payloads(tmp_path)
    monkeypatch.setattr(
        "sys.argv",
        ["create_release_manifest.py", str(tmp_path), "--version", VERSION, "--commit", COMMIT],
    )
    assert release_manifest.main() == 0
    (tmp_path / "a.bin").write_bytes(b"altered")
    snapshot = {path.name: path.read_bytes() for path in tmp_path.iterdir()}
    monkeypatch.setattr(
        "sys.argv",
        [
            "create_release_manifest.py",
            str(tmp_path),
            "--version",
            VERSION,
            "--commit",
            COMMIT,
            "--verify",
        ],
    )
    assert release_manifest.main() == 1
    assert "error" in capsys.readouterr().err.lower()
    assert snapshot == {path.name: path.read_bytes() for path in tmp_path.iterdir()}
