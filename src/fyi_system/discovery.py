"""Read-only discovery helpers for public FYI/Alaveteli requests."""

from __future__ import annotations

import json
import os
import random
import re
import time
from contextlib import suppress
from dataclasses import asdict, dataclass
from typing import TYPE_CHECKING, Any
from urllib.parse import urlencode

import httpx

from fyi_system.acquisition_receipts import observe_response

from .agent_runtime import build_user_agent, retry_delay_seconds
from .db import (
    acquire_shared_rate_limit,
    read_shared_rate_limit_events,
    read_shared_rate_limit_state,
    record_shared_rate_limit_backoff,
)

if TYPE_CHECKING:
    from collections.abc import Iterable
    from pathlib import Path


# Cryptographic-aligned, contactable identity (opt-in admin contact via FYI_ADMIN_CONTACT).
USER_AGENT = build_user_agent(os.environ.get("FYI_ADMIN_CONTACT"))
REQUEST_RE = re.compile(r"/request/(?P<id>\d+|[a-z0-9_-]+)", re.IGNORECASE)


@dataclass(frozen=True)
class DiscoveredRequest:
    """Request discovered from a feed or ID probe."""

    request_id: int
    url_title: str
    title: str = ""
    authority: str = ""
    state: str = ""
    created_at: str = ""

    def to_json(self) -> str:
        """Serialize as one JSONL row."""
        return json.dumps(asdict(self), sort_keys=True, ensure_ascii=False)


@dataclass(frozen=True)
class DiscoveryReconciliation:
    """Comparison of feed-discovered and ID-backfilled request sets."""

    feed_count: int
    backfill_count: int
    matched_count: int
    missing_from_feed: list[int]
    missing_from_backfill: list[int]

    @property
    def is_complete(self) -> bool:
        """Return true when both discovery methods found the same request IDs."""
        return not self.missing_from_feed and not self.missing_from_backfill

    def to_dict(self) -> dict[str, Any]:
        """Serialize the reconciliation report."""
        return {
            "feed_count": self.feed_count,
            "backfill_count": self.backfill_count,
            "matched_count": self.matched_count,
            "missing_from_feed": self.missing_from_feed,
            "missing_from_backfill": self.missing_from_backfill,
            "is_complete": self.is_complete,
        }


class PoliteRateLimiter:
    """Simple per-process rate limiter for polite archive discovery."""

    def __init__(
        self,
        interval_seconds: float,
        *,
        jitter_seconds: float = 0.25,
        clock: Any = time.monotonic,
        sleeper: Any = time.sleep,
        randomizer: Any = random.random,
    ) -> None:
        self.interval_seconds = max(interval_seconds, 0)
        self.jitter_seconds = max(jitter_seconds, 0)
        self.clock = clock
        self.sleeper = sleeper
        self.randomizer = randomizer
        self.next_allowed_at: float | None = None

    def wait(self) -> None:
        """Sleep until the next request is allowed."""
        now = float(self.clock())
        if self.next_allowed_at is not None and now < self.next_allowed_at:
            sleep_for = self.next_allowed_at - now
            if sleep_for > 0:
                self.sleeper(sleep_for)
                now = float(self.clock())
        jitter = self.jitter_seconds * float(self.randomizer()) if self.interval_seconds > 0 else 0
        self.next_allowed_at = now + self.interval_seconds + jitter


@dataclass(frozen=True)
class SharedRateLimitSnapshot:
    """Snapshot of a shared rate limit state row."""

    name: str
    next_allowed_at: float
    last_acquired_at: str
    last_owner_id: str
    last_sleep_seconds: float
    interval_seconds: float
    jitter_seconds: float
    recent_events: list[dict[str, Any]]

    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "next_allowed_at": self.next_allowed_at,
            "last_acquired_at": self.last_acquired_at,
            "last_owner_id": self.last_owner_id,
            "last_sleep_seconds": self.last_sleep_seconds,
            "interval_seconds": self.interval_seconds,
            "jitter_seconds": self.jitter_seconds,
            "recent_events": self.recent_events,
        }


