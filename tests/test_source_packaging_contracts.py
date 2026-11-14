"""Offline regression contracts for deliberately blocked source-package templates."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SOURCE_COMMIT = "86fcbd8914037735b6f27e5a329d5f3013033efd"
BUILD_FILES = (
    "packaging/aur/PKGBUILD",
    "packaging/debian/rules",
    "packaging/fedora/fyi-cli.spec",
    "packaging/snap/snapcraft.yaml",
    "packaging/flatpak/io.github.edithatogo.fyi-cli.yml",
)


@pytest.mark.parametrize("relative", BUILD_FILES)
def test_source_builds_never_retry_unlocked_or_hide_failures(relative: str) -> None:
    text = (ROOT / relative).read_text(encoding="utf-8")
    if relative.endswith(".yaml"):
        text = json.loads(text)["parts"]["fyi-cli"]["override-build"]
    elif relative.endswith(".yml"):
        text = "\n".join(
            command
            for module in json.loads(text)["modules"]
            for command in module["build-commands"]
        )
    executable = "\n".join(line for line in text.splitlines() if not line.lstrip().startswith("#"))
    assert "||" not in executable
    assert "cargo fetch" not in executable
    for line in executable.splitlines():
        if re.search(r"\bcargo\b.*\b(build|test)\b", line):
            assert "--frozen" in line


def test_source_templates_pin_immutable_source_and_missing_hashes() -> None:
    for relative in (
        "packaging/aur/PKGBUILD",
        "packaging/nix/default.nix",
        "packaging/snap/snapcraft.yaml",
        "packaging/flatpak/io.github.edithatogo.fyi-cli.yml",
        "packaging/fedora/fyi-cli.spec",
    ):
        text = (ROOT / relative).read_text(encoding="utf-8")
        assert SOURCE_COMMIT in text
        assert "@" in text
        assert "fakeHash" not in text
        assert "'SKIP'" not in text


def test_nix_claims_linux_and_keeps_checks_enabled() -> None:
    text = (ROOT / "packaging/nix/default.nix").read_text(encoding="utf-8")
    assert "platforms.windows" not in text
    assert "doCheck = false" not in text
    assert "@SOURCE_NAR_HASH@" in text
    assert "@CARGO_VENDOR_HASH@" in text


def test_flatpak_has_no_host_home_or_build_network_permission() -> None:
    manifest = json.loads(
        (ROOT / "packaging/flatpak/io.github.edithatogo.fyi-cli.yml").read_text(encoding="utf-8"),
    )
    assert "--filesystem=home" not in manifest["finish-args"]
    assert not any(arg.startswith("--filesystem=") for arg in manifest["finish-args"])
    for module in manifest["modules"]:
        assert "--share=network" not in module.get("build-options", {}).get("build-args", [])
    assert manifest["runtime-version"] != "24.08"


def test_snap_declares_hashed_vendor_input_and_frozen_build() -> None:
    manifest = json.loads((ROOT / "packaging/snap/snapcraft.yaml").read_text(encoding="utf-8"))
    assert manifest["parts"]["vendor"]["source-checksum"] == "sha256/@CARGO_VENDOR_SHA256@"
    assert manifest["parts"]["fyi-cli"]["plugin"] == "nil"
    assert manifest["confinement"] == "strict"
    for app in manifest["apps"].values():
        assert "removable-media" not in app["plugs"]


def test_snap_requires_a_checksummed_offline_toolchain() -> None:
    manifest = json.loads((ROOT / "packaging/snap/snapcraft.yaml").read_text(encoding="utf-8"))
    part = manifest["parts"]["fyi-cli"]
    assert not {"cargo", "rustc"}.intersection(part["build-packages"])
    assert {"vendor", "rust-toolchain"}.issubset(part["after"])
    toolchain = manifest["parts"]["rust-toolchain"]
    assert toolchain["source"] == "fyi-cli-rust-toolchain.tar.gz"
    assert toolchain["source-checksum"] == "sha256/@RUST_TOOLCHAIN_SHA256@"
    assert toolchain["prime"] == ["-*"]


@pytest.mark.skipif(
    os.name == "nt" or shutil.which("bash") is None or shutil.which("dpkg") is None,
    reason="The Ubuntu shell/toolchain qualification commands are unavailable",
)
@pytest.mark.parametrize("toolchain_state", ["missing", "old", "wrong_arch", "qualified"])
def test_snap_qualifies_toolchain_before_building(tmp_path: Path, toolchain_state: str) -> None:
    manifest = json.loads((ROOT / "packaging/snap/snapcraft.yaml").read_text(encoding="utf-8"))
    stage = tmp_path / "stage"
    stage.mkdir()
    (stage / "vendor").mkdir()
    (stage / "cargo-config.toml").write_text("# Offline fixture\n", encoding="utf-8")
    prefix = stage / "rust-toolchain"
    prefix.mkdir()
    tool_bin = prefix / "bin"
    tool_bin.mkdir()
    build_log = tmp_path / "cargo-build.log"
    fallback_log = tmp_path / "fallback.log"
    fallback_bin = tmp_path / "fallback-bin"
    fallback_bin.mkdir()
    fallback = fallback_bin / "cargo"
    fallback.write_text('#!/bin/sh\nprintf called >> "$FALLBACK_LOG"\nexit 97\n', encoding="utf-8")
    fallback.chmod(0o755)

    version = "1.75.0" if toolchain_state == "old" else "1.88.0"
    host = (
        "aarch64-unknown-linux-gnu"
        if toolchain_state == "wrong_arch"
        else "x86_64-unknown-linux-gnu"
    )
    (stage / "toolchain-version").write_text(version + "\n", encoding="utf-8")
    (stage / "toolchain-target").write_text("x86_64-unknown-linux-gnu\n", encoding="utf-8")
    if toolchain_state != "missing":
        rustc = tool_bin / "rustc"
        rustc.write_text(
            f"#!/bin/sh\nprintf 'rustc {version}\\nrelease: {version}\\nhost: {host}\\n'\n",
            encoding="utf-8",
        )
        rustc.chmod(0o755)
        cargo = tool_bin / "cargo"
        cargo.write_text(
            '#!/bin/sh\nif [ "$1" = "--version" ]; then\n'
            f"  printf 'cargo {version}\\n'\n  exit 0\nfi\n"
            'printf "%s\\n" "$*" >> "$CARGO_BUILD_LOG"\n'
            "mkdir -p target/release\n"
            'printf "#!/bin/sh\\nexit 0\\n" > target/release/fyi-cli\n'
            "cp target/release/fyi-cli target/release/fyi-mcp\n",
            encoding="utf-8",
        )
        cargo.chmod(0o755)

    environment = os.environ.copy()
    environment.update(
        {
            "PATH": str(fallback_bin) + os.pathsep + environment["PATH"],
            "CRAFT_STAGE": str(stage),
            "CRAFT_PART_INSTALL": str(tmp_path / "install"),
            "CRAFT_ARCH_BUILD_FOR": "amd64",
            "CARGO_BUILD_LOG": str(build_log),
            "FALLBACK_LOG": str(fallback_log),
        },
    )
    result = subprocess.run(  # noqa: S603 - parsed repository hook, isolated fake toolchain.
        [shutil.which("bash"), "-c", manifest["parts"]["fyi-cli"]["override-build"]],
        cwd=tmp_path,
        env=environment,
        check=False,
        capture_output=True,
        text=True,
        timeout=10,
    )
    assert not fallback_log.exists()
    if toolchain_state == "qualified":
        assert result.returncode == 0, result.stderr
        commands = build_log.read_text(encoding="utf-8").splitlines()
        assert len(commands) == 2
        assert all("--frozen" in command for command in commands)
        assert (tmp_path / "install/bin/fyi-cli").is_file()
        assert (tmp_path / "install/bin/fyi-mcp").is_file()
    else:
        assert result.returncode != 0
        assert not build_log.exists()


@pytest.mark.skipif(
    shutil.which("bash") is None or os.name == "nt",
    reason="POSIX Bash is unavailable",
)
def test_aur_check_preserves_cargo_failure(tmp_path: Path) -> None:
    source = tmp_path / f"fyi-cli-{SOURCE_COMMIT}"
    source.mkdir()
    result = subprocess.run(  # noqa: S603 - trusted repository hook; cargo is a shell fixture.
        [
            shutil.which("bash"),
            "-c",
            'source "$1"; srcdir="$2"; cargo() { return 42; }; check',
            "test-source-packaging",
            str(ROOT / "packaging/aur/PKGBUILD"),
            str(tmp_path),
        ],
        check=False,
        capture_output=True,
        text=True,
        timeout=10,
    )
    assert result.returncode == 42


@pytest.mark.skipif(
    shutil.which("make") is None or os.name == "nt",
    reason="POSIX make is unavailable",
)
def test_debian_configure_rejects_missing_vendor_before_cargo(tmp_path: Path) -> None:
    result = subprocess.run(  # noqa: S603 - trusted makefile and isolated temporary source tree.
        [
            shutil.which("make"),
            "-f",
            str(ROOT / "packaging/debian/rules"),
            "override_dh_auto_configure",
        ],
        cwd=tmp_path,
        check=False,
        capture_output=True,
        text=True,
        timeout=10,
    )
    assert result.returncode != 0
    assert "vendor" in result.stdout + result.stderr


def test_debian_maintainer_fields_are_mailboxes() -> None:
    text = (ROOT / "packaging/debian/control").read_text(encoding="utf-8")
    fields = re.findall(r"^(?:Maintainer|Uploaders): (.+)$", text, re.MULTILINE)
    assert fields
    assert all(re.search(r"<[^<>\s]+@[^<>\s]+>", field) for field in fields)
