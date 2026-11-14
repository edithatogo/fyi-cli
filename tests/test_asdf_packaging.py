"""Offline execution contracts for the asdf-compatible packaging scripts."""

from __future__ import annotations

import gzip
import hashlib
import io
import os
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Callable

ROOT = Path(__file__).resolve().parents[1]
VERSION = "2.3.4"
ASSET_NAMES = (
    "fyi-cli-linux-amd64.tar.gz",
    "fyi-cli-macos-amd64.tar.gz",
    "fyi-cli-macos-arm64.tar.gz",
)
pytestmark = pytest.mark.skipif(os.name == "nt", reason="POSIX asdf plugin scripts")


@pytest.fixture
def plugin(tmp_path: Path) -> Path:
    """Copy only plugin assets; fixtures never modify repository allowlists."""
    result = tmp_path / "plugin"
    shutil.copytree(ROOT / "packaging/asdf", result)
    (result / "checksums").mkdir(exist_ok=True)
    return result


@pytest.fixture
def environment(tmp_path: Path) -> dict[str, str]:
    """Stub every external command that could reach a registry or build code."""
    commands = tmp_path / "commands"
    commands.mkdir()
    # Profilers can prepend Python wrappers and rewrite sys.executable itself.
    interpreter = Path(getattr(sys, "_base_executable", sys.executable)).resolve()
    (commands / "python3").symlink_to(interpreter)
    scripts = {
        "uname": '#!/bin/sh\ncase "$1" in -s) echo "$TEST_OS";; -m) echo "$TEST_ARCH";; esac\n',
        "curl": r"""#!/usr/bin/env python3
import os
import pathlib
import shutil
import sys
if '--version' in sys.argv:
    print(os.environ.get('TEST_CURL_VERSION', 'curl 8.4.0 (fixture) libcurl/8.4.0'))
    sys.exit(0)
pathlib.Path(os.environ['TEST_DOWNLOAD_LOG']).write_text('\n'.join(sys.argv[1:]))
if os.environ.get('TEST_DOWNLOAD_FAIL') == '1':
    sys.exit(22)
flag = '--output' if '--output' in sys.argv else '-o'
shutil.copyfile(os.environ['TEST_ARCHIVE'], sys.argv[sys.argv.index(flag) + 1])
""",
        "cargo": '#!/bin/sh\necho called > "$TEST_BUILD_LOG"\nexit 1\n',
        "gh": '#!/bin/sh\necho called > "$TEST_GH_LOG"\nprintf "fyi-mcp-v0.1.2\\nv1.2.0\\n"\n',
    }
    for name, body in scripts.items():
        executable = commands / name
        executable.write_text(body, encoding="utf-8")
        executable.chmod(0o755)
    return {
        "HOME": str(tmp_path),
        "LANG": "C.UTF-8",
        "PATH": str(commands) + os.pathsep + os.environ["PATH"],
        "ASDF_INSTALL_TYPE": "version",
        "ASDF_INSTALL_VERSION": VERSION,
        "ASDF_INSTALL_PATH": str(tmp_path / "installation"),
        "TEST_OS": "Linux",
        "TEST_ARCH": "x86_64",
        "TEST_ARCHIVE": str(tmp_path / "release.tar.gz"),
        "TEST_DOWNLOAD_LOG": str(tmp_path / "downloads.log"),
        "TEST_BUILD_LOG": str(tmp_path / "builds.log"),
        "TEST_GH_LOG": str(tmp_path / "gh.log"),
    }


@pytest.fixture
def archive(environment: dict[str, str]) -> Callable[..., Path]:
    def write_archive(
        *,
        name: str = "fyi-cli",
        kind: bytes = tarfile.REGTYPE,
        extra: bool = False,
        binary_version: str = VERSION,
    ) -> Path:
        path = Path(environment["TEST_ARCHIVE"])
        content = f'#!/bin/sh\necho "fyi-cli {binary_version}"\n'.encode()
        with tarfile.open(path, "w:gz") as handle:
            member = tarfile.TarInfo(name)
            member.type = kind
            member.mode = 0o755
            member.size = len(content) if kind == tarfile.REGTYPE else 0
            if kind in {tarfile.SYMTYPE, tarfile.LNKTYPE}:
                member.linkname = "../outside"
            handle.addfile(member, io.BytesIO(content))
            if extra:
                handle.addfile(member, io.BytesIO(content))
        return path

    return write_archive


