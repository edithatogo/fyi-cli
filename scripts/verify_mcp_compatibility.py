"""Run portable offline MCP contracts and retain an exact, fail-closed matrix receipt.

The SDK cases exercise stdio only: local reads, schema rejection, write preparation
and denial, and startup/restart recovery. They do not qualify source transports,
HTTP clients, platform ACLs, or remote writes. Prepare locked dependencies before
running; every Cargo invocation here is offline. The existing required Python
3.15.2 suite remains separate from this additional Python 3.12 client matrix.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import time
import tomllib
import xml.etree.ElementTree as ET
from collections import Counter
from importlib.metadata import PackageNotFoundError, version
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MSRV_TOOLCHAIN = "1.99.1"
CLIENT_PACKAGES = ("mcp", "pytest", "hypothesis")
SOURCE_INPUT_SCOPE = (
    "Cargo*",
    "rust-toolchain.toml",
    ".cargo",
    ".gitattributes",
    "crates",
    "scripts",
    "tests",
    "pyproject.toml",
    "uv.lock",
)
TRACKED_VERIFIER_INPUTS = (
    "scripts/verify_mcp_compatibility.py",
    "tests/test_mcp_compatibility.py",
)
SDK_MODULE = "tests/test_mcp_client_interop.py"
SDK_CASES = (
    "test_official_sdk_local_contract[None-stdio]",
    "test_official_sdk_local_contract[2025-06-18-stdio]",
    "test_official_sdk_local_contract[2025-03-26-stdio]",
    "test_official_sdk_invalid_arguments_and_private_errors[stdio]",
    "test_official_sdk_invalid_parameter_property",
    "test_official_sdk_read_only_registration[default]",
    "test_official_sdk_read_only_registration[degraded]",
    "test_official_sdk_read_only_registration[recovered]",
    "test_official_sdk_synthetic_write_preparation",
    "test_official_sdk_degraded_stdio_restart_recovery[true-false]",
    "test_official_sdk_degraded_stdio_restart_recovery[1-0]",
    "test_degraded_stdio_invalid_configuration_fails_before_database[empty]",
    "test_degraded_stdio_invalid_configuration_fails_before_database[case-typo]",
    "test_degraded_stdio_invalid_configuration_fails_before_database[unknown]",
    "test_degraded_stdio_invalid_configuration_fails_before_database[private-value]",
    "test_degraded_stdio_invalid_configuration_fails_before_database[multiline-value]",
)


def toolchain_for(label: str, root: Path = ROOT) -> str:
    """Resolve the pinned current compiler or reject a stale/mismatched MSRV cell."""
    minimums = {
        tomllib.loads((root / "crates" / crate / "Cargo.toml").read_text(encoding="utf-8"))[
            "package"
        ]["rust-version"]
        for crate in ("fyi-core", "fyi-cli", "fyi-mcp")
    }
    if minimums != {".".join(MSRV_TOOLCHAIN.split(".")[:2])}:
        message = "MSRV compatibility compiler must match every workspace crate minimum"
        raise ValueError(message)
    if label == "msrv":
        return MSRV_TOOLCHAIN
    if label != "current":
        message = "Unknown compatibility compiler label"
        raise ValueError(message)
    current = tomllib.loads((root / "rust-toolchain.toml").read_text(encoding="utf-8"))[
        "toolchain"
    ]["channel"]
    if not isinstance(current, str) or re.fullmatch(r"\d+\.\d+\.\d+", current) is None:
        message = "Current compatibility compiler must be an exact pinned release"
        raise ValueError(message)
    return current


def locked_client_requirements(root: Path = ROOT) -> tuple[str, ...]:
    """Read exact client/test roots; uv resolves their hash-locked dependency closure."""
    packages = tomllib.loads((root / "uv.lock").read_text(encoding="utf-8"))["package"]
    requirements = []
    for name in CLIENT_PACKAGES:
        versions = {package["version"] for package in packages if package["name"] == name}
        if len(versions) != 1:
            message = "Client dependency must have one unambiguous locked version"
            raise ValueError(message)
        requirements.append(f"{name}=={versions.pop()}")
    return tuple(requirements)


def verify_sdk_version(installed: str, root: Path = ROOT) -> None:
    """Reject unqualified SDK versions even if their tests happen to pass."""
    packages = tomllib.loads((root / "uv.lock").read_text(encoding="utf-8"))["package"]
    locked = {package["version"] for package in packages if package["name"] == "mcp"}
    if locked != {installed}:
        message = "Installed MCP SDK does not match the single locked version"
        raise ValueError(message)


def verify_junit(path: Path) -> int:
    """Require every selected stdio case exactly once, with no skip or failure."""
    try:
        document = ET.parse(path).getroot()  # noqa: S314 - locally generated pytest report.
    except (OSError, ET.ParseError) as error:
        message = "Missing or malformed SDK JUnit evidence"
        raise ValueError(message) from error
    if document.tag not in {"testsuite", "testsuites"}:
        message = "Unexpected SDK JUnit document"
        raise ValueError(message)
    for element in document.iter():
        if element.tag in {"failure", "error", "skipped"} or (
            element.tag in {"testsuite", "testsuites"}
            and any(element.get(field, "0") != "0" for field in ("failures", "errors", "skipped"))
        ):
            message = "SDK JUnit evidence contains an unsuccessful or skipped scenario"
            raise ValueError(message)
    cases = list(document.iter("testcase"))
    if Counter(case.get("name") for case in cases) != Counter(SDK_CASES):
        message = "SDK JUnit scenario inventory is missing, duplicated, or unexpected"
        raise ValueError(message)
    if any(case.get("classname") != "tests.test_mcp_client_interop" for case in cases):
        message = "SDK JUnit scenarios originate from an unexpected module"
        raise ValueError(message)
    return len(cases)


def run_command(
    command: list[str],
    log: Path,
    environment: dict[str, str],
    *,
    timeout: int = 900,
) -> None:
    """Execute one bounded command without a shell, retaining its complete output."""
    with log.open("w", encoding="utf-8") as output:
        completed = subprocess.run(  # noqa: S603 - fixed tools/arguments, no shell.
            command,
            cwd=ROOT,
            env=environment,
            stdout=output,
            stderr=subprocess.STDOUT,
            timeout=timeout,
            check=False,
        )
    if completed.returncode:
        message = f"Compatibility command failed; see {log.name}"
        raise ValueError(message)


def verify_clean_source(git: str, environment: dict[str, str], root: Path = ROOT) -> str:
    """Require committed build/test inputs and return their checkout identity."""

    def read_git(arguments: list[str], failure: str) -> str:
        completed = subprocess.run(  # noqa: S603 - fixed Git queries, no shell.
            [git, *arguments],
            cwd=root,
            env=environment,
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        if completed.returncode:
            raise ValueError(failure)
        return completed.stdout

    status = read_git(
        ["status", "--porcelain", "--untracked-files=all", "--", *SOURCE_INPUT_SCOPE],
        "Compatibility source state could not be verified",
    )
    if status:
        message = "Compatibility build and test inputs must be committed and clean"
        raise ValueError(message)
    # An ignored copy must not impersonate the versioned evidence verifier.
    read_git(
        ["ls-files", "--error-unmatch", "--", *TRACKED_VERIFIER_INPUTS],
        "Compatibility runner and regression tests must be tracked",
    )
    checkout = read_git(
        ["rev-parse", "HEAD"],
        "Compatibility checkout identity is unavailable",
    ).strip()
    if re.fullmatch(r"[0-9a-f]{40}", checkout) is None:
        message = "Compatibility checkout identity is not a full GitHub commit SHA"
        raise ValueError(message)
    return checkout


def run_contracts(label: str, output: Path) -> dict[str, object]:
    """Build the selected compiler's binary and verify real Rust and SDK clients."""
    output.mkdir(parents=True, exist_ok=True)
    if sys.version_info[:2] != (3, 12) or sys.implementation.name != "cpython":
        message = "This additional compatibility cell requires CPython 3.12"
        raise ValueError(message)
    toolchain = toolchain_for(label)
    sdk_version = version("mcp")
    verify_sdk_version(sdk_version)
    cargo = shutil.which("cargo")
    rustc = shutil.which("rustc")
    git = shutil.which("git")
    if cargo is None or rustc is None or git is None:
        message = "Compatibility evidence requires Cargo, rustc, and git"
        raise ValueError(message)
    environment = os.environ.copy()
    environment.update({"CARGO_NET_OFFLINE": "true", "PYTEST_ADDOPTS": "", "UV_OFFLINE": "1"})
    checkout_sha = verify_clean_source(git, environment)
    (output / "checkout.txt").write_text(checkout_sha + "\n", encoding="utf-8")
    run_command(
        [rustc, f"+{toolchain}", "--version", "--verbose"],
        output / "rustc.txt",
        environment,
        timeout=30,
    )
    run_command(
        [
            cargo,
            f"+{toolchain}",
            "build",
            "--locked",
            "--offline",
            "-p",
            "fyi-mcp",
            "--message-format=json",
        ],
        output / "build.log",
        environment,
    )
    binary = None
    for line in (output / "build.log").read_text(encoding="utf-8").splitlines():
        if not line.startswith("{"):
            continue
        artifact = json.loads(line)
        if artifact.get("target", {}).get("name") == "fyi-mcp" and artifact.get("executable"):
            binary = Path(artifact["executable"])
    if binary is None or not binary.is_file():
        message = "Selected compiler did not produce the fyi-mcp executable"
        raise ValueError(message)
    run_command(
        [
            cargo,
            f"+{toolchain}",
            "test",
            "--locked",
            "--offline",
            "-p",
            "fyi-mcp",
            "--test",
            "wire_contract",
            "--test",
            "write_journal_contract",
            "--",
            "--test-threads=2",
        ],
        output / "rust-contracts.log",
        environment,
    )
    results = re.findall(
        r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;.*? (\d+) filtered out;",
        (output / "rust-contracts.log").read_text(encoding="utf-8"),
    )
    if len(results) != 2 or any(
        int(passed) == 0 or (failed, ignored, filtered) != ("0", "0", "0")
        for passed, failed, ignored, filtered in results
    ):
        message = "Rust compatibility targets require nonempty unfiltered zero-skip execution"
        raise ValueError(message)
    environment["FYI_MCP_TEST_BINARY"] = str(binary.resolve())
    junit = output / "sdk-junit.xml"
    junit.unlink(missing_ok=True)
    command = [
        sys.executable,
        "-m",
        "pytest",
        "-q",
        "-o",
        "addopts=",
        f"--junitxml={junit}",
        *(f"{SDK_MODULE}::{case}" for case in SDK_CASES),
    ]
    run_command(command, output / "sdk.log", environment, timeout=300)
    count = verify_junit(junit)
    with binary.open("rb") as executable:
        binary_sha256 = hashlib.file_digest(executable, "sha256").hexdigest()
    if verify_clean_source(git, environment) != checkout_sha:
        message = "Compatibility source commit changed during execution"
        raise ValueError(message)
    return {
        "schema_version": 1,
        "status": "passed",
        "checkout_sha": checkout_sha,
        "source_inputs_clean": True,
        "source_input_scope": list(SOURCE_INPUT_SCOPE),
        "source_state_checks": ["before_execution", "after_execution"],
        "source_head_sha": os.environ.get("SOURCE_HEAD_SHA"),
        "github_run_id": os.environ.get("GITHUB_RUN_ID"),
        "github_run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "platform": platform.system(),
        "platform_release": platform.release(),
        "architecture": platform.machine(),
        "python": platform.python_version(),
        "python_implementation": platform.python_implementation(),
        "compiler_cell": label,
        "compiler": (output / "rustc.txt").read_text(encoding="utf-8"),
        "mcp_sdk": sdk_version,
        "client_dependencies": {name: version(name) for name in CLIENT_PACKAGES},
        "binary_sha256": binary_sha256,
        "cargo_lock_sha256": hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest(),
        "uv_lock_sha256": hashlib.sha256((ROOT / "uv.lock").read_bytes()).hexdigest(),
        "rust_test_counts": [int(result[0]) for result in results],
        "sdk_cases_passed": count,
        "sdk_cases": list(SDK_CASES),
        "scope": "offline stdio, local persistence, read-only denial, gated preparation/restart",
        "unqualified": [
            "source transports",
            "successful remote writes",
            "HTTP clients",
            "platform ACLs",
            "power loss",
            "peak source memory",
        ],
    }


def main() -> int:
    """Prepare matrix metadata or execute a cell, never reuse a previous pass."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--toolchain", choices=("current", "msrv"), default="current")
    parser.add_argument("--print-toolchain", action="store_true")
    parser.add_argument("--print-client-requirements", action="store_true")
    parser.add_argument("--output", type=Path, default=ROOT / "artifacts/mcp-compatibility")
    arguments = parser.parse_args()
    if arguments.print_toolchain:
        print(toolchain_for(arguments.toolchain))  # noqa: T201 - machine-readable matrix output.
        return 0
    if arguments.print_client_requirements:
        print("\n".join(locked_client_requirements()))  # noqa: T201 - pip requirements output.
        return 0
    output = arguments.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    report_path = output / "result.json"
    report: dict[str, object] = {"schema_version": 1, "status": "failed"}
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    started = time.monotonic()
    try:
        report = run_contracts(arguments.toolchain, output)
    except (OSError, ValueError, PackageNotFoundError, subprocess.TimeoutExpired) as error:
        report["error"] = str(error)
    report["duration_seconds"] = round(time.monotonic() - started, 3)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))  # noqa: T201 - compatibility receipt.
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
