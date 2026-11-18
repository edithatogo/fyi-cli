"""Reject incomplete compatibility receipts instead of reporting empty green cells."""

from __future__ import annotations

import io
import json
import os
import shutil
import subprocess
import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path
from unittest.mock import patch

import pytest

from scripts import verify_mcp_compatibility as compatibility


class CompatibilityEvidenceTests(unittest.TestCase):
    """The compatibility gate requires every declared scenario to execute."""

    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.report = self.root / "sdk.xml"

    def write_report(self, names: tuple[str, ...] | None = None) -> ET.Element:
        """Write a synthetic, complete JUnit report for the declared stdio cases."""
        cases = compatibility.SDK_CASES if names is None else names
        suite = ET.Element(
            "testsuite",
            tests=str(len(cases)),
            failures="0",
            errors="0",
            skipped="0",
        )
        for name in cases:
            ET.SubElement(suite, "testcase", name=name, classname="tests.test_mcp_client_interop")
        ET.ElementTree(suite).write(self.report, encoding="utf-8")
        return suite

    def test_complete_executed_stdio_matrix_is_accepted(self) -> None:
        self.write_report()
        assert compatibility.verify_junit(self.report) == 16
        assert all("http" not in name for name in compatibility.SDK_CASES)

    def test_missing_report_is_rejected(self) -> None:
        with pytest.raises(ValueError, match="JUnit"):
            compatibility.verify_junit(self.report)

    def test_empty_or_missing_case_is_rejected(self) -> None:
        for cases in ((), compatibility.SDK_CASES[:-1]):
            with self.subTest(cases=len(cases)):
                self.write_report(cases)
                with pytest.raises(ValueError, match="scenario"):
                    compatibility.verify_junit(self.report)

    def test_duplicate_or_unexpected_case_is_rejected(self) -> None:
        for extra in (compatibility.SDK_CASES[0], "test_unqualified_http"):
            with self.subTest(extra=extra):
                self.write_report((*compatibility.SDK_CASES, extra))
                with pytest.raises(ValueError, match="scenario"):
                    compatibility.verify_junit(self.report)

    def test_skipped_failed_or_errored_case_is_rejected(self) -> None:
        for outcome in ("skipped", "failure", "error"):
            with self.subTest(outcome=outcome):
                suite = self.write_report()
                ET.SubElement(suite[0], outcome, message="synthetic test outcome")
                ET.ElementTree(suite).write(self.report, encoding="utf-8")
                with pytest.raises(ValueError, match="unsuccessful"):
                    compatibility.verify_junit(self.report)

    def test_suite_level_errors_are_not_hidden_by_successful_testcases(self) -> None:
        suite = self.write_report()
        suite.set("errors", "1")
        ET.ElementTree(suite).write(self.report, encoding="utf-8")
        with pytest.raises(ValueError, match="unsuccessful"):
            compatibility.verify_junit(self.report)

    def test_malformed_report_is_rejected(self) -> None:
        self.report.write_text("<testsuite><testcase>", encoding="utf-8")
        with pytest.raises(ValueError, match="JUnit"):
            compatibility.verify_junit(self.report)

    def test_wrong_test_module_cannot_satisfy_case_inventory(self) -> None:
        suite = self.write_report()
        suite[0].set("classname", "tests.unrelated")
        ET.ElementTree(suite).write(self.report, encoding="utf-8")
        with pytest.raises(ValueError, match="module"):
            compatibility.verify_junit(self.report)

    def write_manifests(self, versions: tuple[str, str, str]) -> None:
        """Create a minimal workspace with deliberately independent crate minima."""
        (self.root / "rust-toolchain.toml").write_text(
            '[toolchain]\nchannel = "1.100.0"\n',
            encoding="utf-8",
        )
        for crate, version in zip(("fyi-core", "fyi-cli", "fyi-mcp"), versions, strict=True):
            directory = self.root / "crates" / crate
            directory.mkdir(parents=True, exist_ok=True)
            (directory / "Cargo.toml").write_text(
                f'[package]\nrust-version = "{version}"\n',
                encoding="utf-8",
            )

    def test_toolchains_match_the_current_pin_and_common_minimum(self) -> None:
        self.write_manifests(("1.99", "1.99", "1.99"))
        assert compatibility.toolchain_for("current", self.root) == "1.100.0"
        assert compatibility.toolchain_for("msrv", self.root) == "1.99.1"

    def test_mismatched_or_advanced_msrv_requires_matrix_update(self) -> None:
        for versions in (("1.99", "1.99", "1.100"), ("1.100", "1.100", "1.100")):
            with self.subTest(versions=versions):
                self.write_manifests(versions)
                with pytest.raises(ValueError, match="MSRV"):
                    compatibility.toolchain_for("msrv", self.root)

    def test_installed_client_must_match_the_frozen_lock(self) -> None:
        (self.root / "uv.lock").write_text(
            '[[package]]\nname = "mcp"\nversion = "1.30.0"\n',
            encoding="utf-8",
        )
        compatibility.verify_sdk_version("1.30.0", self.root)
        with pytest.raises(ValueError, match="locked"):
            compatibility.verify_sdk_version("1.29.0", self.root)

    def test_client_roots_are_exact_lock_versions(self) -> None:
        (self.root / "uv.lock").write_text(
            '[[package]]\nname = "mcp"\nversion = "1.30.0"\n'
            '[[package]]\nname = "pytest"\nversion = "9.1.1"\n'
            '[[package]]\nname = "hypothesis"\nversion = "6.156.4"\n',
            encoding="utf-8",
        )
        assert compatibility.locked_client_requirements(self.root) == (
            "mcp==1.30.0",
            "pytest==9.1.1",
            "hypothesis==6.156.4",
        )
        with (self.root / "uv.lock").open("a", encoding="utf-8") as lock:
            lock.write('[[package]]\nname = "mcp"\nversion = "1.29.0"\n')
        with pytest.raises(ValueError, match="unambiguous"):
            compatibility.locked_client_requirements(self.root)

    def test_failed_run_cannot_reuse_a_previous_pass(self) -> None:
        receipt = self.root / "result.json"
        receipt.write_text('{"status":"passed"}', encoding="utf-8")

        def fail_after_checking_reset(*_args: object) -> None:
            assert json.loads(receipt.read_text(encoding="utf-8"))["status"] == "failed"
            message = "synthetic command failure"
            raise ValueError(message)

        with (
            patch("sys.argv", ["verify_mcp_compatibility.py", "--output", str(self.root)]),
            patch.object(compatibility, "run_contracts", side_effect=fail_after_checking_reset),
            patch("sys.stdout", new_callable=io.StringIO),
        ):
            assert compatibility.main() == 1
        result = json.loads(receipt.read_text(encoding="utf-8"))
        assert result["status"] == "failed"
        assert result["error"] == "synthetic command failure"


