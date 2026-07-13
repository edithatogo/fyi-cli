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
    missing = module.missing_release_assets(["crates/fyi-core/Cargo.toml"])
    assert missing
    assert "crates/fyi-cli/Cargo.toml" in missing
    assert "server.json" in missing


def test_synchronized_rust_release_is_accepted():
    module = load_module()
    assert module.missing_release_assets(module.VERSIONED_RELEASE_ASSETS) == []
