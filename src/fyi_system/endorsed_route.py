"""Fail-closed client-side evaluation of an operator-endorsed route."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

PROTOCOL_VERSION = "fyi-endorsed-client/v1"


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
    def from_mapping(cls, payload: dict[str, Any]) -> "CapabilityDocument":
        try:
            quotas = payload["quotas"]
            bulk = payload["bulk_export"]
            return cls(
                protocol=str(payload["protocol"]),
                instance_id=str(payload["instance_id"]),
                enabled=bool(payload["enabled"]),
                kill_switch=bool(payload["kill_switch"]),
                revoked=bool(payload["revoked"]),
                expires_at=int(payload["expires_at"]),
                client_allowlist=tuple(str(value) for value in payload["client_allowlist"]),
                scopes=tuple(str(value) for value in payload["scopes"]),
                quotas=RouteQuotas(**{key: int(value) for key, value in quotas.items()}),
                bulk_export=BulkExportCapability(
                    enabled=bool(bulk["enabled"]),
                    scope=str(bulk["scope"]),
                    max_items=int(bulk["max_items"]),
                    max_bytes=int(bulk["max_bytes"]),
                ),
            )
        except (KeyError, TypeError, ValueError) as exc:
            raise EndorsedRouteError("malformed endorsed-route capability document") from exc

    def authorize(
        self,
        *,
        client_id: str,
        scopes: tuple[str, ...],
        now_epoch: int,
        bulk_export: bool = False,
    ) -> dict[str, Any]:
        if self.protocol != PROTOCOL_VERSION:
            raise EndorsedRouteError("unsupported endorsed-route protocol")
        if not self.enabled:
            raise EndorsedRouteError("endorsed route is disabled")
        if self.kill_switch:
            raise EndorsedRouteError("endorsed route kill switch is active")
        if self.revoked:
            raise EndorsedRouteError("endorsed route authorization is revoked")
        if self.expires_at <= now_epoch:
            raise EndorsedRouteError("endorsed route capability document has expired")
        if not client_id.strip() or client_id not in self.client_allowlist:
            raise EndorsedRouteError("client is not in the endorsed-route allowlist")
        if any(scope not in self.scopes for scope in scopes):
            raise EndorsedRouteError("requested scope is not authorized")
        if any(
            value <= 0
            for value in (
                self.quotas.max_requests,
                self.quotas.max_bytes,
                self.quotas.max_runtime_seconds,
                self.quotas.max_concurrency,
            )
        ):
            raise EndorsedRouteError("endorsed route contains a non-positive quota")

        bulk = None
        if bulk_export:
            if not self.bulk_export.enabled:
                raise EndorsedRouteError("bulk export is not enabled for this route")
            if self.bulk_export.scope not in scopes:
                raise EndorsedRouteError("bulk export scope is not authorized")
            if self.bulk_export.max_items <= 0 or self.bulk_export.max_bytes <= 0:
                raise EndorsedRouteError("endorsed route contains a non-positive quota")
            bulk = self.bulk_export

        return {
            "instance_id": self.instance_id,
            "scopes": list(scopes),
            "quotas": self.quotas,
            "bulk_export": bulk,
        }
