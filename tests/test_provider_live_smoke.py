import json

import pytest

from scripts.provider_live_smoke import schema_fingerprint, validate_response


def test_live_smoke_is_disabled_without_explicit_opt_in(monkeypatch, capsys):
    monkeypatch.delenv("FYI_PROVIDER_LIVE_SMOKE", raising=False)
    from scripts.provider_live_smoke import run

    assert run("muckrock", live=False) == 0
    assert json.loads(capsys.readouterr().out)["status"] == "disabled"


def test_schema_fingerprint_is_stable_and_values_do_not_change_it():
    left = {"results": [{"id": 1, "title": "a"}], "count": 1}
    right = {"count": 99, "results": [{"id": 2, "title": "b"}]}
    assert schema_fingerprint(left) == schema_fingerprint(right)


def test_muckrock_schema_requires_request_fields():
    with pytest.raises(ValueError, match="required fields"):
        validate_response("muckrock", {"results": [{"id": 1}]})


def test_fragdenstaat_accepts_public_request_shape():
    report = validate_response("fragdenstaat", {"results": [{"id": "abc-1", "subject": "Akten"}]})
    assert report["status"] == "ok"