def approve(plugin: Path, path: Path, version: str = VERSION) -> Path:
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    receipt = plugin / "checksums" / f"{version}.sha256"
    receipt.write_text(
        f"# release-tag: v{version}\n# binary-version: {version}\n"
        + "".join(f"{digest}  {name}\n" for name in ASSET_NAMES),
        encoding="utf-8",
    )
    return receipt


def run_script(plugin: Path, environment: dict[str, str], name: str = "install"):
    return subprocess.run(  # noqa: S603 - local fixture scripts only
        [shutil.which("bash"), str(plugin / "bin" / name)],
        env=environment,
        capture_output=True,
        text=True,
        timeout=10,
        check=False,
    )


def test_no_versions_listed_without_approved_release(plugin, environment):
    result = run_script(plugin, environment, "list-all")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == ""
    assert not Path(environment["TEST_GH_LOG"]).exists()
    assert not Path(environment["TEST_DOWNLOAD_LOG"]).exists()


def test_inherited_python_wrapper_cannot_intercept_fixture_commands(
    plugin,
    environment,
    archive,
    tmp_path,
):
    wrappers = tmp_path / "profiler-wrappers"
    wrappers.mkdir()
    wrapper = wrappers / "python3"
    wrapper.write_text('#!/bin/sh\necho called > "$TEST_PYTHON_WRAPPER_LOG"\nexit 73\n')
    wrapper.chmod(0o755)
    log = tmp_path / "python-wrapper.log"
    environment["TEST_PYTHON_WRAPPER_LOG"] = str(log)
    fixture_commands, inherited = environment["PATH"].split(os.pathsep, 1)
    environment["PATH"] = os.pathsep.join((fixture_commands, str(wrappers), inherited))
    approve(plugin, archive())

    versions = run_script(plugin, environment, "list-all")
    assert versions.returncode == 0, versions.stderr
    assert versions.stdout.strip() == VERSION
    installed = run_script(plugin, environment)
    assert installed.returncode == 0, installed.stderr
    assert (Path(environment["ASDF_INSTALL_PATH"]) / "bin/fyi-cli").is_file()
    assert not log.exists()


@pytest.mark.parametrize(
    ("system", "machine", "asset"),
    [
        ("Linux", "x86_64", ASSET_NAMES[0]),
        ("Darwin", "x86_64", ASSET_NAMES[1]),
        ("Darwin", "arm64", ASSET_NAMES[2]),
    ],
)
def test_installs_only_verified_matching_platform(
    plugin,
    environment,
    archive,
    system,
    machine,
    asset,
):
    approve(plugin, archive())
    environment.update(TEST_OS=system, TEST_ARCH=machine)
    result = run_script(plugin, environment)
    assert result.returncode == 0, result.stderr
    binary = Path(environment["ASDF_INSTALL_PATH"]) / "bin/fyi-cli"
    assert binary.is_file()
    assert os.access(binary, os.X_OK)
    arguments = Path(environment["TEST_DOWNLOAD_LOG"]).read_text().splitlines()
    assert (
        arguments[-1]
        == f"https://github.com/edithatogo/fyi-cli/releases/download/v{VERSION}/{asset}"
    )
    assert "--max-time" in arguments
    assert "--max-filesize" in arguments
    assert not (binary.parent / "fyi-mcp").exists()


def test_checksum_mismatch_preserves_existing_install(plugin, environment, archive):
    path = archive()
    approve(plugin, path)
    path.write_bytes(path.read_bytes() + b"corruption")
    binary = Path(environment["ASDF_INSTALL_PATH"]) / "bin/fyi-cli"
    binary.parent.mkdir(parents=True)
    binary.write_text("previous installation")
    result = run_script(plugin, environment)
    assert result.returncode != 0
    assert "checksum" in result.stderr.lower()
    assert binary.read_text() == "previous installation"


