//! Read-only MCP adapters. Network, cache and feed parsing remain in SyncClient.

use super::*;
use fyi_core::sync::{ReadLimits, ReadProvenance, SourceRead};

pub(super) async fn read(
    policy: &RemoteMcpPolicy,
    instance_id: &str,
    operation: &str,
    arguments: &Value,
) -> Result<Value, RemoteReadError> {
    let observed = RemoteOperation::from_tool(operation)
        .filter(|operation| operation.is_read())
        .ok_or_else(RemoteReadError::unknown_operation)?;
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let request_id = arguments.get("request_id").and_then(Value::as_i64);
    let filter = arguments
        .get("filter")
        .and_then(Value::as_str)
        .unwrap_or("");
    let authority_slug = arguments
        .get("authority_slug")
        .and_then(Value::as_str)
        .unwrap_or("");
    let invalid = match observed {
        RemoteOperation::SearchRequests if query.is_empty() || query.chars().count() > 200 => {
            Some("query must be between 1 and 200 characters")
        }
        RemoteOperation::GetRequest if request_id.is_none_or(|id| id < 1) => {
            Some("request_id must be a positive integer")
        }
        RemoteOperation::ListAuthorities if filter.chars().count() > 100 => {
            Some("filter exceeds 100 characters")
        }
        RemoteOperation::DiscoverAuthorityFeeds | RemoteOperation::PlanAuthorityFeed
            if !valid_authority_slug(authority_slug) => {
                Some("authority_slug must contain 1 to 128 ASCII letters, digits, hyphens or underscores")
            }
        _ => None,
    };
    if let Some(message) = invalid {
        return Err(RemoteReadError::validation(observed, message));
    }
    let permit = policy
        .begin_request(instance_id, policy::RemoteCapability::Read)
        .map_err(|error| RemoteReadError::from_policy(observed, &error, None))?;
    let limits = ReadLimits {
        max_bytes: permit
            .remaining_result_bytes()
            .map_err(|error| RemoteReadError::from_policy(observed, &error, None))?,
        max_duration: permit.duration_budget(),
    };
    let client = policy
        .read_client(instance_id)
        .map_err(|error| RemoteReadError::from_policy(observed, &error, None))?;
    let operation_future = async {
        match observed {
            RemoteOperation::Health => {
                let read = client.health_check_read(&limits).await?;
                Ok(with_source(read, instance_id, |health| {
                    json!({
                        "instance_id": instance_id,
                        "network_reachable": health.network_reachable,
                        "api_reachable": health.api_reachable
                    })
                }))
            }
            RemoteOperation::Version => {
                #[cfg(feature = "sdk-regression-fixture")]
                if policy.sdk_regression_fixture {
                    let version = client.get_api_version_for_sdk_fixture().await?;
                    return Ok(with_generated_provenance(
                        json!({"instance_id": instance_id, "version": version}),
                        instance_id,
                    ));
                }
                let read = client.get_api_version_read(&limits).await?;
                Ok(with_source(read, instance_id, |version| {
                    json!({
                        "instance_id": instance_id, "version": version
                    })
                }))
            }
            RemoteOperation::SearchRequests => {
                let limit = arguments
                    .get("limit")
                    .and_then(Value::as_u64)
                    .unwrap_or(10)
                    .clamp(1, 50) as usize;
                let read = client
                    .search_requests_read(
                        &SearchOptions {
                            query: Some(query.into()),
                            per_page: Some(limit as u32),
                            ..Default::default()
                        },
                        &limits,
                    )
                    .await?;
                Ok(with_source(read, instance_id, |requests| {
                    json!({
                        "instance_id": instance_id, "query": query,
                        "requests": requests.into_iter().take(limit).collect::<Vec<_>>()
                    })
                }))
            }
            RemoteOperation::GetRequest => {
                let read = client
                    .fetch_request_read(
                        request_id.expect("request ID validated before admission"),
                        &limits,
                    )
                    .await?;
                Ok(with_source(read, instance_id, |request| {
                    json!({
                        "instance_id": instance_id, "request": request
                    })
                }))
            }
            RemoteOperation::ListAuthorities => {
                let limit = arguments
                    .get("limit")
                    .and_then(Value::as_u64)
                    .unwrap_or(50)
                    .clamp(1, 200) as usize;
                let filter = filter.trim().to_lowercase();
                let read = client.list_authorities_read(&limits).await?;
                Ok(with_source(read, instance_id, |authorities| {
                    let authorities = authorities
                        .into_iter()
                        .filter(|authority| {
                            filter.is_empty()
                                || [
                                    authority.id.as_deref(),
                                    authority.slug.as_deref(),
                                    authority.name.as_deref(),
                                    authority.url.as_deref(),
                                ]
                                .into_iter()
                                .flatten()
                                .any(|field| field.to_lowercase().contains(&filter))
                        })
                        .take(limit)
                        .collect::<Vec<_>>();
                    json!({"instance_id": instance_id, "authorities": authorities})
                }))
            }
            RemoteOperation::DiscoverAuthorityFeeds => {
                let feed_url = client.authority_feed_url(authority_slug)?;
                Ok(with_generated_provenance(
                    json!({
                        "instance_id": instance_id, "authority_slug": authority_slug,
                        "feeds": [{"url": feed_url, "availability": "not_checked"}]
                    }),
                    instance_id,
                ))
            }
            RemoteOperation::PlanAuthorityFeed => {
                let limit = arguments
                    .get("limit")
                    .and_then(Value::as_u64)
                    .unwrap_or(50)
                    .clamp(1, 200) as usize;
                let read = client
                    .plan_authority_feed(authority_slug, limit, &limits)
                    .await?;
                Ok(with_source(read, instance_id, |plan| {
                    json!({
                        "instance_id": instance_id, "authority_slug": authority_slug, "plan": plan
                    })
                }))
            }
            _ => unreachable!("read operation validated before admission"),
        }
    };
    execute_remote_operation_typed(policy, permit, observed, operation_future).await
}

