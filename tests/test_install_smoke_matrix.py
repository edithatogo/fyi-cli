import json
from pathlib import Path

from scripts.validate_install_smoke_matrix import validate


ROOT = Path(__file__).parents[1]


def test_install_smoke_matrix_is_valid():
    assert validate(ROOT) == []


def test_install_smokes_are_help_only():
    payload = json.loads((ROOT / "packaging" / "install-smoke-matrix.json").read_text())
    assert all("--help" in target["smoke"] for target in payload["targets"])
    assert all(target["remote_writes"] is False for target in payload["targets"])
