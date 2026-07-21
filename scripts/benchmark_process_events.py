"""Bounded offline benchmark for the public-safe process-event exporter."""

from __future__ import annotations

import argparse
import json
import sys
import tempfile
import time
from pathlib import Path

from fyi_system.process_events import export_process_events


def build_fixture(root: Path, request_count: int, events_per_request: int) -> Path:
    derived = root / "derived"
    for request_id in range(1, request_count + 1):
        path = derived / "Agency" / str(request_id)
        path.mkdir(parents=True, exist_ok=True)
        events = [
            {
                "id": f"event-{request_id}-{index}",
                "event_type": "opened" if index == 0 else "observed",
                "created_at": "2026-01-01T00:00:00Z",
            }
            for index in range(events_per_request)
        ]
        path.joinpath("request.json").write_text(
            json.dumps({"id": request_id, "info_request_events": events}),
            encoding="utf-8",
        )
    return derived


def run(request_count: int, events_per_request: int) -> dict[str, int | float]:
    with tempfile.TemporaryDirectory(prefix="fyi-process-events-benchmark-") as directory:
        root = Path(directory)
        derived = build_fixture(root, request_count, events_per_request)
        started = time.perf_counter()
        result = export_process_events(
            derived_dir=derived,
            output=root / "events.ndjson",
            captured_at="2026-02-01T00:00:00Z",
            checkpoint=root / "checkpoint.json",
        )
        elapsed = time.perf_counter() - started
        return {
            "request_count": request_count,
            "events_per_request": events_per_request,
            "event_count": result["total_event_count"],
            "elapsed_seconds": round(elapsed, 4),
            "events_per_second": round(result["total_event_count"] / elapsed, 2) if elapsed else 0,
        }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--requests", type=int, default=1000)
    parser.add_argument("--events-per-request", type=int, default=8)
    args = parser.parse_args()
    if args.requests < 1 or args.events_per_request < 1:
        parser.error("requests and events-per-request must be positive")
    sys.stdout.write(json.dumps(run(args.requests, args.events_per_request), sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
