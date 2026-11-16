"""Validate versioned API contract JSON, RSS, Atom, and error fixtures."""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET
from pathlib import Path

FIXTURES = Path(__file__).resolve().parents[1] / "tests/fixtures/api_contracts"


def test_rss_and_atom_feed_fixtures_are_well_formed_xml() -> None:
    rss_root = ET.fromstring((FIXTURES / "feed-rss-v1.xml").read_text(encoding="utf-8"))
    assert rss_root.tag == "rss"
    links = [elem.text for elem in rss_root.findall("./channel/item/link") if elem.text]
    assert any("/request/1001/" in link for link in links)
    assert any("/request/1002/" in link for link in links)

    atom_root = ET.fromstring((FIXTURES / "feed-atom-v1.xml").read_text(encoding="utf-8"))
    assert atom_root.tag.endswith("feed")
    ns = {"atom": "http://www.w3.org/2005/Atom"}
    ids = [elem.text for elem in atom_root.findall("./atom:entry/atom:id", ns) if elem.text]
    assert any("/requests/2001" in entry_id for entry_id in ids)
    assert any("/requests/2002" in entry_id for entry_id in ids)


def test_error_fixtures_declare_expected_status_and_error_keys() -> None:
    expected = {
        "error-401-unauthorized.json": 401,
        "error-403-forbidden.json": 403,
        "error-404-not-found.json": 404,
        "error-503-unavailable.json": 503,
    }
    for filename, status in expected.items():
        payload = json.loads((FIXTURES / filename).read_text(encoding="utf-8"))
        assert payload["status"] == status
        assert isinstance(payload["error"], str) and payload["error"]
        assert isinstance(payload["message"], str) and payload["message"]
