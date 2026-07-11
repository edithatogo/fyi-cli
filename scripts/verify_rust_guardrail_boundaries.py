#!/usr/bin/env python3
"""Fail-closed sensor for Rust outbound send-boundary guardrails."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SYNC = ROOT / "crates/fyi-core/src/sync.rs"
TOR = ROOT / "crates/fyi-core/src/tor.rs"


def validate_boundaries() -> list[str]:
    errors: list[str] = []
    sync = SYNC.read_text(encoding="utf-8")
    tor = TOR.read_text(encoding="utf-8")

    required_sync = {
        "send_guarded": ["before_request", "acquire_owned", ".execute(request)", "after_response"],
        "response_byte_accounting": ["record_response_chunk", "max_response_bytes"],
    }
    required_tor = {
        "tor_execute": ["before_request", "acquire_owned", ".execute(request)", "after_response"],
    }
    for boundary, needles in required_sync.items():
        for needle in needles:
            if needle not in sync:
                errors.append(f"{boundary}: missing {needle}")
    for boundary, needles in required_tor.items():
        for needle in needles:
            if needle not in tor:
                errors.append(f"{boundary}: missing {needle}")

    # These are the only production Rust send sites; direct reqwest calls in
    # api.rs are test fixtures and must remain under cfg(test).
    if sync.count(".execute(request)") != 1:
        errors.append("sync: unexpected number of direct request execution sites")
    if tor.count(".execute(request)") != 1:
        errors.append("tor: unexpected number of direct request execution sites")
    api = (ROOT / "crates/fyi-core/src/api.rs").read_text(encoding="utf-8")
    production_api = api.split("#[cfg(test)]", 1)[0]
    if "reqwest::Client::new()" in production_api:
        errors.append("api: direct reqwest fixture is not visibly test-scoped")
    return errors


def main() -> int:
    errors = validate_boundaries()
    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1
    print("Rust outbound guardrail boundaries valid: SyncClient and TorAgentClient")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