@pytest.mark.parametrize(
    "reported_version",
    ["curl 7.88.1 (fixture)", "curl 8.3.0 (fixture)", "unrecognized curl version"],
)
def test_old_or_unknown_curl_rejects_before_download(
    plugin,
    environment,
    archive,
    reported_version,
):
    approve(plugin, archive())
    environment["TEST_CURL_VERSION"] = reported_version
    result = run_script(plugin, environment)
    assert result.returncode != 0
    assert "curl 8.4.0" in result.stderr
    assert not Path(environment["TEST_DOWNLOAD_LOG"]).exists()


def test_curl_version_comparison_is_numeric(plugin, environment, archive):
    approve(plugin, archive())
    environment["TEST_CURL_VERSION"] = "curl 8.10.1 (fixture) libcurl/8.10.1"
    result = run_script(plugin, environment)
    assert result.returncode == 0, result.stderr


@pytest.mark.parametrize(
    "defect",
    [
        "concatenated-tar",
        "concatenated-gzip",
        "nonzero-member-padding",
        "nonzero-trailer",
        "missing-end-markers",
        "truncated-gzip",
    ],
)
def test_full_archive_validation_preserves_existing_install(
    plugin,
    environment,
    archive,
    defect,
):
    path = archive()
    compressed = path.read_bytes()
    unpacked = gzip.decompress(compressed)
    with tarfile.open(fileobj=io.BytesIO(unpacked)) as bundle:
        member = bundle.next()
        data_end = member.offset_data + member.size
    if defect == "concatenated-tar":
        damaged = gzip.compress(unpacked + unpacked)
    elif defect == "concatenated-gzip":
        damaged = compressed + compressed
    elif defect == "nonzero-member-padding":
        damaged = gzip.compress(unpacked[:data_end] + b"X" + unpacked[data_end + 1 :])
    elif defect == "nonzero-trailer":
        damaged = gzip.compress(unpacked + b"unexpected")
    elif defect == "missing-end-markers":
        damaged = gzip.compress(unpacked[:1024])
    else:
        damaged = compressed[:-8]
    path.write_bytes(damaged)
    approve(plugin, path)
    binary = Path(environment["ASDF_INSTALL_PATH"]) / "bin/fyi-cli"
    binary.parent.mkdir(parents=True)
    binary.write_text("previous installation")
    result = run_script(plugin, environment)
    assert result.returncode != 0
    assert binary.read_text() == "previous installation"


def test_inflated_limit_covers_padding_after_tar_end(plugin, environment, archive):
    # Scale the cap only in the isolated plugin copy; keep this regression small.
    installer = plugin / "bin/install"
    content = installer.read_text()
    original = "MAX_BINARY_BYTES = 256 * 1024 * 1024"
    assert content.count(original) == 1
    installer.write_text(content.replace(original, "MAX_BINARY_BYTES = 1024"))
    path = archive()
    path.write_bytes(gzip.compress(gzip.decompress(path.read_bytes()) + bytes(512 * 1024)))
    approve(plugin, path)
    result = run_script(plugin, environment)
    assert result.returncode != 0
    assert "decompressed archive exceeds" in result.stderr
    assert not (Path(environment["ASDF_INSTALL_PATH"]) / "bin/fyi-cli").exists()


