"""Portable Conductor contract checker and Python migration inventory.

Pure stdlib. Read-only; emits deterministic reports and never changes track state.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
import tomllib

TRACK = "frontier-rust-convergence-20261113"
CRATES = ("fyi-cli", "fyi-core", "fyi-mcp")
EXCLUDED_DIRS = {"__pycache__", ".venv", "build", "dist", ".git"}


def inventory_legacy_python(repo: Path) -> dict[str, object]:
    """Inventory production Python and published Python command entrypoints."""
    pyproject = repo / "pyproject.toml"
    with pyproject.open("rb") as handle:
        project = tomllib.load(handle).get("project", {})
    scripts = project.get("scripts", {})
    root = repo / "src" / "fyi_system"
    files = (
        sorted(
            p.relative_to(repo).as_posix()
            for p in root.rglob("*.py")
            if not any(part in EXCLUDED_DIRS for part in p.relative_to(root).parts)
        )
        if root.exists()
        else []
    )
    return {
        "entrypoints": dict(sorted(scripts.items())),
        "production_python_files": files,
        "requires_python": project.get("requires-python", "unspecified"),
        "disposition": "migration_in_progress",
    }


def evaluate(repo: Path) -> list[str]:
    """Check only the scoped frontier contract; legacy tracks are reported separately."""
    errors: list[str] = []
    conductor = repo / ".conductor"
    track = conductor / "tracks" / TRACK
    registry = conductor / "tracks.md"
    if not registry.is_file():
        errors.append("missing .conductor/tracks.md")
    elif registry.read_text(encoding="utf-8").count("**Track: " + TRACK + "**") != 1:
        errors.append("frontier track must have exactly one registry entry")
    for name in ("spec.md", "plan.md", "metadata.json"):
        if not (track / name).is_file():
            errors.append(f"missing track/{name}")
    metadata = track / "metadata.json"
    if metadata.is_file():
        try:
            parsed = json.loads(metadata.read_text(encoding="utf-8"))
        except (ValueError, UnicodeError) as exc:
            errors.append(f"invalid track metadata: {exc}")
        else:
            if parsed.get("track_id") != TRACK:
                errors.append("metadata track_id differs from directory")
            if parsed.get("status") not in ("in_progress", "completed", "blocked"):
                errors.append("metadata status must be normalized")
            if parsed.get("status") == "completed" and not parsed.get("verification"):
                errors.append("completed track requires verification evidence")
    for crate in CRATES:
        manifest = repo / "crates" / crate / "Cargo.toml"
        if not manifest.is_file():
            errors.append(f"missing {crate}/Cargo.toml")
            continue
        try:
            with manifest.open("rb") as handle:
                package = tomllib.load(handle).get("package", {})
            if package.get("rust-version") != "1.99":
                errors.append(f"{crate}: rust-version must equal 1.99")
        except (ValueError, OSError) as exc:
            errors.append(f"{crate}: invalid manifest: {exc}")
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--json", action="store_true", help="print machine-readable report")
    parser.add_argument("--inventory", action="store_true", help="include Python migration inventory")
    args = parser.parse_args(argv)
    errors = evaluate(args.root)
    report: dict[str, object] = {"track": TRACK, "valid": not errors, "errors": errors}
    if args.inventory:
        report["python_migration"] = inventory_legacy_python(args.root)
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(f"Frontier Conductor contract: {'PASS' if not errors else 'FAIL'}")
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
