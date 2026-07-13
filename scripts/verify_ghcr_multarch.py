#!/usr/bin/env python3
"""Verify that a GHCR tag exposes both required Linux architectures."""

from __future__ import annotations

import argparse
import json
import subprocess


REQUIRED_PLATFORMS = {"linux/amd64", "linux/arm64"}


def parse_inspect_output(output: str) -> set[str]:
    platforms: set[str] = set()
    for line in output.splitlines():
        value = line.strip().replace(" ", "")
        if value.startswith("linux/"):
            platforms.add(value)
    return platforms


def verify(image: str, tag: str) -> dict[str, object]:
    reference = f"{image}:{tag}"
    completed = subprocess.run(
        ["docker", "buildx", "imagetools", "inspect", reference],
        check=False,
        capture_output=True,
        text=True,
        timeout=30,
    )
    if completed.returncode:
        return {"image": reference, "status": "unverified", "error": "inspect_failed", "exit_code": completed.returncode}
    platforms = parse_inspect_output(completed.stdout)
    missing = sorted(REQUIRED_PLATFORMS - platforms)
    return {"image": reference, "status": "verified" if not missing else "incomplete", "platforms": sorted(platforms), "missing": missing}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--image", default="ghcr.io/edithatogo/fyi-mcp")
    parser.add_argument("--tag", required=True)
    args = parser.parse_args()
    try:
        report = verify(args.image, args.tag)
    except (FileNotFoundError, subprocess.TimeoutExpired):
        report = {"image": f"{args.image}:{args.tag}", "status": "unverified", "error": "docker_unavailable_or_timeout"}
    print(json.dumps(report, sort_keys=True))
    return 0 if report["status"] == "verified" else 1


if __name__ == "__main__":
    raise SystemExit(main())
