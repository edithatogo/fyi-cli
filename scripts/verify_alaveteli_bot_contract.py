"""Run the offline fyi-cli side of the fork-local Alaveteli bot contract."""

from __future__ import annotations

import json
import os
from pathlib import Path

from fyi_system.agent_runtime import RateLimitSnapshot
from fyi_system.endorsed_route import CapabilityDocument, EndorsedRouteError

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    route_payload = json.loads(
        (ROOT / "tests/fixtures/endorsed-client-route/enabled.json").read_text()
    )
    route = CapabilityDocument.from_mapping(route_payload)
    authorized = route.authorize(
        client_id="fyi-cli-prod",
        scopes=("read", "bulk_export"),
        now_epoch=1_700_000_000,
        bulk_export=True,
    )
    disabled = dict(route_payload, enabled=False)
    try:
        CapabilityDocument.from_mapping(disabled).authorize(
            client_id="fyi-cli-prod",
            scopes=("read",),
            now_epoch=1_700_000_000,
        )
    except EndorsedRouteError:
        pass
    else:
        raise AssertionError("disabled endorsed route did not fail closed")

    headers = json.loads((ROOT / "tests/fixtures/backpressure_headers.json").read_text())
    for case in headers.values():
        snapshot = RateLimitSnapshot.from_headers(case["headers"])
        expected = case["expected"]
        assert snapshot.remaining == expected["remaining"]
        assert snapshot.advisory_status == expected["advisory_status"]

    live = os.environ.get("FYI_ALAVETELI_CONTRACT_LIVE") == "1"
    report = {
        "mode": "bounded-live-opt-in" if live else "offline",
        "route": authorized["instance_id"],
        "bulk_max_items": authorized["bulk_export"].max_items,
        "backpressure_cases": len(headers),
        "live_smoke": "requested; run only with an authorized test cohort"
        if live
        else "disabled by default",
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
