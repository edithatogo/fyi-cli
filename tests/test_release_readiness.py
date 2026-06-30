import importlib.util
import sys
from pathlib import Path


def load_release_readiness_module():
    script_path = Path(__file__).resolve().parents[1] / "scripts" / "release_readiness.py"
    spec = importlib.util.spec_from_file_location("release_readiness", script_path)
    module = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_release_readiness_inventory_passes_after_release_hygiene(tmp_path):
    module = load_release_readiness_module()
    repo_root = Path(__file__).resolve().parents[1]
    report = module.build_report(repo_root)

    scanned_paths = {surface["path"] for surface in report["surfaces"]}

    assert report["issues"] == []
    assert "README.md" in scanned_paths
    assert "pyproject.toml" in scanned_paths

    checklist = tmp_path / "release-readiness.md"
    module.write_markdown_report(report, checklist)
    content = checklist.read_text(encoding="utf-8")

    assert "# Release Readiness Inventory" in content
    assert "No issues found" in content
    assert "cargo +stable-x86_64-pc-windows-gnu test --workspace --all-features" in content
