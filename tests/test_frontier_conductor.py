"""Conductor frontier contract regression tests (pure stdlib, no network)."""
import json
from pathlib import Path

from scripts.verify_frontier_conductor import evaluate, inventory_legacy_python


def make_repo(tmp_path: Path) -> Path:
    root = tmp_path
    (root / ".conductor" / "tracks" / "frontier-rust-convergence-20261113").mkdir(parents=True)
    (root / ".conductor" / "tracks.md").write_text(
        "- [~] **Track: frontier-rust-convergence-20261113**\n", encoding="utf-8"
    )
    track = root / ".conductor" / "tracks" / "frontier-rust-convergence-20261113"
    (track / "spec.md").write_text("# Spec\n", encoding="utf-8")
    (track / "plan.md").write_text("# Plan\n", encoding="utf-8")
    (track / "metadata.json").write_text(
        json.dumps({"track_id": track.name, "status": "in_progress"}),
        encoding="utf-8",
    )
    (root / "pyproject.toml").write_text(
        '[project]\nrequires-python = ">=3.11"\n[project.scripts]\nfyi = "fyi_system.cli:main"\n',
        encoding="utf-8",
    )
    (root / "crates").mkdir()
    for name in ("fyi-cli", "fyi-core", "fyi-mcp"):
        crate = root / "crates" / name
        crate.mkdir()
        (crate / "Cargo.toml").write_text(
            '[package]\nname = "' + name + '"\nrust-version = "1.99"\n',
            encoding="utf-8",
        )
    return root


def test_valid_frontier_contract(tmp_path: Path) -> None:
    repo = make_repo(tmp_path)
    assert evaluate(repo) == []


def test_missing_registered_spec_is_a_failing_error(tmp_path: Path) -> None:
    repo = make_repo(tmp_path)
    (repo / ".conductor" / "tracks" / "frontier-rust-convergence-20261113" / "spec.md").unlink()
    assert any("spec.md" in e for e in evaluate(repo))


def test_reject_inconsistent_track_id(tmp_path: Path) -> None:
    repo = make_repo(tmp_path)
    path = repo / ".conductor" / "tracks" / "frontier-rust-convergence-20261113" / "metadata.json"
    path.write_text('{"track_id":"wrong","status":"in_progress"}', encoding="utf-8")
    assert any("track_id" in e for e in evaluate(repo))


def test_reject_missing_rust_msrv(tmp_path: Path) -> None:
    repo = make_repo(tmp_path)
    path = repo / "crates" / "fyi-core" / "Cargo.toml"
    path.write_text('[package]\nname = "fyi-core"\n', encoding="utf-8")
    assert any("rust-version" in e for e in evaluate(repo))


def test_inventory_finds_actual_python_entrypoints(tmp_path: Path) -> None:
    repo = make_repo(tmp_path)
    (repo / "src" / "fyi_system").mkdir(parents=True)
    (repo / "src" / "fyi_system" / "cli.py").write_text("def main(): pass\n", encoding="utf-8")
    result = inventory_legacy_python(repo)
    assert result["entrypoints"] == {"fyi": "fyi_system.cli:main"}
    assert "src/fyi_system/cli.py" in result["production_python_files"]


def test_inventory_excludes_caches(tmp_path: Path) -> None:
    repo = make_repo(tmp_path)
    (repo / "src" / "fyi_system" / "__pycache__").mkdir(parents=True)
    (repo / "src" / "fyi_system" / "__pycache__" / "thing.py").write_text("")
    assert not inventory_legacy_python(repo)["production_python_files"]
