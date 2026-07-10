"""Resource-aware agent network helpers (Python parity for live paths).

Mirrors the Rust ``fyi_core::agent_runtime`` policy for:
- cryptographic-aligned User-Agent (product, version, fingerprint, opt-in contact)
- RateLimit-* / Retry-After parsing
- exponential backoff
- continuous behavioral guardrails
- simple filesystem response cache
- JSONL execution traces (Langfuse/Braintrust-friendly field names)

FOSS-only; no proprietary SDKs required.
"""

from __future__ import annotations

import hashlib
import json
import re
import time
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path
from typing import Any, Mapping

PRODUCT_NAME = "fyi-cli"
PRODUCT_VERSION = "0.6.0"
PRODUCT_REPO = "https://github.com/edithatogo/fyi-cli"

_GENERIC_UA = re.compile(
    r"^(?:curl|wget|python-requests|python-urllib|go-http-client|reqwest)/",
    re.IGNORECASE,
)


def content_fingerprint(product: str, version: str, homepage: str) -> str:
    """Non-secret SHA-256 prefix for build-aligned identity."""
    h = hashlib.sha256()
    h.update(product.encode())
    h.update(b"\0")
    h.update(version.encode())
    h.update(b"\0")
    h.update(homepage.encode())
    return h.hexdigest()[:16]


def is_generic_user_agent(ua: str) -> bool:
    t = (ua or "").strip()
    if not t:
        return True
    if t.lower() == "mozilla/5.0":
        return True
    if _GENERIC_UA.match(t):
        return True
    if "/" not in t:
        return True
    return False


@dataclass(frozen=True)
class ClientIdentity:
    product: str
    version: str
    fingerprint: str
    homepage: str
    admin_contact: str | None = None

    @classmethod
    def default(cls, admin_contact: str | None = None) -> ClientIdentity:
        contact = (admin_contact or "").strip() or None
        fp = content_fingerprint(PRODUCT_NAME, PRODUCT_VERSION, PRODUCT_REPO)
        identity = cls(
            product=PRODUCT_NAME,
            version=PRODUCT_VERSION,
            fingerprint=fp,
            homepage=PRODUCT_REPO,
            admin_contact=contact,
        )
        identity.validate()
        return identity

    def validate(self) -> None:
        if not self.product.strip():
            raise ValueError("product name is required")
        if not self.version.strip():
            raise ValueError("version is required")
        if len(self.fingerprint) < 8:
            raise ValueError("fingerprint too short")
        if "://" not in self.homepage:
            raise ValueError("homepage must be an absolute URL")
        if self.admin_contact is not None and not self.admin_contact.strip():
            raise ValueError("admin contact, if set, must be non-empty")
        if is_generic_user_agent(self.user_agent()):
            raise ValueError("user-agent is blank or generic")

    def user_agent(self, component: str | None = None) -> str:
        inner = f"fp:{self.fingerprint}; +{self.homepage}"
        if self.admin_contact:
            inner += f"; contact:{self.admin_contact}"
        product = f"{self.product} {component}" if component else self.product
        return f"{product}/{self.version} ({inner})"


def build_user_agent(
    admin_contact: str | None = None, *, component: str | None = None
) -> str:
    """Contactable default UA for discovery/capture live paths."""
    return ClientIdentity.default(admin_contact).user_agent(component)


@dataclass
class RateLimitSnapshot:
    limit: int | None = None
    remaining: int | None = None
    reset_seconds: int | None = None
    retry_after_seconds: int | None = None
    http_status: int | None = None

    @classmethod
    def from_headers(cls, headers: Mapping[str, str]) -> RateLimitSnapshot:
        snap = cls()
        normalized = {str(k).lower(): str(v) for k, v in headers.items()}
        snap.limit = _parse_int(normalized.get("ratelimit-limit") or normalized.get("x-ratelimit-limit"))
        snap.remaining = _parse_int(
            normalized.get("ratelimit-remaining") or normalized.get("x-ratelimit-remaining")
        )
        snap.reset_seconds = _parse_delay(
            normalized.get("ratelimit-reset") or normalized.get("x-ratelimit-reset")
        )
        ra = normalized.get("retry-after")
        if ra is not None:
            snap.retry_after_seconds = _parse_delay(ra)
        return snap


def _parse_int(value: str | None) -> int | None:
    if value is None:
        return None
    try:
        return int(float(value.strip().split(".", 1)[0]))
    except (TypeError, ValueError):
        return None


def _parse_delay(value: str | None) -> int | None:
    """Parse delta-seconds or an HTTP-date into a non-negative delay."""
    parsed = _parse_int(value)
    if parsed is not None:
        return max(0, parsed)
    if not value:
        return None
    try:
        target = parsedate_to_datetime(value.strip())
        if target.tzinfo is None:
            target = target.replace(tzinfo=timezone.utc)
        return max(0, int((target - datetime.now(timezone.utc)).total_seconds()))
    except (TypeError, ValueError, OverflowError):
        return None


def retry_delay_seconds(
    headers: Mapping[str, str], *, attempt: int = 0, max_seconds: int = 300
) -> int:
    """Return the server retry delay, falling back to bounded exponential backoff."""
    snapshot = RateLimitSnapshot.from_headers(headers)
    return exponential_backoff_seconds(
        attempt,
        retry_after=snapshot.retry_after_seconds,
        max_seconds=max_seconds,
    )


