#!/usr/bin/env python3
"""Create a deterministic release manifest and SHA-256 inventory."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


def create_manifest(assets: Path, version: str, commit: str) -> dict:
    files = []
    for path in sorted(p for p in assets.iterdir() if p.is_file()):
        if path.name in {"release-manifest.json", "SHA256SUMS"}:
            continue
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        files.append({"name": path.name, "sha256": digest, "size": path.stat().st_size})
    return {"schema_version": 1, "version": version, "commit": commit, "files": files}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("assets", type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    args = parser.parse_args()
    if not args.assets.is_dir():
        parser.error(f"assets directory does not exist: {args.assets}")
    manifest = create_manifest(args.assets, args.version, args.commit)
    (args.assets / "release-manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (args.assets / "SHA256SUMS").write_text(
        "".join(f"{item['sha256']}  {item['name']}\n" for item in manifest["files"]),
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
