#!/usr/bin/env python3
"""Bounded, opt-in public-endpoint smoke checks for non-Alaveteli providers."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path
from urllib.request import Request, urlopen

MAX_BYTES = 1_000_000
TIMEOUT_SECONDS = 10
USER_AGENT = "fyi-cli-provider-smoke/0.1 (+https://github.com/edithatogo/fyi-cli/issues)"
PROVIDERS = {
    "muckrock": "https://www.muckrock.com/api_v2/requests/?format=json&page_size=1",
    "fragdenstaat": "https://fragdenstaat.de/api/v1/request/?limit=1",
}


def schema_fingerprint(value: object) -> str:
    """Hash a stable JSON shape, excluding values and volatile list contents."""

    def shape(item: object) -> object:
        if isinstance(item, dict):
            return {key: shape(item[key]) for key in sorted(item)}
        if isinstance(item, list):
            return [shape(item[0])] if item else []
        return type(item).__name__

    canonical = json.dumps(shape(value), separators=(",", ":"), sort_keys=True)
    return hashlib.sha256(canonical.encode()).hexdigest()


def fetch_json(url: str) -> object:
    request = Request(url, headers={"Accept": "application/json", "User-Agent": USER_AGENT})
    with urlopen(request, timeout=TIMEOUT_SECONDS) as response:  # nosec B310: fixed HTTPS URLs only
        body = response.read(MAX_BYTES + 1)
    if len(body) > MAX_BYTES:
        raise ValueError(f"response exceeds {MAX_BYTES} bytes")
    return json.loads(body)


def validate_response(provider: str, payload: object) -> dict[str, object]:
    if not isinstance(payload, dict):
        raise ValueError("response must be a JSON object")
    results = payload.get("results")
    if not isinstance(results, list):
        raise ValueError("response is missing a results array")
    if results:
        item = results[0]
        if not isinstance(item, dict):
            raise ValueError("results[0] must be a JSON object")
        required = {"muckrock": {"id", "title"}, "fragdenstaat": {"id"}}[provider]
        missing = sorted(required - item.keys())
        if missing:
            raise ValueError(f"response schema missing required fields: {', '.join(missing)}")
    return {"provider": provider, "status": "ok", "schema_fingerprint": schema_fingerprint(payload), "sample_count": len(results)}


def run(provider: str, *, live: bool) -> int:
    if not live and os.environ.get("FYI_PROVIDER_LIVE_SMOKE") != "1":
        print(json.dumps({"status": "disabled", "provider": provider}))
        return 0
    try:
        report = validate_response(provider, fetch_json(PROVIDERS[provider]))
    except Exception as exc:  # bounded operator sensor: never emit response bodies
        print(json.dumps({"provider": provider, "status": "failed", "error": type(exc).__name__}))
        return 2
    print(json.dumps(report, sort_keys=True))
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("provider", choices=sorted(PROVIDERS))
    parser.add_argument("--live", action="store_true", help="explicitly enable the bounded public smoke")
    args = parser.parse_args(argv)
    return run(args.provider, live=args.live)


if __name__ == "__main__":
    raise SystemExit(main())