def exponential_backoff_seconds(
    attempt: int,
    retry_after: int | None = None,
    max_seconds: int = 300,
) -> int:
    """Exponential backoff with Retry-After floor and light deterministic jitter."""
    attempt = max(0, min(int(attempt), 8))
    base = min(1 << attempt, max_seconds)
    wait = base
    if retry_after is not None:
        wait = max(wait, int(retry_after))
        wait = min(wait, max(max_seconds, int(retry_after)))
    jitter = max(wait // 8, 1)
    mixed = wait + (attempt % (jitter + 1))
    return min(mixed, max(max_seconds, int(retry_after or 0)))


@dataclass
class GuardrailConfig:
    max_requests: int = 10_000
    max_response_bytes: int = 500 * 1024 * 1024
    max_runtime_seconds: float = 3600.0
    max_concurrency: int = 4


class GuardrailTracker:
    def __init__(self, config: GuardrailConfig | None = None) -> None:
        self.config = config or GuardrailConfig()
        self.started = time.monotonic()
        self.requests = 0
        self.response_bytes = 0

    def record_request_start(self) -> None:
        if self.requests >= self.config.max_requests:
            raise RuntimeError(
                f"guardrail tripped: maximum request count reached ({self.config.max_requests})"
            )
        if time.monotonic() - self.started >= self.config.max_runtime_seconds:
            raise RuntimeError(
                f"guardrail tripped: maximum runtime exceeded ({self.config.max_runtime_seconds}s)"
            )
        self.requests += 1

    def record_response_bytes(self, nbytes: int) -> None:
        self.response_bytes += int(nbytes)
        if self.response_bytes > self.config.max_response_bytes:
            raise RuntimeError(
                "guardrail tripped: maximum response bytes exceeded "
                f"({self.response_bytes} > {self.config.max_response_bytes})"
            )


@dataclass
class RetrievalPlan:
    instance_id: str
    description: str
    estimated_requests: int = 0
    date_from: str | None = None
    date_to: str | None = None
    max_pages: int | None = None
    recursive_unbounded: bool = False
    is_heavy: bool = False
    force_schedule: bool = False


def reflect_plan(plan: RetrievalPlan) -> dict[str, Any]:
    """Plan-and-solve reflection; returns accept/rewrite/reject decision dict."""
    if plan.recursive_unbounded:
        if plan.date_from and plan.date_to:
            rewritten = RetrievalPlan(
                instance_id=plan.instance_id,
                description=f"{plan.description} [rewritten: bounded pages]",
                estimated_requests=plan.estimated_requests,
                date_from=plan.date_from,
                date_to=plan.date_to,
                max_pages=min(plan.max_pages or 50, 50),
                recursive_unbounded=False,
                is_heavy=plan.is_heavy,
                force_schedule=plan.force_schedule,
            )
            return {
                "decision": "rewrite",
                "rationale": "unbounded recursive retrieval rewritten with page bound",
                "rewritten": asdict(rewritten),
            }
        return {
            "decision": "reject",
            "rationale": "unbounded recursive retrieval without a date window is rejected",
        }
    if plan.estimated_requests > 50_000 and not plan.force_schedule:
        return {
            "decision": "reject",
            "rationale": "estimated request count exceeds safety ceiling without force_schedule",
        }
    return {"decision": "accept", "rationale": "plan is bounded and within safety policy"}


class FilesystemResponseCache:
    def __init__(self, root: str | Path) -> None:
        self.root = Path(root)
        self.root.mkdir(parents=True, exist_ok=True)

    def _path(self, url: str) -> Path:
        digest = hashlib.sha256(url.encode()).hexdigest()
        return self.root / digest[:2] / digest

    def get(self, url: str) -> bytes | None:
        path = self._path(url)
        if not path.exists():
            return None
        return path.read_bytes()

    def put(self, url: str, body: bytes) -> None:
        path = self._path(url)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(body)


@dataclass
class JsonlTraceSink:
    path: Path
    run_id: str = field(default_factory=lambda: f"run-{int(time.time())}")

    def emit(
        self,
        name: str,
        type_: str = "event",
        instance_id: str | None = None,
        metadata: dict[str, Any] | None = None,
    ) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        payload = {
            "type": type_,
            "name": name,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "run_id": self.run_id,
            "instance_id": instance_id,
            "level": "DEFAULT",
            "metadata": redact_secrets(metadata or {}),
            "id": hashlib.sha256(f"{self.run_id}:{name}:{time.time()}".encode()).hexdigest()[:16],
        }
        with self.path.open("a", encoding="utf-8") as fh:
            fh.write(json.dumps(payload, default=str) + "\n")


def redact_secrets(value: Any) -> Any:
    if isinstance(value, dict):
        out: dict[str, Any] = {}
        for k, v in value.items():
            lower = str(k).lower()
            if any(s in lower for s in ("api_key", "authorization", "password", "secret", "cookie", "token")):
                out[k] = "[redacted]"
            else:
                out[k] = redact_secrets(v)
        return out
    if isinstance(value, list):
        return [redact_secrets(v) for v in value]
    if isinstance(value, str) and ("api_key=" in value or "Bearer " in value):
        return "[redacted]"
    return value
