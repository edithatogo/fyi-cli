"""Fail-closed client-side evaluation of an operator-endorsed route."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

PROTOCOL_VERSION = "fyi-endorsed-client/v1"
MALFORMED = "malformed endorsed-route capability document"
INVALID = "endorsed route capability document is invalid"
UNSUPPORTED = "unsupported endorsed-route protocol"
DISABLED = "endorsed route is disabled"
KILL_SWITCH = "endorsed route kill switch is active"
REVOKED = "endorsed route authorization is revoked"
EXPIRED = "endorsed route capability document has expired"
NOT_ALLOWED = "client is not in the endorsed-route allowlist"
SCOPE_NOT_ALLOWED = "requested scope is not authorized"
NON_POSITIVE_QUOTA = "endorsed route contains a non-positive quota"
BULK_DISABLED = "bulk export is not enabled for this route"
BULK_SCOPE_NOT_ALLOWED = "bulk export scope is not authorized"


class EndorsedRouteError(ValueError):
    """Raised when a capability document cannot authorize a route."""


@dataclass(frozen=True)
class RouteQuotas:
    max_requests: int
    max_bytes: int
    max_runtime_seconds: int
    max_concurrency: int
    max_retries: int


@dataclass(frozen=True)
class BulkExportCapability:
    enabled: bool
    scope: str
    max_items: int
    max_bytes: int


@dataclass(frozen=True)
class CapabilityDocument:
    protocol: str
    instance_id: str
    enabled: bool
    kill_switch: bool
    revoked: bool
    expires_at: int
    client_allowlist: tuple[str, ...]
    scopes: tuple[str, ...]
    quotas: RouteQuotas
    bulk_export: BulkExportCapability

    @classmethod
    def from_mapping(cls, payload: dict[str, Any]) -> CapabilityDocument:  # noqa: C901
        try:
            required = {
                "protocol",
                "instance_id",
                "enabled",
                "kill_switch",
                "revoked",
                "expires_at",
                "client_allowlist",
                "scopes",
                "quotas",
                "bulk_export",
            }
            if not isinstance(payload, dict) or set(payload) != required:
                raise EndorsedRouteError(MALFORMED)
            quotas = payload["quotas"]
            bulk = payload["bulk_export"]
            if not isinstance(quotas, dict) or not isinstance(bulk, dict):
                raise EndorsedRouteError(MALFORMED)
            quota_keys = {
                "max_requests",
                "max_bytes",
                "max_runtime_seconds",
                "max_concurrency",
                "max_retries",
            }
            if set(quotas) != quota_keys or not all(
                type(quotas[key]) is int and quotas[key] >= 0 for key in quota_keys
            ):
                raise EndorsedRouteError(MALFORMED)
            if set(bulk) != {"enabled", "scope", "max_items", "max_bytes"}:
                raise EndorsedRouteError(MALFORMED)
            if not all(type(payload[key]) is bool for key in ("enabled", "kill_switch", "revoked")):
                raise EndorsedRouteError(MALFORMED)
            if not all(type(payload[key]) is str for key in ("protocol", "instance_id")):
                raise EndorsedRouteError(MALFORMED)
            if type(payload["expires_at"]) is not int or payload["expires_at"] < 0:
                raise EndorsedRouteError(MALFORMED)
            if not isinstance(payload["client_allowlist"], list) or not all(
                type(value) is str for value in payload["client_allowlist"]
            ):
                raise EndorsedRouteError(MALFORMED)
            if not isinstance(payload["scopes"], list) or not all(
                type(value) is str for value in payload["scopes"]
            ):
                raise EndorsedRouteError(MALFORMED)
            if (
                type(bulk["enabled"]) is not bool
                or type(bulk["scope"]) is not str
                or not all(
                    type(bulk[key]) is int and bulk[key] >= 0
                    for key in ("max_items", "max_bytes")
                )
            ):
                raise EndorsedRouteError(MALFORMED)
            return cls(
                protocol=payload["protocol"],
                instance_id=payload["instance_id"],
                enabled=payload["enabled"],
                kill_switch=payload["kill_switch"],
                revoked=payload["revoked"],
                expires_at=payload["expires_at"],
                client_allowlist=tuple(payload["client_allowlist"]),
                scopes=tuple(payload["scopes"]),
                quotas=RouteQuotas(**quotas),
                bulk_export=BulkExportCapability(
                    enabled=bulk["enabled"],
                    scope=bulk["scope"],
                    max_items=bulk["max_items"],
                    max_bytes=bulk["max_bytes"],
                ),
            )
        except (KeyError, TypeError, ValueError) as exc:
            raise EndorsedRouteError(MALFORMED) from exc

    def authorize(  # noqa: C901
        self,
        *,
        client_id: str,
        scopes: tuple[str, ...],
        now_epoch: int,
        bulk_export: bool = False,
    ) -> dict[str, Any]:
        if self.protocol != PROTOCOL_VERSION:
            raise EndorsedRouteError(UNSUPPORTED)
        if not self.enabled:
            raise EndorsedRouteError(DISABLED)
        if self.kill_switch:
            raise EndorsedRouteError(KILL_SWITCH)
        if self.revoked:
            raise EndorsedRouteError(REVOKED)
        if self.expires_at <= now_epoch:
            raise EndorsedRouteError(EXPIRED)
        if (
            not self.instance_id.strip()
            or not self.client_allowlist
            or not self.scopes
            or any(not value.strip() for value in self.client_allowlist)
            or any(not value.strip() for value in self.scopes)
            or not scopes
            or any(not value.strip() for value in scopes)
            or not self.bulk_export.scope.strip()
        ):
            raise EndorsedRouteError(INVALID)
        if not client_id.strip() or client_id not in self.client_allowlist:
            raise EndorsedRouteError(NOT_ALLOWED)
        if any(scope not in self.scopes for scope in scopes):
            raise EndorsedRouteError(SCOPE_NOT_ALLOWED)
        if any(
            value <= 0
            for value in (
                self.quotas.max_requests,
                self.quotas.max_bytes,
                self.quotas.max_runtime_seconds,
                self.quotas.max_concurrency,
            )
        ):
            raise EndorsedRouteError(NON_POSITIVE_QUOTA)

        bulk = None
        if bulk_export:
            if not self.bulk_export.enabled:
                raise EndorsedRouteError(BULK_DISABLED)
            if self.bulk_export.scope not in scopes:
                raise EndorsedRouteError(BULK_SCOPE_NOT_ALLOWED)
            if self.bulk_export.max_items <= 0 or self.bulk_export.max_bytes <= 0:
                raise EndorsedRouteError(NON_POSITIVE_QUOTA)
            bulk = self.bulk_export

        return {
            "instance_id": self.instance_id,
            "scopes": list(scopes),
            "quotas": self.quotas,
            "bulk_export": bulk,
        }
