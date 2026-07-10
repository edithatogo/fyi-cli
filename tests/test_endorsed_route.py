import json
from pathlib import Path

import pytest

from fyi_system.endorsed_route import (
    CapabilityDocument,
    EndorsedRouteError,
)

FIXTURE = Path(__file__).parent / "fixtures" / "endorsed-client-route" / "enabled.json"


def document() -> CapabilityDocument:
    return CapabilityDocument.from_mapping(json.loads(FIXTURE.read_text()))


def test_authorizes_allowlisted_scoped_bounded_bulk_route():
    authorized = document().authorize(
        client_id="fyi-cli-prod",
        scopes=("read", "bulk_export"),
        now_epoch=1_700_000_000,
        bulk_export=True,
    )
    assert authorized["instance_id"] == "nz-fyi"
    assert authorized["bulk_export"].max_items == 1000


@pytest.mark.parametrize(
    ("field", "value", "message"),
    [
        ("enabled", False, "disabled"),
        ("kill_switch", True, "kill switch"),
        ("revoked", True, "revoked"),
        ("expires_at", 1_700_000_000, "expired"),
    ],
)
def test_disablement_and_expiry_fail_closed(field, value, message):
    payload = json.loads(FIXTURE.read_text())
    payload[field] = value
    with pytest.raises(EndorsedRouteError, match=message):
        CapabilityDocument.from_mapping(payload).authorize(
            client_id="fyi-cli-prod",
            scopes=("read",),
            now_epoch=1_700_000_000,
        )


def test_unknown_client_and_bulk_scope_are_rejected():
    with pytest.raises(EndorsedRouteError, match="allowlist"):
        document().authorize(
            client_id="unknown",
            scopes=("read",),
            now_epoch=1_700_000_000,
        )
    with pytest.raises(EndorsedRouteError, match="bulk export scope"):
        document().authorize(
            client_id="fyi-cli-prod",
            scopes=("read",),
            now_epoch=1_700_000_000,
            bulk_export=True,
        )


@pytest.mark.parametrize(
    ("field", "value"),
    [("enabled", "false"), ("kill_switch", 1), ("expires_at", True)],
)
def test_malformed_types_and_unknown_fields_are_rejected(field, value):
    payload = json.loads(FIXTURE.read_text())
    payload[field] = value
    with pytest.raises(EndorsedRouteError, match="malformed"):
        CapabilityDocument.from_mapping(payload)

    payload = json.loads(FIXTURE.read_text())
    payload["unexpected"] = True
    with pytest.raises(EndorsedRouteError, match="malformed"):
        CapabilityDocument.from_mapping(payload)


def test_empty_scope_is_rejected():
    with pytest.raises(EndorsedRouteError, match="invalid"):
        document().authorize(
            client_id="fyi-cli-prod",
            scopes=("",),
            now_epoch=1_700_000_000,
        )