class SharedRateLimiter:
    """Cross-process limiter backed by a SQLite coordination row."""

    def __init__(
        self,
        db_path: str | Path,
        *,
        name: str = "archive-discovery",
        jitter_seconds: float = 0.25,
        owner_id: str | None = None,
        sleeper: Any = time.sleep,
        randomizer: Any = random.random,
    ) -> None:
        self.db_path = db_path
        self.name = name
        self.jitter_seconds = max(jitter_seconds, 0)
        self.owner_id = owner_id or f"pid:{os.getpid()}"
        self.sleeper = sleeper
        self.randomizer = randomizer
        self.last_acquire: dict[str, Any] | None = None

    def wait(self, interval_seconds: float) -> None:
        """Reserve and sleep for the next shared slot."""
        state = acquire_shared_rate_limit(
            self.db_path,
            name=self.name,
            interval_seconds=interval_seconds,
            jitter_seconds=self.jitter_seconds,
            owner_id=self.owner_id,
            randomizer=self.randomizer,
        )
        self.last_acquire = state
        sleep_for = float(state["sleep_seconds"])
        if sleep_for > 0:
            self.sleeper(sleep_for)

    def backoff(self, delay_seconds: float, *, status_code: int | None = None) -> dict[str, Any]:
        """Advance the shared limiter after a transient response."""
        return record_shared_rate_limit_backoff(
            self.db_path,
            name=self.name,
            delay_seconds=delay_seconds,
            jitter_seconds=self.jitter_seconds,
            owner_id=self.owner_id,
            status_code=status_code,
            randomizer=self.randomizer,
        )

    def snapshot(self) -> SharedRateLimitSnapshot | None:
        """Read the current shared limiter state from SQLite."""
        row = read_shared_rate_limit_state(self.db_path, name=self.name)
        if row is None:
            return None
        events = read_shared_rate_limit_events(self.db_path, name=self.name)
        return SharedRateLimitSnapshot(
            name=str(row["name"]),
            next_allowed_at=float(row["next_allowed_at"]),
            last_acquired_at=str(row["last_acquired_at"]),
            last_owner_id=str(row["last_owner_id"]),
            last_sleep_seconds=float(row["last_sleep_seconds"]),
            interval_seconds=float(row["interval_seconds"]),
            jitter_seconds=float(row["jitter_seconds"]),
            recent_events=events,
        )


def build_search_url(
    *,
    base_url: str,
    date_from: str | None = None,
    date_to: str | None = None,
    authority: str | None = None,
    status: str | None = None,
    page: int = 1,
) -> str:
    """Build an Alaveteli search-feed JSON URL."""
    params = {"output": "json", "page": str(page)}
    if date_from:
        params["requested_after"] = date_from
    if date_to:
        params["requested_before"] = date_to
    if authority:
        params["public_body"] = authority
    if status:
        params["latest_status"] = status
    return f"{base_url.rstrip('/')}/search/all?{urlencode(params)}"


def client(base_url: str, *, transport: httpx.BaseTransport | None = None) -> httpx.Client:
    """Create a polite read-only HTTP client."""
    return httpx.Client(
        base_url=base_url.rstrip("/"),
        follow_redirects=True,
        headers={"User-Agent": USER_AGENT},
        timeout=60,
        transport=transport,
    )


def parse_robots_disallow(robots_txt: str) -> list[str]:
    """Parse simple robots.txt Disallow directives for all user agents."""
    disallows = []
    applies = False
    for raw_line in robots_txt.splitlines():
        line = raw_line.split("#", 1)[0].strip()
        if not line:
            continue
        key, _, value = line.partition(":")
        key = key.strip().lower()
        value = value.strip()
        if key == "user-agent":
            applies = value == "*"
        elif applies and key == "disallow" and value:
            disallows.append(value)
    return disallows


