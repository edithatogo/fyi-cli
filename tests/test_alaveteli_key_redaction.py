"""Alaveteli authority keys must not escape through errors or traces."""

from __future__ import annotations

import traceback
from typing import TYPE_CHECKING

import pytest
import requests

from fyi_system.agent_runtime import JsonlTraceSink, redact_secrets
from fyi_system.alaveteli_client import AlaveteliAPIError, AlaveteliClient
from fyi_system.security import redact_text, sanitize_payload

if TYPE_CHECKING:
    from pathlib import Path


def test_authority_key_is_redacted_from_urls_and_trace_metadata(tmp_path: Path) -> None:
    marker = "synthetic-private-value"
    url = f"https://example.invalid/request?k={marker}&page=2"
    assert marker not in redact_text(url)
    assert "page=2" in redact_text(url)
    assert marker not in redact_text(url.replace("?k=", "?%6B="))
    assert marker not in redact_text(f"https://[invalid/?k={marker}")
    redacted = redact_secrets({"url": url, "nested": {"k": marker}})
    assert marker not in str(redacted)
    assert marker not in str(redact_secrets(url.replace("?k=", "?%6B=")))
    assert redacted["nested"]["k"] == "[redacted]"
    assert sanitize_payload({"nested": {"k": marker}})["nested"]["k"] == "[redacted]"
    sink = JsonlTraceSink(tmp_path / "trace.jsonl", run_id="synthetic")
    sink.emit("http.request", metadata={"url": url, "nested": {"k": marker}})
    assert marker not in (tmp_path / "trace.jsonl").read_text(encoding="utf-8")


@pytest.mark.parametrize("method", ["get", "post"])
@pytest.mark.parametrize("http_failure", [False, True])
def test_api_errors_do_not_expose_authority_key(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    method: str,
    *,
    http_failure: bool,
) -> None:
    marker = "synthetic-private-value"
    client = AlaveteliClient(
        base_url="https://example.invalid",
        api_key=marker,
        db_path=tmp_path / "cache.sqlite3",
    )
    url = f"https://example.invalid/api/v2/request.json?k={marker}"
    response = requests.Response()
    response.status_code = 403
    response.url = url
    if http_failure:
        error = requests.exceptions.HTTPError(f"403 for {url}", response=response)
    else:
        error = requests.exceptions.ConnectionError(f"connection failed for {url}")

    def fail(*_args: object, **_kwargs: object) -> None:
        raise error

    monkeypatch.setattr(client.session, method, fail)
    with pytest.raises(AlaveteliAPIError) as caught:
        getattr(client, f"_{method}")("/api/v2/request.json")
    assert marker not in "".join(traceback.format_exception(caught.value))
    assert caught.value.status_code == (403 if http_failure else None)


@pytest.mark.parametrize("method", ["get", "post"])
def test_http_response_errors_keep_status_without_the_request_url(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    method: str,
) -> None:
    marker = "synthetic-private-value"
    client = AlaveteliClient(
        base_url="https://example.invalid",
        api_key=marker,
        db_path=tmp_path / "cache.sqlite3",
    )
    response = requests.Response()
    response.status_code = 403
    response.url = f"https://example.invalid/request?k={marker}"
    monkeypatch.setattr(client.session, method, lambda *_args, **_kwargs: response)
    with pytest.raises(AlaveteliAPIError) as caught:
        getattr(client, f"_{method}")("/api/v2/request.json")
    assert caught.value.status_code == 403
    assert marker not in "".join(traceback.format_exception(caught.value))