class CompatibilitySourceIdentityTests(unittest.TestCase):
    """A compatibility pass must belong to committed executable/test inputs."""

    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.git = shutil.which("git")
        assert self.git is not None
        self.environment = os.environ.copy()
        self.run_git("init", "--quiet")
        self.write("Cargo.toml", "synthetic manifest\n")
        self.write("scripts/verify_mcp_compatibility.py", "synthetic runner\n")
        self.write("tests/test_mcp_compatibility.py", "synthetic test\n")
        self.run_git("add", "Cargo.toml", "scripts", "tests")
        self.run_git(
            "-c",
            "user.name=Synthetic Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "Synthetic fixture",
        )
        self.head = self.run_git("rev-parse", "HEAD").strip()

    def run_git(self, *arguments: str) -> str:
        """Operate only on this disposable synthetic fixture repository."""
        result = subprocess.run(  # noqa: S603 - fixture-controlled arguments, no shell.
            [self.git, *arguments],
            cwd=self.root,
            env=self.environment,
            capture_output=True,
            text=True,
            timeout=10,
            check=True,
        )
        return result.stdout

    def write(self, name: str, contents: str) -> None:
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents, encoding="utf-8")

    def test_clean_inputs_ignore_artifacts_and_environment_directories(self) -> None:
        for name in ("target/output", "artifacts/result.json", ".venv/output", "docs/note.md"):
            self.write(name, "synthetic generated output")
        assert compatibility.verify_clean_source(self.git, self.environment, self.root) == self.head

    def test_tracked_modified_input_is_rejected(self) -> None:
        self.write("Cargo.toml", "changed synthetic manifest\n")
        with pytest.raises(ValueError, match="committed"):
            compatibility.verify_clean_source(self.git, self.environment, self.root)

    def test_staged_input_is_rejected(self) -> None:
        self.write("tests/test_mcp_compatibility.py", "changed synthetic test\n")
        self.run_git("add", "tests/test_mcp_compatibility.py")
        with pytest.raises(ValueError, match="committed"):
            compatibility.verify_clean_source(self.git, self.environment, self.root)

    def test_untracked_implementation_or_test_input_is_rejected(self) -> None:
        for name in ("crates/untracked.rs", "scripts/untracked.py", "tests/untracked.py"):
            with self.subTest(name=name):
                self.write(name, "synthetic untracked input\n")
                with pytest.raises(ValueError, match="committed"):
                    compatibility.verify_clean_source(self.git, self.environment, self.root)
                (self.root / name).unlink()

    def test_untracked_runner_cannot_hide_behind_an_ignore_rule(self) -> None:
        self.run_git("rm", "--cached", "scripts/verify_mcp_compatibility.py")
        self.write(".git/info/exclude", "scripts/verify_mcp_compatibility.py\n")
        self.run_git(
            "-c",
            "user.name=Synthetic Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "Untrack fixture runner",
        )
        with pytest.raises(ValueError, match="tracked"):
            compatibility.verify_clean_source(self.git, self.environment, self.root)


if __name__ == "__main__":
    unittest.main()
