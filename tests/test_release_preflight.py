import importlib.util
import sys
from pathlib import Path


def load_module():
    script = Path(__file__).resolve().parents[1] / "scripts" / "release_preflight.py"
    spec = importlib.util.spec_from_file_location("release_preflight", script)
    module = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_current_release_preflight_passes():
    module = load_module()
    report = module.build_report()
    assert report["ok"] is True
    assert report["version"] == "0.1.2"
    assert any(
        check["id"] == "hosted-connector-docs" and check["ok"] for check in report["checks"]
    )


def test_requested_version_mismatch_fails():
    module = load_module()
    report = module.build_report(requested_version="9.9.9")
    assert report["ok"] is False
    assert any(check["id"] == "requested-version" and not check["ok"] for check in report["checks"])


def test_operator_actions_include_hosted_connector_prerequisite():
    module = load_module()
    report = module.build_report()
    assert any(
        "FYI_MCP_TRANSPORT=http" in action and "deploy/remote-mcp/README.md" in action
        for action in report["operator_actions"]
    )
