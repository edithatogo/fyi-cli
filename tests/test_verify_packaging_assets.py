"""Unit tests for scripts/verify_packaging_assets.py."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path


def load_module():
    script_path = (
        Path(__file__).resolve().parents[1] / "scripts" / "verify_packaging_assets.py"
    )
    spec = importlib.util.spec_from_file_location("verify_packaging_assets", script_path)
    module = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_repo_packaging_assets_pass():
    module = load_module()
    repo_root = Path(__file__).resolve().parents[1]
    report = module.verify_packaging_assets(repo_root)

    assert report.expected_version == "0.1.2"
    assert report.ok, module.format_human_report(report)
    critical_ok = [
        r for r in report.results if r.kind == "critical" and r.ok
    ]
    assert len(critical_ok) >= len(module.CRITICAL_ASSETS)


def test_missing_critical_fails(tmp_path: Path):
    module = load_module()
    # Empty tree → all critical assets missing
    report = module.verify_packaging_assets(
        tmp_path,
        expected_version="0.1.2",
        optional=(),
        versioned=(),
    )
    assert not report.ok
    assert report.errors
    assert all(r.kind == "critical" for r in report.errors)


def test_version_mismatch_detected(tmp_path: Path):
    module = load_module()
    rel = "packaging/scoop/fyi-cli.json"
    path = tmp_path / rel
    path.parent.mkdir(parents=True)
    path.write_text('{"version": "9.9.9"}\n', encoding="utf-8")

    # Only check this one versioned file; presence of full critical set skipped.
    report = module.verify_packaging_assets(
        tmp_path,
        expected_version="0.1.2",
        critical=(),
        optional=(),
        versioned=(rel,),
    )
    assert not report.ok
    assert any(r.kind == "version" and not r.ok for r in report.results)


def test_file_mentions_version():
    module = load_module()
    assert module.file_mentions_version('version = "0.1.2"', "0.1.2")
    assert not module.file_mentions_version('version = "0.1.1"', "0.1.2")


def test_main_exit_code_on_repo():
    module = load_module()
    repo_root = Path(__file__).resolve().parents[1]
    code = module.main(["--repo-root", str(repo_root)])
    assert code == 0