@pytest.mark.parametrize(
    ("name", "kind", "extra"),
    [
        ("../fyi-cli", tarfile.REGTYPE, False),
        ("/fyi-cli", tarfile.REGTYPE, False),
        ("fyi-cli", tarfile.SYMTYPE, False),
        ("fyi-cli", tarfile.LNKTYPE, False),
        ("fyi-cli", tarfile.REGTYPE, True),
    ],
)
def test_unsafe_archives_fail_closed(plugin, environment, archive, name, kind, extra):
    approve(plugin, archive(name=name, kind=kind, extra=extra))
    result = run_script(plugin, environment)
    assert result.returncode != 0
    assert not (Path(environment["ASDF_INSTALL_PATH"]) / "bin/fyi-cli").exists()
    assert not Path(environment["TEST_BUILD_LOG"]).exists()


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("ASDF_INSTALL_TYPE", "ref"),
        ("ASDF_INSTALL_VERSION", "../../other"),
        ("ASDF_INSTALL_VERSION", "1.2.3-rc.1"),
        ("ASDF_INSTALL_VERSION", "01.2.3"),
        ("TEST_ARCH", "aarch64"),
        ("TEST_OS", "FreeBSD"),
    ],
)
def test_unsupported_requests_reject_before_download(plugin, environment, archive, key, value):
    approve(plugin, archive())
    environment[key] = value
    result = run_script(plugin, environment)
    assert result.returncode != 0
    assert not Path(environment["TEST_DOWNLOAD_LOG"]).exists()
    assert not Path(environment["TEST_BUILD_LOG"]).exists()


def test_unapproved_version_rejects_before_download(plugin, environment, archive):
    archive()
    result = run_script(plugin, environment)
    assert result.returncode != 0
    assert not Path(environment["TEST_DOWNLOAD_LOG"]).exists()


def test_version_mismatch_in_archive_rejects(plugin, environment, archive):
    approve(plugin, archive(binary_version="1.0.0"))
    result = run_script(plugin, environment)
    assert result.returncode != 0
    assert not (Path(environment["ASDF_INSTALL_PATH"]) / "bin/fyi-cli").exists()


def test_allowlist_version_mismatch_rejects(plugin, environment, archive):
    receipt = approve(plugin, archive())
    receipt.write_text(receipt.read_text().replace(f"v{VERSION}", "v1.0.0"))
    result = run_script(plugin, environment)
    assert result.returncode != 0
    assert not Path(environment["TEST_DOWNLOAD_LOG"]).exists()


@pytest.mark.parametrize("mode", ["install", "list-all"])
@pytest.mark.parametrize("defect", ["duplicate", "placeholder", "unsupported-asset"])
def test_malformed_approval_fails_without_network(plugin, environment, archive, mode, defect):
    receipt = approve(plugin, archive())
    content = receipt.read_text()
    entry = content.splitlines()[2]
    if defect == "duplicate":
        content += entry + "\n"
    elif defect == "placeholder":
        content = content.replace(entry.split()[0], "0" * 64)
    else:
        content = content.replace(ASSET_NAMES[0], "fyi-mcp-linux-amd64.tar.gz")
    receipt.write_text(content)
    result = run_script(plugin, environment, mode)
    assert result.returncode != 0
    assert not Path(environment["TEST_DOWNLOAD_LOG"]).exists()
    assert not Path(environment["TEST_GH_LOG"]).exists()


def test_failed_download_never_builds_fallback(plugin, environment, archive):
    approve(plugin, archive())
    environment["TEST_DOWNLOAD_FAIL"] = "1"
    result = run_script(plugin, environment)
    assert result.returncode != 0
    assert not Path(environment["TEST_BUILD_LOG"]).exists()


def test_list_all_is_sorted_and_excludes_mcp_only_versions(plugin, environment, archive):
    path = archive()
    for version in ["2.10.0", "2.9.0", "1.0.0"]:
        approve(plugin, path, version)
    result = run_script(plugin, environment, "list-all")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "1.0.0 2.9.0 2.10.0"
    assert not Path(environment["TEST_GH_LOG"]).exists()


def test_symlink_install_directory_is_rejected(plugin, environment, archive, tmp_path):
    approve(plugin, archive())
    destination = tmp_path / "outside"
    destination.mkdir()
    prefix = Path(environment["ASDF_INSTALL_PATH"])
    prefix.mkdir()
    (prefix / "bin").symlink_to(destination, target_is_directory=True)
    result = run_script(plugin, environment)
    assert result.returncode != 0
    assert not (destination / "fyi-cli").exists()