def robots_allows(path: str, disallows: list[str]) -> bool:
    """Return false when a path is disallowed by robots.txt."""
    return not any(path.startswith(rule) for rule in disallows if rule != "/")


def load_robots_disallow(http: httpx.Client, *, recorder: Any | None = None) -> list[str]:
    """Fetch robots.txt disallow rules, failing open when robots is unavailable."""
    response = http.get("/robots.txt")
    observe_response(recorder, response)
    if response.status_code >= 400:
        return []
    return parse_robots_disallow(response.text)


def _wait_with_shared_fallback(
    shared_rate_limiter: SharedRateLimiter | None,
    rate_limiter: PoliteRateLimiter | None,
    interval_seconds: float,
) -> None:
    if shared_rate_limiter is not None:
        with suppress(Exception):
            shared_rate_limiter.wait(interval_seconds)
            return
    if rate_limiter is not None:
        rate_limiter.wait()


def _record_shared_backoff(
    shared_rate_limiter: SharedRateLimiter | None,
    *,
    delay_seconds: float,
    status_code: int,
) -> None:
    if shared_rate_limiter is None:
        return
    with suppress(Exception):
        shared_rate_limiter.backoff(delay_seconds, status_code=status_code)


def get_with_backoff(
    http: httpx.Client,
    url: str,
    *,
    disallows: list[str],
    shared_rate_limiter: SharedRateLimiter | None = None,
    rate_limiter: PoliteRateLimiter | None = None,
    retries: int = 3,
    backoff_seconds: float = 1.0,
    sleeper: Any = time.sleep,
) -> httpx.Response:
    """GET with robots enforcement and bounded status/transport retries."""
    path = httpx.URL(url).path if url.startswith("http") else url
    if not robots_allows(path, disallows):
        msg = f"robots.txt disallows fetching {path}"
        raise PermissionError(msg)
    retry_delays: list[float] = []
    for attempt in range(retries + 1):
        _wait_with_shared_fallback(shared_rate_limiter, rate_limiter, backoff_seconds)
        try:
            response = http.get(url)
        except (httpx.TimeoutException, httpx.TransportError):
            if attempt == retries:
                raise
            delay = min(float(backoff_seconds) * (2**attempt), 60.0)
            retry_delays.append(delay)
            _record_shared_backoff(
                shared_rate_limiter,
                delay_seconds=delay,
                status_code=599,
            )
            if delay > 0:
                sleeper(delay)
            continue
        if response.status_code not in {429, 500, 502, 503, 504}:
            response.extensions["fyi_attempts"] = attempt + 1
            response.extensions["fyi_retry_delays_seconds"] = retry_delays
            return response
        if attempt == retries:
            response.extensions["fyi_attempts"] = attempt + 1
            response.extensions["fyi_retry_delays_seconds"] = retry_delays
            return response
        delay = retry_delay_seconds(
            response.headers, attempt=attempt, max_seconds=max(1, int(backoff_seconds * 256))
        )
        retry_delays.append(delay)
        _record_shared_backoff(
            shared_rate_limiter,
            delay_seconds=delay,
            status_code=response.status_code,
        )
        if delay > 0:
            sleeper(delay)
    return response


def load_checkpoint(path: Path | None) -> int:
    """Load the next page number from a checkpoint file."""
    if path is None or not path.exists():
        return 1
    data = json.loads(path.read_text(encoding="utf-8"))
    return int(data.get("next_page") or 1)


def write_checkpoint(path: Path | None, next_page: int) -> None:
    """Write the next page number to a checkpoint file."""
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"next_page": next_page}, indent=2) + "\n", encoding="utf-8")


def parse_feed_entries(data: dict[str, Any]) -> tuple[list[DiscoveredRequest], bool]:
    """Parse an Alaveteli-ish JSON search feed page."""
    raw_entries = first_list(data, "entries", "items", "results", "requests")
    entries = []
    for raw in raw_entries:
        if isinstance(raw, dict):
            parsed = parse_entry(raw)
            if parsed is not None:
                entries.append(parsed)
    return dedupe(entries), has_next_page(data, page_had_entries=bool(entries))