fn valid_authority_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 128
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn with_source<T>(read: SourceRead<T>, instance_id: &str, value: impl FnOnce(T) -> Value) -> Value {
    let mut payload = value(read.value);
    let ReadProvenance {
        retrieved_at,
        cache_status,
        source_status,
        response_bytes,
    } = read.provenance;
    payload["provenance"] = json!({
        "instance_id": instance_id, "retrieved_at": retrieved_at,
        "cache_status": cache_status, "source_status": source_status,
        "response_bytes": response_bytes, "correlation_id": null
    });
    payload
}

fn with_generated_provenance(mut payload: Value, instance_id: &str) -> Value {
    let retrieved_at = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    payload["provenance"] = json!({
        "instance_id": instance_id, "retrieved_at": retrieved_at,
        "cache_status": "not_applicable", "source_status": null,
        "response_bytes": 0, "correlation_id": null
    });
    payload
}

pub(super) fn provenance_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "instance_id": {"type": "string", "minLength": 1, "maxLength": 64},
            "retrieved_at": {"type": "string", "format": "date-time"},
            "cache_status": {"type": "string", "enum": ["miss", "revalidated", "not_applicable"]},
            "correlation_id": {"type": "string", "minLength": 1, "maxLength": 64},
            "source_status": {"type": ["integer", "null"], "minimum": 100, "maximum": 999},
            "response_bytes": {"type": "integer", "minimum": 0}
        },
        "required": ["instance_id", "retrieved_at", "cache_status", "correlation_id", "source_status", "response_bytes"],
        "additionalProperties": false
    })
}

pub(super) fn error_schema(operation: &str) -> Value {
    json!({
        "type": "object",
        "properties": {"error": {
            "type": "object",
            "properties": {
                "schema_version": {"type": "integer", "const": 1},
                "kind": {"type": "string", "enum": ["auth", "forbidden", "not_found", "rate_limited", "upstream_unavailable", "upstream_server", "timeout", "upstream_unknown", "validation", "malformed_payload", "guardrail", "transport", "cache", "disabled", "budget_exceeded"]},
                "operation": {"type": "string", "const": operation},
                "retryable": {"type": "boolean"},
                "status": {"type": ["integer", "null"], "minimum": 100, "maximum": 999},
                "safe_message": {"type": "string", "minLength": 1},
                "correlation_id": {"type": ["string", "null"], "minLength": 1, "maxLength": 64}
            },
            "required": ["schema_version", "kind", "operation", "retryable", "status", "safe_message", "correlation_id"],
            "additionalProperties": false
        }},
        "required": ["error"], "additionalProperties": false
    })
}
