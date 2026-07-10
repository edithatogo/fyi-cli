//! Versioned, fail-closed negotiation for an opt-in Alaveteli client route.
//!
//! This module only evaluates a capability document supplied by an instance
//! operator. It does not enable a route, mint credentials, or contact an
//! upstream service by itself.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PROTOCOL_VERSION: &str = "fyi-endorsed-client/v1";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDocument {
    pub protocol: String,
    pub instance_id: String,
    pub enabled: bool,
    pub kill_switch: bool,
    pub revoked: bool,
    pub expires_at: u64,
    pub client_allowlist: Vec<String>,
    pub scopes: Vec<String>,
    pub quotas: RouteQuotas,
    pub bulk_export: BulkExportCapability,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RouteQuotas {
    pub max_requests: u64,
    pub max_bytes: u64,
    pub max_runtime_seconds: u64,
    pub max_concurrency: u32,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BulkExportCapability {
    pub enabled: bool,
    pub scope: String,
    pub max_items: u64,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRequest<'a> {
    pub client_id: &'a str,
    pub scopes: &'a [&'a str],
    pub now_epoch: u64,
    pub bulk_export: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedRoute {
    pub instance_id: String,
    pub scopes: Vec<String>,
    pub quotas: RouteQuotas,
    pub bulk_export: Option<BulkExportCapability>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RouteError {
    #[error("unsupported endorsed-route protocol")]
    UnsupportedProtocol,
    #[error("endorsed route is disabled")]
    Disabled,
    #[error("endorsed route kill switch is active")]
    KillSwitch,
    #[error("endorsed route authorization is revoked")]
    Revoked,
    #[error("endorsed route capability document has expired")]
    Expired,
    #[error("client is not in the endorsed-route allowlist")]
    ClientNotAllowed,
    #[error("requested scope is not authorized")]
    ScopeNotAllowed,
    #[error("endorsed route capability document is invalid")]
    InvalidDocument,
    #[error("endorsed route contains a non-positive quota")]
    InvalidQuota,
    #[error("bulk export is not enabled for this route")]
    BulkExportDisabled,
    #[error("bulk export scope is not authorized")]
    BulkExportScopeNotAllowed,
}

impl CapabilityDocument {
    /// Authorize a client request against operator-published capabilities.
    /// Every ambiguous or unsafe condition fails closed.
    pub fn authorize(&self, request: RouteRequest<'_>) -> Result<AuthorizedRoute, RouteError> {
        if self.protocol != PROTOCOL_VERSION {
            return Err(RouteError::UnsupportedProtocol);
        }
        if !self.enabled {
            return Err(RouteError::Disabled);
        }
        if self.kill_switch {
            return Err(RouteError::KillSwitch);
        }
        if self.revoked {
            return Err(RouteError::Revoked);
        }
        if self.expires_at <= request.now_epoch {
            return Err(RouteError::Expired);
        }
        if self.instance_id.trim().is_empty()
            || self.client_allowlist.is_empty()
            || self.scopes.is_empty()
            || self.client_allowlist.iter().any(|id| id.trim().is_empty())
            || self.scopes.iter().any(|scope| scope.trim().is_empty())
            || request.scopes.is_empty()
            || request.scopes.iter().any(|scope| scope.trim().is_empty())
        {
            return Err(RouteError::InvalidDocument);
        }
        if request.client_id.trim().is_empty()
            || !self
                .client_allowlist
                .iter()
                .any(|id| id == request.client_id)
        {
            return Err(RouteError::ClientNotAllowed);
        }
        if self.quotas.max_requests == 0
            || self.quotas.max_bytes == 0
            || self.quotas.max_runtime_seconds == 0
            || self.quotas.max_concurrency == 0
        {
            return Err(RouteError::InvalidQuota);
        }
        if request
            .scopes
            .iter()
            .any(|scope| !self.scopes.iter().any(|allowed| allowed == scope))
        {
            return Err(RouteError::ScopeNotAllowed);
        }

        let bulk_export = if request.bulk_export {
            if !self.bulk_export.enabled {
                return Err(RouteError::BulkExportDisabled);
            }
            if !request
                .scopes
                .iter()
                .any(|scope| *scope == self.bulk_export.scope)
            {
                return Err(RouteError::BulkExportScopeNotAllowed);
            }
            if self.bulk_export.scope.trim().is_empty() {
                return Err(RouteError::InvalidDocument);
            }
            if self.bulk_export.max_items == 0 || self.bulk_export.max_bytes == 0 {
                return Err(RouteError::InvalidQuota);
            }
            Some(self.bulk_export.clone())
        } else {
            None
        };

        Ok(AuthorizedRoute {
            instance_id: self.instance_id.clone(),
            scopes: request
                .scopes
                .iter()
                .map(|scope| (*scope).to_string())
                .collect(),
            quotas: self.quotas.clone(),
            bulk_export,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> CapabilityDocument {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/endorsed-client-route/enabled.json"
        ))
        .unwrap()
    }

    #[test]
    fn allows_scoped_client_and_bounded_bulk_export() {
        let doc = document();
        let route = doc
            .authorize(RouteRequest {
                client_id: "fyi-cli-prod",
                scopes: &["read", "bulk_export"],
                now_epoch: 1_700_000_000,
                bulk_export: true,
            })
            .unwrap();
        assert_eq!(route.instance_id, "nz-fyi");
        assert!(route.bulk_export.is_some());
        assert_eq!(route.quotas.max_concurrency, 2);
    }

    #[test]
    fn disabled_revoked_expired_and_unknown_clients_fail_closed() {
        let mut doc = document();
        doc.enabled = false;
        assert_eq!(
            doc.authorize(RouteRequest {
                client_id: "fyi-cli-prod",
                scopes: &["read"],
                now_epoch: 1_700_000_000,
                bulk_export: false,
            }),
            Err(RouteError::Disabled)
        );

        let mut doc = document();
        doc.kill_switch = true;
        assert_eq!(
            doc.authorize(RouteRequest {
                client_id: "fyi-cli-prod",
                scopes: &["read"],
                now_epoch: 1_700_000_000,
                bulk_export: false,
            }),
            Err(RouteError::KillSwitch)
        );

        let mut doc = document();
        doc.expires_at = 1_700_000_000;
        assert_eq!(
            doc.authorize(RouteRequest {
                client_id: "fyi-cli-prod",
                scopes: &["read"],
                now_epoch: 1_700_000_000,
                bulk_export: false,
            }),
            Err(RouteError::Expired)
        );

        let doc = document();
        assert_eq!(
            doc.authorize(RouteRequest {
                client_id: "unknown",
                scopes: &["read"],
                now_epoch: 1_700_000_000,
                bulk_export: false,
            }),
            Err(RouteError::ClientNotAllowed)
        );
    }

    #[test]
    fn bulk_export_requires_explicit_scope() {
        let doc = document();
        assert_eq!(
            doc.authorize(RouteRequest {
                client_id: "fyi-cli-prod",
                scopes: &["read"],
                now_epoch: 1_700_000_000,
                bulk_export: true,
            }),
            Err(RouteError::BulkExportScopeNotAllowed)
        );
    }

    #[test]
    fn malformed_documents_and_empty_scopes_fail_closed() {
        let mut payload: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/endorsed-client-route/enabled.json"
        ))
        .unwrap();
        payload["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CapabilityDocument>(payload).is_err());

        let doc = document();
        assert_eq!(
            doc.authorize(RouteRequest {
                client_id: "fyi-cli-prod",
                scopes: &[""],
                now_epoch: 1_700_000_000,
                bulk_export: false,
            }),
            Err(RouteError::InvalidDocument)
        );
    }
}
