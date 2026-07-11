#!/usr/bin/env python3
"""Validate and optionally execute the repository's layered test harness.

The inventory is deliberately explicit: a green unit suite must not be
mistaken for coverage of integration, release, security, or mutation risks.
"""

from __future__ import annotations

import argparse
import logging
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOGGER = logging.getLogger(__name__)


@dataclass(frozen=True)
class Layer:
    name: str
    paths: tuple[str, ...]
    command: tuple[str, ...]
    network: bool = False
    expensive: bool = False


LAYERS = (
    Layer(
        "unit",
        ("crates/fyi-core/tests/db_tests.rs", "tests/test_cli.py"),
        ("cargo", "test", "-p", "fyi-core", "--lib"),
    ),
    Layer(
        "integration",
        ("tests/test_integration.py", "crates/fyi-core/tests/db_tests.rs"),
        ("pytest", "tests/test_integration.py"),
    ),
    Layer(
        "end-to-end",
        (
            "tests/test_e2e_cli.py",
            "tests/test_e2e_workflows.py",
            "crates/fyi-cli/tests/e2e_tests.rs",
        ),
        ("pytest", "tests/test_e2e_cli.py", "tests/test_e2e_workflows.py"),
    ),
    Layer(
        "smoke/system",
        ("tests/test_discovery_smoke.py", ".github/workflows/ci.yml"),
        ("pytest", "tests/test_discovery_smoke.py"),
        network=True,
    ),
    Layer(
        "mutation",
        ("mutation_test.py", "mutants.toml", "run-mutants.ps1"),
        ("python", "mutation_test.py"),
        expensive=True,
    ),
    Layer(
        "property-based",
        ("tests/test_hypothesis.py", "crates/fyi-core/tests/property_tests.rs"),
        ("pytest", "tests/test_hypothesis.py"),
    ),
    Layer(
        "edge",
        ("tests/test_fuzz.py", "tests/test_endorsed_route.py"),
        ("pytest", "tests/test_fuzz.py", "tests/test_endorsed_route.py"),
    ),
    Layer(
        "performance",
        ("tests/test_benchmarks.py", ".github/workflows/profiling.yml"),
        ("pytest", "tests/test_benchmarks.py", "--benchmark-only"),
        expensive=True,
    ),
    Layer(
        "security",
        (
            "tests/test_security.py",
            "tests/test_security_middleware.py",
            "crates/fyi-core/tests/security_tests.rs",
        ),
        ("pytest", "tests/test_security.py", "tests/test_security_middleware.py"),
    ),
    Layer(
        "compatibility",
        ("tests/test_api_contract.py", "tests/test_alaveteli_client.py"),
        ("pytest", "tests/test_api_contract.py", "tests/test_alaveteli_client.py"),
    ),
    Layer(
        "usability",
        ("tests/test_e2e_cli.py", "tests/test_webapp.py", "tests/test_webapp_forms.py"),
        ("pytest", "tests/test_e2e_cli.py", "tests/test_webapp.py", "tests/test_webapp_forms.py"),
    ),
    Layer(
        "regression",
        ("tests/test_phase7.py", "tests/test_phase14.py", "tests/test_release_readiness.py"),
        (
            "pytest",
            "tests/test_phase7.py",
            "tests/test_phase14.py",
            "tests/test_release_readiness.py",
        ),
    ),
    Layer(
        "sanity",
        ("tests/test_cli.py", "tests/test_verify_packaging_assets.py", ".github/workflows/ci.yml"),
        ("pytest", "tests/test_cli.py", "tests/test_verify_packaging_assets.py"),
    ),
    Layer(
        "remote-mcp-contract",
        ("tests/fixtures/remote_mcp/v1.json", "scripts/verify_remote_mcp_contract.py"),
        ("python", "scripts/verify_remote_mcp_contract.py"),
    ),
    Layer(
        "rust-guardrail-boundaries",
        ("crates/fyi-core/src/api.rs", "crates/fyi-core/src/sync.rs", "crates/fyi-core/src/tor.rs", "scripts/verify_rust_guardrail_boundaries.py"),
        ("python", "scripts/verify_rust_guardrail_boundaries.py"),
    ),
)


def validate_inventory() -> list[str]:
    errors: list[str] = []
    for layer in LAYERS:
        missing = [path for path in layer.paths if not (ROOT / path).exists()]
        if missing:
            errors.append(f"{layer.name}: missing {', '.join(missing)}")
    names = [layer.name for layer in LAYERS]
    if len(names) != len(set(names)):
        errors.append("duplicate harness layer names")
    return errors


def run_layer(layer: Layer) -> int:
    if layer.network and not _live_smoke_enabled():
        LOGGER.info("SKIP %s: live smoke is opt-in (set FYI_LIVE_SMOKE=1)", layer.name)
        return 0
    command = _python_command(layer.command)
    LOGGER.info("RUN  %s: %s", layer.name, " ".join(command))
    completed = subprocess.run(  # noqa: S603
        command,
        cwd=ROOT,
        check=False,
        env=_command_environment(command),
    )
    return completed.returncode


def _live_smoke_enabled() -> bool:
    return os.environ.get("FYI_LIVE_SMOKE") == "1"


def _python_command(command: tuple[str, ...]) -> tuple[str, ...]:
    if command and command[0] == "pytest":
        return (sys.executable, "-m", "pytest", *command[1:])
    if command and command[0] == "python":
        return (sys.executable, *command[1:])
    return command


def _command_environment(command: tuple[str, ...]) -> dict[str, str]:
    environment = os.environ.copy()
    if os.name == "nt" and command and command[0] == "cargo":
        environment.setdefault("RUSTUP_TOOLCHAIN", "stable-x86_64-pc-windows-gnu")
        environment.setdefault("CARGO_BUILD_TARGET", "x86_64-pc-windows-gnu")
        linker = Path.home() / "scoop/apps/mingw/current/bin/x86_64-w64-mingw32-gcc.exe"
        if linker.exists():
            environment.setdefault("CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER", str(linker))
    return environment


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--run", action="store_true", help="execute each safe layer after validation",
    )
    parser.add_argument(
        "--run-expensive",
        action="store_true",
        help="also run mutation and benchmark layers; these can take several minutes",
    )
    args = parser.parse_args()

    errors = validate_inventory()
    if errors:
        for error in errors:
            LOGGER.error(error)
            return 1
    LOGGER.info(
        "Validated %d harness layers: %s",
        len(LAYERS),
        ", ".join(layer.name for layer in LAYERS),
    )
    if not args.run:
        return 0
    for layer in LAYERS:
        if layer.expensive and not args.run_expensive:
            LOGGER.info(
                "SKIP %s: pass --run-expensive for long-running analysis", layer.name,
            )
            continue
        if run_layer(layer):
            LOGGER.error("FAILED %s", layer.name)
            return 1
    return 0


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(message)s")
    raise SystemExit(main())