def first_list(data: dict[str, Any], *keys: str) -> list[Any]:
    """Return the first list value under one of the given keys."""
    for key in keys:
        value = data.get(key)
        if isinstance(value, list):
            return value
    return []


def has_next_page(data: dict[str, Any], *, page_had_entries: bool) -> bool:
    """Infer whether a JSON feed has another page."""
    if data.get("next") or data.get("next_page"):
        return True
    links = data.get("links")
    if isinstance(links, dict) and links.get("next"):
        return True
    return bool(data.get("has_more")) or (page_had_entries and data.get("final_page") is False)


def parse_entry(raw: dict[str, Any]) -> DiscoveredRequest | None:
    """Normalize one feed entry into the archive request shape."""
    request_id = request_id_from_entry(raw)
    if request_id is None:
        return None
    link = str(raw.get("url") or raw.get("link") or raw.get("html_url") or "")
    return DiscoveredRequest(
        request_id=request_id,
        url_title=url_title_from_link(link) or str(raw.get("url_title") or f"request-{request_id}"),
        title=str(raw.get("title") or raw.get("name") or ""),
        authority=authority_name(raw.get("authority") or raw.get("public_body")),
        state=str(raw.get("state") or raw.get("status") or raw.get("latest_status") or ""),
        created_at=str(
            raw.get("created_at") or raw.get("requested_at") or raw.get("updated") or "",
        ),
    )


def authority_name(value: Any) -> str:
    """Normalize FYI authority values to a stable short string."""
    if isinstance(value, dict):
        return str(value.get("url_name") or value.get("name") or value.get("id") or "")
    return str(value or "")


def request_id_from_entry(raw: dict[str, Any]) -> int | None:
    """Extract a numeric request ID from an entry."""
    for key in ("request_id", "id"):
        value = raw.get(key)
        if isinstance(value, int):
            return value
        if isinstance(value, str) and value.isdigit():
            return int(value)
    for key in ("url", "link", "html_url", "id"):
        match = REQUEST_RE.search(str(raw.get(key) or ""))
        if match and match.group("id").isdigit():
            return int(match.group("id"))
    return None


def url_title_from_link(link: str) -> str | None:
    """Extract a url_title slug from a request URL when present."""
    match = REQUEST_RE.search(link)
    if not match:
        return None
    value = match.group("id")
    return None if value.isdigit() else value


def dedupe(entries: Iterable[DiscoveredRequest]) -> list[DiscoveredRequest]:
    """Deduplicate request rows by request_id, preserving order."""
    seen: set[int] = set()
    out = []
    for entry in entries:
        if entry.request_id in seen:
            continue
        seen.add(entry.request_id)
        out.append(entry)
    return out


def discover_feed(
    *,
    base_url: str = "https://fyi.org.nz",
    date_from: str | None = None,
    date_to: str | None = None,
    authority: str | None = None,
    status: str | None = None,
    checkpoint_path: Path | None = None,
    max_pages: int | None = None,
    delay_seconds: float = 1.0,
    shared_rate_limit_db_path: str | Path | None = None,
    shared_rate_limit_name: str = "archive-discovery",
    transport: httpx.BaseTransport | None = None,
    recorder: Any | None = None,
) -> list[DiscoveredRequest]:
    """Walk paginated search feed pages and return deduplicated requests."""
    page = load_checkpoint(checkpoint_path)
    final_page = None if max_pages is None else page + max_pages - 1
    all_entries: list[DiscoveredRequest] = []
    with client(base_url, transport=transport) as http:
        disallows = load_robots_disallow(http, recorder=recorder)
        shared_rate_limiter = (
            SharedRateLimiter(shared_rate_limit_db_path, name=shared_rate_limit_name)
            if shared_rate_limit_db_path is not None
            else None
        )
        rate_limiter = PoliteRateLimiter(delay_seconds)
        while final_page is None or page <= final_page:
            url = build_search_url(
                base_url=base_url,
                date_from=date_from,
                date_to=date_to,
                authority=authority,
                status=status,
                page=page,
            )
            response = get_with_backoff(
                http,
                url,
                disallows=disallows,
                shared_rate_limiter=shared_rate_limiter,
                rate_limiter=rate_limiter,
                backoff_seconds=delay_seconds,
            )
            observe_response(recorder, response)
            response.raise_for_status()
            entries, has_next = parse_feed_entries(response.json())
            all_entries.extend(entries)
            write_checkpoint(checkpoint_path, page + 1)
            if not has_next:
                break
            page += 1
    return dedupe(all_entries)


