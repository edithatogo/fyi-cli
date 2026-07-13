import importlib.util
import sys
from pathlib import Path


def load_module():
    script = Path(__file__).resolve().parents[1] / "scripts" / "verify_release_scope.py"
    spec = importlib.util.spec_from_file_location("verify_release_scope", script)
    module = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_non_release_changes_do_not_require_distribution_assets():
    module = load_module()
    assert module.missing_release_assets(["src/fyi_cli/__init__.py"]) == []


def test_component_only_rust_release_is_rejected():
    module = load_module()
    missing = module.missing_release_assets(
        ["crates/fyi-core/Cargo.toml"], version_changed=True
    )
    assert missing
    assert "crates/fyi-cli/Cargo.toml" in missing
    assert "server.json" in missing


def test_synchronized_rust_release_is_accepted():
    module = load_module()
    assert module.missing_release_assets(module.VERSIONED_RELEASE_ASSETS) == []


def test_dependency_only_manifest_change_is_allowed():
    module = load_module()

    assert module.missing_release_assets(
        ["crates/fyi-mcp/Cargo.toml"], version_changed=False
    ) == []


def test_only_version_lines_trigger_release_scope():
    module = load_module()

    diff_header = "diff --git a/crates/fyi-core/Cargo.toml b/crates/fyi-core/Cargo.toml\n"
    assert module.rust_version_changed(
        diff_header + '-version = "0.1.2"\n+version = "0.1.3"'
    )
    assert not module.rust_version_changed(diff_header + '+axum = "0.7"')
