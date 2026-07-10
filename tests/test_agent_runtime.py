"""Tests for Python agent_runtime good-citizen helpers."""

from __future__ import annotations

import json
from datetime import datetime, timedelta, timezone
from email.utils import format_datetime
from pathlib import Path

import pytest

from fyi_system.agent_runtime import (
    ClientIdentity,
    FilesystemResponseCache,
    GuardrailConfig,
    GuardrailTracker,
    JsonlTraceSink,
    RateLimitSnapshot,
    RetrievalPlan,
    build_user_agent,
    exponential_backoff_seconds,
    is_generic_user_agent,
    redact_secrets,
    reflect_plan,
    retry_delay_seconds,
)


def test_default_identity_traceable():
    ua = build_user_agent()
    assert ua.startswith("fyi-cli/")
    assert "fp:" in ua
    assert "github.com/edithatogo/fyi-cli" in ua
    assert "contact:" not in ua
    assert not is_generic_user_agent(ua)


def test_opt_in_contact():
    ua = build_user_agent("ops@example.org")
    assert "contact:ops@example.org" in ua


def test_rejects_generic():
    assert is_generic_user_agent("")
    assert is_generic_user_agent("curl/8.0")
    assert is_generic_user_agent("python-requests/2.32")


def test_rate_limit_headers():
    snap = RateLimitSnapshot.from_headers(
        {
            "RateLimit-Limit": "100",
            "RateLimit-Remaining": "2",
            "Retry-After": "15",
        }
    )
    assert snap.limit == 100
    assert snap.remaining == 2
    assert snap.retry_after_seconds == 15


def test_retry_after_http_date_is_parsed_as_delay():
    retry_at = datetime.now(timezone.utc) + timedelta(seconds=20)
    snap = RateLimitSnapshot.from_headers({"Retry-After": format_datetime(retry_at, usegmt=True)})
    assert snap.retry_after_seconds is not None
    assert 0 <= snap.retry_after_seconds <= 20


def test_retry_delay_prefers_server_header():
    assert retry_delay_seconds({"Retry-After": "40"}, attempt=0, max_seconds=10) >= 40


def test_backoff_honours_retry_after():
    assert exponential_backoff_seconds(1, retry_after=40, max_seconds=30) >= 40


def test_guardrails_max_requests():
    g = GuardrailTracker(GuardrailConfig(max_requests=2, max_response_bytes=1000, max_runtime_seconds=60))
    g.record_request_start()
    g.record_request_start()
    with pytest.raises(RuntimeError, match="maximum request count"):
        g.record_request_start()


def test_plan_reject_unbounded():
    decision = reflect_plan(
        RetrievalPlan(
            instance_id="nz-fyi",
            description="all",
            recursive_unbounded=True,
        )
    )
    assert decision["decision"] == "reject"


def test_plan_rewrite_with_window():
    decision = reflect_plan(
        RetrievalPlan(
            instance_id="nz-fyi",
            description="windowed",
            recursive_unbounded=True,
            date_from="2020-01-01",
            date_to="2020-01-31",
        )
    )
    assert decision["decision"] == "rewrite"
    assert decision["rewritten"]["recursive_unbounded"] is False


def test_filesystem_cache(tmp_path: Path):
    cache = FilesystemResponseCache(tmp_path / "cache")
    url = "https://fyi.org.nz/request/1"
    assert cache.get(url) is None
    cache.put(url, b"body")
    assert cache.get(url) == b"body"


def test_trace_jsonl_and_redaction(tmp_path: Path):
    sink = JsonlTraceSink(tmp_path / "trace.jsonl", run_id="t1")
    sink.emit("http.request", metadata={"api_key": "secret", "ok": True})
    lines = (tmp_path / "trace.jsonl").read_text(encoding="utf-8").strip().splitlines()
    assert len(lines) == 1
    event = json.loads(lines[0])
    assert event["name"] == "http.request"
    assert event["metadata"]["api_key"] == "[redacted]"
    assert event["metadata"]["ok"] is True


def test_redact_secrets_string():
    assert redact_secrets("api_key=secret-token") == "[redacted]"


def test_identity_validate_requires_homepage():
    with pytest.raises(ValueError):
        ClientIdentity(
            product="fyi-cli",
            version="1.0",
            fingerprint="abcdabcdabcdabcd",
            homepage="not-a-url",
        ).validate()