def backfill_ids(
    *,
    id_from: int,
    id_to: int,
    base_url: str = "https://fyi.org.nz",
    delay_seconds: float = 1.0,
    shared_rate_limit_db_path: str | Path | None = None,
    shared_rate_limit_name: str = "archive-discovery",
    transport: httpx.BaseTransport | None = None,
    recorder: Any | None = None,
) -> list[DiscoveredRequest]:
    """Probe numeric request IDs, following redirects to url_title slugs."""
    entries = []
    with client(base_url, transport=transport) as http:
        disallows = load_robots_disallow(http, recorder=recorder)
        shared_rate_limiter = (
            SharedRateLimiter(shared_rate_limit_db_path, name=shared_rate_limit_name)
            if shared_rate_limit_db_path is not None
            else None
        )
        rate_limiter = PoliteRateLimiter(delay_seconds)
        for request_id in range(id_from, id_to + 1):
            response = get_with_backoff(
                http,
                f"/request/{request_id}.json",
                disallows=disallows,
                shared_rate_limiter=shared_rate_limiter,
                rate_limiter=rate_limiter,
                backoff_seconds=delay_seconds,
            )
            observe_response(recorder, response)
            if response.status_code == 404:
                continue
            response.raise_for_status()
            data = response.json()
            parsed = parse_entry(
                {
                    "request_id": request_id,
                    "url": str(response.url).removesuffix(".json"),
                    "title": data.get("title") or data.get("info_request", {}).get("title"),
                    "authority": data.get("public_body") or data.get("authority"),
                    "state": data.get("state") or data.get("described_state"),
                    "created_at": data.get("created_at"),
                },
            )
            if parsed is not None:
                entries.append(parsed)
    return dedupe(entries)


def write_jsonl(path: Path, rows: Iterable[DiscoveredRequest]) -> None:
    """Write discovered requests as JSONL."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(row.to_json() for row in rows) + "\n", encoding="utf-8")


def read_jsonl_request_ids(path: Path) -> set[int]:
    """Read request IDs from discovery JSONL output."""
    ids: set[int] = set()
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        data = json.loads(line)
        request_id = data.get("request_id")
        if not isinstance(request_id, int):
            msg = f"{path}:{line_number} missing integer request_id"
            raise TypeError(msg)
        ids.add(request_id)
    return ids


def reconcile_discovery_files(feed_path: Path, backfill_path: Path) -> DiscoveryReconciliation:
    """Compare feed and backfill JSONL outputs by request ID."""
    feed_ids = read_jsonl_request_ids(feed_path)
    backfill_ids = read_jsonl_request_ids(backfill_path)
    matched = feed_ids & backfill_ids
    return DiscoveryReconciliation(
        feed_count=len(feed_ids),
        backfill_count=len(backfill_ids),
        matched_count=len(matched),
        missing_from_feed=sorted(backfill_ids - feed_ids),
        missing_from_backfill=sorted(feed_ids - backfill_ids),
    )


def shared_rate_limit_status(
    db_path: str | Path,
    *,
    name: str = "archive-discovery",
) -> dict[str, Any] | None:
    """Return the current shared limiter state as plain JSON-compatible data."""
    snapshot = SharedRateLimiter(db_path, name=name).snapshot()
    return snapshot.to_dict() if snapshot is not None else None
