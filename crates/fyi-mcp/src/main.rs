use chrono::NaiveDate;
use fyi_core::api::AlaveteliRequest;
use fyi_core::api::{
    AddCorrespondencePayload, CorrespondenceDirection, CreateRequestPayload,
    UpdateRequestStatePayload,
};
use fyi_core::db::DbPool;
use fyi_core::deadlines::{calculate_deadline, DeadlineInput, WorkingDayRule};
use fyi_core::endorsed_route::{CapabilityDocument, RouteRequest};
use fyi_core::search::{InMemorySearchIndex, SearchDocument, SearchIndex};
use fyi_core::sync::{SearchOptions, SyncClient};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub mod policy;
use policy::RemoteMcpPolicy;
pub mod write_gate;
use write_gate::RemoteWriteGate;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Authority {
    pub slug: String,
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportAuthoritiesResult {
    pub imported: usize,
}

impl JsonRpcResponse {
    fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Option<Value>, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }
}

fn tool_success(id: Option<Value>, payload: Value) -> JsonRpcResponse {
    JsonRpcResponse::success(
        id,
        json!({
            "structuredContent": payload,
            "content": [
                {
                    "type": "text",
                    "text": serde_json::to_string_pretty(&payload).unwrap()
                }
            ]
        }),
    )
}

fn endorsed_route_status(arguments: &Value) -> Result<Value, String> {
    let document: CapabilityDocument = serde_json::from_value(
        arguments
            .get("capabilities")
            .cloned()
            .ok_or_else(|| "capabilities is required".to_string())?,
    )
    .map_err(|_| "capabilities document is malformed".to_string())?;
    let client_id = arguments
        .get("client_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "client_id is required".to_string())?;
    let scope_values = arguments
        .get("scopes")
        .and_then(Value::as_array)
        .ok_or_else(|| "scopes is required".to_string())?;
    let scope_strings = scope_values
        .iter()
        .map(|value| value.as_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| "scopes must contain strings".to_string())?;
    let scope_refs = scope_strings.iter().map(String::as_str).collect::<Vec<_>>();
    let now_epoch = arguments
        .get("now_epoch")
        .and_then(Value::as_u64)
        .ok_or_else(|| "now_epoch is required".to_string())?;
    let bulk_export = arguments
        .get("bulk_export")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    match document.authorize(RouteRequest {
        client_id,
        scopes: &scope_refs,
        now_epoch,
        bulk_export,
    }) {
        Ok(route) => serde_json::to_value(json!({
            "status": "authorized",
            "instance_id": route.instance_id,
            "scopes": route.scopes,
            "quotas": route.quotas,
            "bulk_export": route.bulk_export,
        }))
        .map_err(|error| error.to_string()),
        Err(error) => Ok(json!({
            "status": "denied",
            "reason": error.to_string(),
        })),
    }
}

/// Helper function to generate the next auto-incremented ID for Alaveteli requests in SQLite.
async fn get_next_request_id(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(id), 0) + 1 FROM requests")
        .fetch_one(pool)
        .await
        .unwrap_or(1)
}

/// Ensure that the authorities table exists in SQLite database.
async fn ensure_authorities_table(pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS authorities (
            slug TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            url TEXT
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn request_schema(description: &str) -> Value {
    json!({
        "type": "object",
        "description": description,
        "properties": {
            "id": {
                "type": "integer",
                "minimum": 1,
                "description": "Stable local request identifier."
            },
            "title": {
                "type": "string",
                "minLength": 1,
                "description": "Public title of the official information request."
            },
            "body": {
                "type": "string",
                "minLength": 1,
                "description": "Request body or draft correspondence text."
            },
            "user_name": {
                "type": ["string", "null"],
                "description": "Requester display name when known."
            },
            "status": {
                "type": ["string", "null"],
                "enum": ["draft", "submitted", "waiting_response", "successful", "partially_successful", "refused", "overdue", "clean", "dirty", "pending", "conflict", null],
                "description": "Current Alaveteli/FYI request lifecycle status."
            },
            "created_at": {
                "type": ["string", "null"],
                "format": "date-time",
                "description": "ISO-8601 creation timestamp when recorded."
            },
            "updated_at": {
                "type": ["string", "null"],
                "format": "date-time",
                "description": "ISO-8601 update timestamp used for ordering and sync."
            },
            "url": {
                "type": ["string", "null"],
                "format": "uri",
                "description": "Canonical public request URL when available."
            },
            "tags": {
                "type": ["array", "null"],
                "description": "Optional local tags attached to the request.",
                "items": { "type": "string" }
            }
        },
        "required": ["id", "title", "body"],
        "additionalProperties": false
    })
}

fn correspondence_schema() -> Value {
    json!({
        "type": "object",
        "description": "A correspondence item associated with an FYI request.",
        "properties": {
            "direction": {
                "type": "string",
                "enum": ["request", "response"],
                "description": "Whether the correspondence was sent by the requester or received as an authority response."
            },
            "body": {
                "type": "string",
                "minLength": 1,
                "description": "Message body or extracted correspondence text."
            },
            "sent_at": {
                "type": "string",
                "format": "date-time",
                "description": "ISO-8601 sent timestamp when captured."
            },
            "state": {
                "type": ["string", "null"],
                "description": "Optional Alaveteli correspondence state when captured."
            },
            "attachments": {
                "type": ["array", "null"],
                "description": "Optional attachment URLs or identifiers linked to the correspondence.",
                "items": {
                    "type": "string",
                    "minLength": 1
                }
            }
        },
        "required": ["direction", "body", "sent_at"],
        "additionalProperties": false
    })
}

fn authority_schema() -> Value {
    json!({
        "type": "object",
        "description": "Public authority record used to route or classify requests.",
        "properties": {
            "slug": {
                "type": "string",
                "minLength": 1,
                "description": "Stable authority slug used as an import key."
            },
            "name": {
                "type": "string",
                "minLength": 1,
                "description": "Human-readable public authority name."
            },
            "url": {
                "type": ["string", "null"],
                "format": "uri",
                "description": "Optional public authority URL."
            }
        },
        "required": ["slug", "name"],
        "additionalProperties": false
    })
}

fn enrich_tool_definitions(tools: &mut Value) {
    let Some(tool_list) = tools.get_mut("tools").and_then(Value::as_array_mut) else {
        return;
    };

    for tool in tool_list {
        let Some(name) = tool.get("name").and_then(Value::as_str) else {
            continue;
        };

        match name {
            "list_requests" => {
                tool["description"] = json!(
                    "List locally tracked FYI/Alaveteli official information requests, ordered newest first. Use for dashboards, triage, or finding an ID before retrieve_request; do not use when you already know the ID and need correspondence (use retrieve_request) or only need health metrics (use check_status). Read-only and idempotent; does not contact remote authorities."
                );
                tool["inputSchema"]["properties"]["limit"]["description"] = json!(
                    "Maximum number of request records to return (1-500). Defaults to 100 when omitted; raise only when the agent needs a broader scan."
                );
                tool["inputSchema"]["properties"]["limit"]["minimum"] = json!(1);
                tool["inputSchema"]["properties"]["limit"]["maximum"] = json!(500);
                tool["inputSchema"]["properties"]["limit"]["default"] = json!(100);
                tool["outputSchema"]["properties"]["requests"]["items"] =
                    request_schema("One tracked FYI/Alaveteli request.");
            }
            "retrieve_request" => {
                tool["description"] = json!(
                    "Retrieve one locally tracked FYI/Alaveteli request and its stored correspondence by request ID. Use after list_requests when you need full text/history for a single request; do not use for bulk listing (list_requests) or database health (check_status). Read-only and idempotent; fails if the ID is missing; never contacts remote sites."
                );
                tool["inputSchema"]["properties"]["id"]["description"] = json!(
                    "Stable local request ID to load (must already exist; typically from list_requests)."
                );
                tool["inputSchema"]["properties"]["id"]["minimum"] = json!(1);
                tool["outputSchema"]["properties"]["request"] =
                    request_schema("The requested FYI/Alaveteli request record.");
                tool["outputSchema"]["properties"]["correspondence"]["description"] =
                    json!("Stored correspondence items linked to the request, in database order.");
                tool["outputSchema"]["properties"]["correspondence"]["items"] =
                    correspondence_schema();
            }
            "create_request" => {
                tool["description"] = json!(
                    "Create a new local FYI/Alaveteli request row (draft or tracked) in SQLite. Use only when starting a new request; use update_request to change an existing ID and delete_request to remove one. Mutating and not idempotent: each call allocates a new ID. Does not submit to a remote authority, send email, or require network access; title and body are required."
                );
                tool["inputSchema"]["properties"]["title"]["description"] =
                    json!("Short public-facing request title (required, non-empty).");
                tool["inputSchema"]["properties"]["title"]["minLength"] = json!(1);
                tool["inputSchema"]["properties"]["body"]["description"] = json!(
                    "Full request body or draft text to store locally (required, non-empty)."
                );
                tool["inputSchema"]["properties"]["body"]["minLength"] = json!(1);
                tool["inputSchema"]["properties"]["user_name"]["description"] =
                    json!("Optional requester display name for local tracking only.");
                tool["inputSchema"]["properties"]["status"]["description"] = json!(
                    "Optional initial local lifecycle status. Prefer draft until the request is ready; does not trigger remote submission."
                );
                tool["inputSchema"]["properties"]["status"]["enum"] = json!([
                    "draft",
                    "submitted",
                    "waiting_response",
                    "successful",
                    "partially_successful",
                    "refused",
                    "overdue",
                    "clean",
                    "dirty",
                    "pending",
                    "conflict"
                ]);
                tool["inputSchema"]["properties"]["url"]["description"] =
                    json!("Optional absolute FYI/Alaveteli URL when the request already exists online; omit for pure local drafts.");
                tool["inputSchema"]["properties"]["url"]["format"] = json!("uri");
                tool["inputSchema"]["properties"]["tags"]["description"] =
                    json!("Optional local classification tags for filtering/reporting; replace-not-merge semantics only apply on update_request.");
                tool["outputSchema"]["properties"]["request"] =
                    request_schema("The newly created local request record.");
            }
            "update_request" => {
                tool["description"] = json!(
                    "Replace editable fields on an existing local FYI/Alaveteli request by ID and mark the record dirty for offline sync. Use when the request already exists and fields changed; use create_request for a new ID and delete_request to remove. Requires id, title, and body (full replacement for those fields, not a sparse patch). Mutating but non-destructive; safe to re-run with the same values; does not call remote APIs."
                );
                tool["annotations"] = json!({
                    "readOnlyHint": false,
                    "destructiveHint": false,
                    "idempotentHint": true,
                    "openWorldHint": false
                });
                tool["inputSchema"]["properties"]["id"]["description"] =
                    json!("Existing local request ID to update (must already exist).");
                tool["inputSchema"]["properties"]["id"]["minimum"] = json!(1);
                tool["inputSchema"]["properties"]["title"]["description"] =
                    json!("Replacement request title (required; full replace, not merge).");
                tool["inputSchema"]["properties"]["title"]["minLength"] = json!(1);
                tool["inputSchema"]["properties"]["body"]["description"] =
                    json!("Replacement request body or draft text (required; full replace).");
                tool["inputSchema"]["properties"]["body"]["minLength"] = json!(1);
                tool["inputSchema"]["properties"]["user_name"]["description"] =
                    json!("Optional replacement requester display name; when omitted, existing local value is preserved.");
                tool["inputSchema"]["properties"]["status"]["description"] =
                    json!("Optional replacement local lifecycle status; changing status does not submit or withdraw a remote request.");
                tool["inputSchema"]["properties"]["status"]["enum"] = json!([
                    "draft",
                    "submitted",
                    "waiting_response",
                    "successful",
                    "partially_successful",
                    "refused",
                    "overdue",
                    "clean",
                    "dirty",
                    "pending",
                    "conflict"
                ]);
                tool["inputSchema"]["properties"]["url"]["description"] =
                    json!("Optional replacement absolute FYI/Alaveteli URL.");
                tool["inputSchema"]["properties"]["url"]["format"] = json!("uri");
                tool["inputSchema"]["properties"]["tags"]["description"] =
                    json!("Optional full replacement tag list (not appended to existing tags).");
                tool["outputSchema"]["properties"]["request"] =
                    request_schema("The updated local request record.");
            }
            "delete_request" => {
                tool["description"] = json!(
                    "Permanently delete a local request and all of its stored correspondence from SQLite. Use only after the agent confirms the ID should be discarded; prefer update_request for status/text edits and list_requests/retrieve_request for inspection. Destructive and irreversible in this database; does not delete anything on remote FYI/Alaveteli sites."
                );
                tool["inputSchema"]["properties"]["id"]["description"] =
                    json!("Stable local request ID to delete permanently.");
                tool["inputSchema"]["properties"]["id"]["minimum"] = json!(1);
                tool["outputSchema"]["properties"]["deleted"]["description"] =
                    json!("True when the local delete operation completed.");
                tool["outputSchema"]["properties"]["request_id"]["description"] =
                    json!("Request ID that was targeted for deletion.");
            }
            "list_authorities" => {
                tool["description"] = json!(
                    "List imported public authority records (government/public bodies) used to route or classify FOI/OIA requests. Use to browse existing slugs/names before drafting; use import_authorities to add or upsert records, and list_requests for request data. Read-only and idempotent; returns the full local authority table (no pagination)."
                );
                tool["outputSchema"]["properties"]["authorities"]["description"] =
                    json!("Imported public authority records.");
                tool["outputSchema"]["properties"]["authorities"]["items"] = authority_schema();
            }
            "import_authorities" => {
                tool["description"] = json!(
                    "Upsert local public authority reference records by slug for request routing and discovery. Use after list_authorities when seeding or refreshing the catalog; do not use for FOI request CRUD (create_request/update_request). Mutating but non-destructive and idempotent: same slug re-import updates name/url without creating duplicates; does not contact remote authority directories."
                );
                tool["inputSchema"]["properties"]["authorities"]["description"] =
                    json!("Authority records to upsert; slug is the primary key, name is required, url is optional.");
                tool["inputSchema"]["properties"]["authorities"]["items"] = authority_schema();
                tool["outputSchema"]["properties"]["imported"]["description"] =
                    json!("Number of authority records accepted for import or update.");
            }
            "sync_monitor" => {
                tool["description"] = json!(
                    "Summarize offline synchronization health: clean/dirty/conflict request counts, outgoing queue depth, latest sync time, and offline degradation indicators. Use for an operations overview; prefer sync_status for one request, sync_conflicts to list conflicted rows, and check_status for database connectivity/record totals. Read-only and idempotent; does not start a sync job."
                );
                tool["outputSchema"]["properties"]["sync"]["description"] =
                    json!("Aggregate request sync counts and latest sync timestamp.");
                tool["outputSchema"]["properties"]["sync"]["properties"] = json!({
                    "total": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Total number of requests tracked by sync metadata."
                    },
                    "clean": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Requests known to be cleanly synchronized."
                    },
                    "dirty": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Requests with unsynced local changes."
                    },
                    "pending": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Requests pending synchronization."
                    },
                    "conflict": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Requests with unresolved sync conflicts."
                    },
                    "latest_sync": {
                        "type": ["string", "null"],
                        "format": "date-time",
                        "description": "Most recent successful synchronization timestamp."
                    }
                });
                tool["outputSchema"]["properties"]["sync"]["required"] = json!([
                    "total",
                    "clean",
                    "dirty",
                    "pending",
                    "conflict",
                    "latest_sync"
                ]);
                tool["outputSchema"]["properties"]["sync"]["additionalProperties"] = json!(false);
                tool["outputSchema"]["properties"]["queue"]["description"] = json!(
                    "Outgoing offline queue counts by pending, submitted, and failed status."
                );
                tool["outputSchema"]["properties"]["queue"]["properties"] = json!({
                    "pending": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Outgoing queue entries waiting to be submitted."
                    },
                    "submitted": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Outgoing queue entries already submitted."
                    },
                    "failed": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Outgoing queue entries that failed submission."
                    }
                });
                tool["outputSchema"]["properties"]["queue"]["required"] =
                    json!(["pending", "submitted", "failed"]);
                tool["outputSchema"]["properties"]["queue"]["additionalProperties"] = json!(false);
                tool["outputSchema"]["properties"]["offline_degradation"]["description"] =
                    json!("Operational indicators showing queued local changes and dirty records.");
                tool["outputSchema"]["properties"]["offline_degradation"]["properties"] = json!({
                    "queued_changes": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Pending plus failed outgoing queue entries."
                    },
                    "dirty_records": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Requests with unsynced local changes."
                    }
                });
                tool["outputSchema"]["properties"]["offline_degradation"]["required"] =
                    json!(["queued_changes", "dirty_records"]);
                tool["outputSchema"]["properties"]["offline_degradation"]["additionalProperties"] =
                    json!(false);
            }
            "sync_conflicts" => {
                tool["description"] = json!(
                    "List locally tracked requests whose offline sync metadata is marked conflict. Use after sync_monitor shows conflict>0 to obtain IDs for review; use sync_resolve_conflict to clear a conflict, list_requests for all requests regardless of sync state. Read-only and idempotent; does not resolve conflicts or start network sync."
                );
                tool["inputSchema"]["properties"]["limit"]["description"] = json!(
                    "Maximum number of conflicted request records to return (1-500). Defaults to 100 when omitted."
                );
                tool["inputSchema"]["properties"]["limit"]["minimum"] = json!(1);
                tool["inputSchema"]["properties"]["limit"]["maximum"] = json!(500);
                tool["inputSchema"]["properties"]["limit"]["default"] = json!(100);
                tool["outputSchema"]["properties"]["conflicts"]["description"] =
                    json!("Requests with sync_status set to conflict.");
                tool["outputSchema"]["properties"]["conflicts"]["items"] =
                    request_schema("A request currently marked as a sync conflict.");
            }
            "sync_resolve_conflict" => {
                tool["description"] = json!(
                    "Resolve a local offline-sync conflict by updating only the request's sync metadata: mark_clean=true after the agent has reconciled local vs remote (status becomes clean); mark_clean=false keeps the row dirty for a later push. Prerequisite: the request should already appear in sync_conflicts. Side effects are local SQLite metadata only—no automatic merge of body text, no remote API calls, and no deletion of the request. Prefer sync_conflicts to list candidates and sync_status to inspect timestamps; do not use for ordinary field edits (update_request). Mutating, non-destructive, and idempotent for the same mark_clean value."
                );
                tool["annotations"] = json!({
                    "readOnlyHint": false,
                    "destructiveHint": false,
                    "idempotentHint": true,
                    "openWorldHint": false
                });
                tool["inputSchema"]["properties"]["request_id"]["description"] =
                    json!("Local request ID currently in conflict (from sync_conflicts).");
                tool["inputSchema"]["properties"]["request_id"]["minimum"] = json!(1);
                tool["inputSchema"]["properties"]["mark_clean"]["description"] = json!(
                    "true: mark reconciled/clean after manual review; false (default): leave dirty so a later offline push is expected. Does not rewrite title/body."
                );
                tool["inputSchema"]["properties"]["mark_clean"]["default"] = json!(false);
                tool["outputSchema"]["properties"]["request_id"]["description"] =
                    json!("Request ID whose conflict state was updated.");
                tool["outputSchema"]["properties"]["resolved"]["description"] =
                    json!("True when the conflict metadata was written successfully.");
                tool["outputSchema"]["properties"]["sync_status"]["description"] =
                    json!("Resulting sync status string (typically clean or dirty).");
            }
            "sync_status" => {
                tool["description"] = json!(
                    "Read offline-sync metadata either as aggregate counts (omit request_id) or for one request (provide request_id). Use when you need clean/dirty/pending/conflict numbers or per-request last_synced timestamps; use sync_monitor for queue depth + offline degradation, sync_conflicts for conflicted rows only, and check_status for database health. Read-only and idempotent; does not mutate state or trigger network sync."
                );
                tool["inputSchema"]["properties"]["request_id"]["description"] =
                    json!("Optional local request ID. Omit for aggregate counts; set to load that request's sync_status, last_synced_at, and conflict_version.");
                tool["inputSchema"]["properties"]["request_id"]["minimum"] = json!(1);
                tool["outputSchema"] = json!({
                    "type": "object",
                    "description": "Synchronization metadata. Aggregate counts are returned when request_id is omitted; per-request metadata is returned when request_id is provided.",
                    "oneOf": [
                        {
                            "type": "object",
                            "description": "Aggregate offline synchronization counts.",
                            "properties": {
                                "total": {
                                    "type": "integer",
                                    "minimum": 0,
                                    "description": "Total number of requests represented in aggregate sync status."
                                },
                                "clean": {
                                    "type": "integer",
                                    "minimum": 0,
                                    "description": "Number of requests with clean sync metadata."
                                },
                                "dirty": {
                                    "type": "integer",
                                    "minimum": 0,
                                    "description": "Number of requests with unsynced local changes."
                                },
                                "pending": {
                                    "type": "integer",
                                    "minimum": 0,
                                    "description": "Number of requests queued or pending sync."
                                },
                                "conflict": {
                                    "type": "integer",
                                    "minimum": 0,
                                    "description": "Number of requests currently in conflict."
                                }
                            },
                            "required": ["total", "clean", "dirty", "pending", "conflict"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "description": "Per-request offline synchronization metadata.",
                            "properties": {
                                "request_id": {
                                    "type": "integer",
                                    "minimum": 1,
                                    "description": "Request ID for the per-request sync lookup."
                                },
                                "sync_status": {
                                    "type": ["string", "null"],
                                    "enum": ["clean", "dirty", "pending", "conflict", null],
                                    "description": "Per-request sync status, or null when that request has no sync metadata."
                                },
                                "last_synced_at": {
                                    "type": ["string", "null"],
                                    "format": "date-time",
                                    "description": "Last successful sync timestamp for this request."
                                },
                                "remote_updated_at": {
                                    "type": ["string", "null"],
                                    "format": "date-time",
                                    "description": "Last remote update timestamp known for this request."
                                },
                                "local_updated_at": {
                                    "type": ["string", "null"],
                                    "format": "date-time",
                                    "description": "Last local update timestamp known for this request."
                                },
                                "conflict_version": {
                                    "type": ["integer", "null"],
                                    "minimum": 0,
                                    "description": "Conflict version counter when a sync conflict has been detected."
                                }
                            },
                            "required": ["request_id", "sync_status"],
                            "additionalProperties": false
                        }
                    ],
                    "additionalProperties": false
                });
            }
            "compute_deadline" => {
                tool["description"] = json!(
                    "Compute a statutory FOI/OIA deadline from a start date and day count using the bleeding-edge fyi-core deadline engine. Use for local working-day or calendar-day deadline math; do not use for listing requests (list_requests) or sync health (sync_monitor). Read-only and idempotent; pure calculation with no database or network access. Returns a StatutoryDeadline JSON object (start_date, due_date, statutory_deadline_days, working_day_rule)."
                );
                tool["inputSchema"]["properties"]["start_date"]["description"] = json!(
                    "Inclusive statutory period start date in YYYY-MM-DD (typically submission/receipt day)."
                );
                tool["inputSchema"]["properties"]["days"]["description"] = json!(
                    "Number of statutory days to count after start_date (working days by default)."
                );
                tool["inputSchema"]["properties"]["days"]["minimum"] = json!(0);
                tool["inputSchema"]["properties"]["calendar"]["description"] = json!(
                    "When true, count calendar days including weekends; when false/omitted, count weekdays only (Mon–Fri)."
                );
                tool["inputSchema"]["properties"]["calendar"]["default"] = json!(false);
                tool["outputSchema"]["properties"]["start_date"]["description"] =
                    json!("Start date echoed from input (YYYY-MM-DD).");
                tool["outputSchema"]["properties"]["due_date"]["description"] =
                    json!("Computed due date (YYYY-MM-DD).");
                tool["outputSchema"]["properties"]["statutory_deadline_days"]["description"] =
                    json!("Day count used in the calculation.");
                tool["outputSchema"]["properties"]["working_day_rule"]["description"] =
                    json!("Rule applied: weekdays_only or calendar_days.");
            }
            "search_corpus" => {
                tool["description"] = json!(
                    "Search a built-in sample FOI document corpus with the bleeding-edge in-memory inverted index (demo). Use for experimental full-text search over sample titles/bodies; prefer list_requests/retrieve_request for real local SQLite requests. Read-only and idempotent; does not query the database or network. Returns ranked hits with id, score, and title."
                );
                tool["inputSchema"]["properties"]["query"]["description"] =
                    json!("Free-text search query tokenized into alphanumeric terms.");
                tool["inputSchema"]["properties"]["query"]["minLength"] = json!(1);
                tool["inputSchema"]["properties"]["limit"]["description"] =
                    json!("Maximum number of ranked hits to return (1-50). Defaults to 10.");
                tool["inputSchema"]["properties"]["limit"]["minimum"] = json!(1);
                tool["inputSchema"]["properties"]["limit"]["maximum"] = json!(50);
                tool["inputSchema"]["properties"]["limit"]["default"] = json!(10);
                tool["outputSchema"]["properties"]["query"]["description"] =
                    json!("Echo of the search query.");
                tool["outputSchema"]["properties"]["document_count"]["description"] =
                    json!("Number of documents in the demo corpus.");
                tool["outputSchema"]["properties"]["hits"]["description"] =
                    json!("Ranked search hits (id, score, title).");
            }
            "check_status" => {
                tool["description"] = json!(
                    "Check FYI MCP SQLite readiness and return record-count metrics for requests, correspondence, and optional sync totals. Use as a first health probe or liveness check; prefer sync_monitor for queue/offline depth and list_requests for request content. Read-only, idempotent, and safe to call repeatedly; does not write data or contact remote services."
                );
                tool["outputSchema"]["properties"]["status"]["description"] = json!(
                    "Overall service health, reported as healthy when database queries succeed."
                );
                tool["outputSchema"]["properties"]["database"]["description"] =
                    json!("Database connection state used by the MCP server.");
                tool["outputSchema"]["properties"]["metrics"]["description"] =
                    json!("Record-count metrics for core FYI tables.");
                tool["outputSchema"]["properties"]["metrics"]["properties"] = json!({
                    "total_requests": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Number of request records in the database."
                    },
                    "total_correspondence": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Number of correspondence records in the database."
                    },
                    "sync": {
                        "type": ["object", "null"],
                        "description": "Aggregate sync counts when sync metadata can be read.",
                        "properties": {
                            "total": {
                                "type": "integer",
                                "minimum": 0,
                                "description": "Total number of requests represented in sync metadata."
                            },
                            "clean": {
                                "type": "integer",
                                "minimum": 0,
                                "description": "Number of clean synchronized requests."
                            },
                            "dirty": {
                                "type": "integer",
                                "minimum": 0,
                                "description": "Number of requests with unsynced local changes."
                            },
                            "pending": {
                                "type": "integer",
                                "minimum": 0,
                                "description": "Number of requests pending sync."
                            },
                            "conflict": {
                                "type": "integer",
                                "minimum": 0,
                                "description": "Number of requests in sync conflict."
                            }
                        },
                        "required": ["total", "clean", "dirty", "pending", "conflict"],
                        "additionalProperties": false
                    }
                });
                tool["outputSchema"]["properties"]["metrics"]["required"] =
                    json!(["total_requests", "total_correspondence", "sync"]);
                tool["outputSchema"]["properties"]["metrics"]["additionalProperties"] =
                    json!(false);
            }
            _ => {}
        }
    }
}

/// Handles a single incoming JSON-RPC request and produces the response.
pub async fn handle_jsonrpc_request(db: &DbPool, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
    let policy = RemoteMcpPolicy::from_env().unwrap_or_else(|_| {
        RemoteMcpPolicy::new(Vec::new()).expect("empty remote policy is valid")
    });
    handle_jsonrpc_request_with_policy_and_gate(db, req, &policy, &RemoteWriteGate::default()).await
}

fn remote_tool_definition(name: &str, description: &str) -> Value {
    let mut properties = json!({
        "instance_id": {"type": "string", "minLength": 1, "maxLength": 64}
    });
    let mut required = vec!["instance_id"];
    match name {
        "remote_search_requests" => {
            properties["query"] = json!({"type": "string", "minLength": 1, "maxLength": 200});
            properties["limit"] =
                json!({"type": "integer", "minimum": 1, "maximum": 50, "default": 10});
            required.push("query");
        }
        "remote_get_request" => {
            properties["request_id"] = json!({"type": "integer", "minimum": 1});
            required.push("request_id");
        }
        "remote_list_authorities" => {
            properties["filter"] = json!({"type": "string", "maxLength": 100});
            properties["limit"] =
                json!({"type": "integer", "minimum": 1, "maximum": 200, "default": 50});
        }
        "remote_create_request" => {
            properties["title"] = json!({"type": "string", "minLength": 1, "maxLength": 500});
            properties["body"] = json!({"type": "string", "minLength": 1, "maxLength": 100000});
            properties["external_user_name"] =
                json!({"type": "string", "minLength": 1, "maxLength": 200});
            properties["external_url"] = json!({"type": "string", "maxLength": 2048});
            properties["tags"] = json!({"type": "string", "maxLength": 500});
            properties["idempotency_key"] =
                json!({"type": "string", "minLength": 1, "maxLength": 128});
            required.extend([
                "title",
                "body",
                "external_user_name",
                "external_url",
                "idempotency_key",
            ]);
        }
        "remote_add_correspondence" => {
            properties["request_id"] = json!({"type": "integer", "minimum": 1});
            properties["direction"] = json!({"type": "string", "enum": ["request", "response"]});
            properties["body"] = json!({"type": "string", "minLength": 1, "maxLength": 100000});
            properties["sent_at"] = json!({"type": "string", "maxLength": 64});
            properties["attachments"] = json!({"type": "array", "maxItems": 8, "items": {"type": "string", "minLength": 1, "maxLength": 1024}});
            properties["idempotency_key"] =
                json!({"type": "string", "minLength": 1, "maxLength": 128});
            required.extend([
                "request_id",
                "direction",
                "body",
                "sent_at",
                "idempotency_key",
            ]);
        }
        "remote_update_state" => {
            properties["request_id"] = json!({"type": "integer", "minimum": 1});
            properties["expected_current_state"] =
                json!({"type": "string", "minLength": 1, "maxLength": 64});
            properties["state"] = json!({"type": "string", "minLength": 1, "maxLength": 64});
            properties["idempotency_key"] =
                json!({"type": "string", "minLength": 1, "maxLength": 128});
            required.extend([
                "request_id",
                "expected_current_state",
                "state",
                "idempotency_key",
            ]);
        }
        _ => {}
    }
    json!({
        "name": name,
        "title": description,
        "description": description,
        "inputSchema": {"type": "object", "properties": properties, "required": required, "additionalProperties": false},
        "annotations": {"readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": true}
    })
}

pub async fn handle_jsonrpc_request_with_policy(
    db: &DbPool,
    req: JsonRpcRequest,
    remote_policy: &RemoteMcpPolicy,
) -> Option<JsonRpcResponse> {
    handle_jsonrpc_request_with_policy_and_gate(db, req, remote_policy, &RemoteWriteGate::default())
        .await
}

pub async fn handle_jsonrpc_request_with_policy_and_gate(
    db: &DbPool,
    req: JsonRpcRequest,
    remote_policy: &RemoteMcpPolicy,
    write_gate: &RemoteWriteGate,
) -> Option<JsonRpcResponse> {
    match req.method.as_str() {
        "initialize" => {
            let res = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {
                        "subscribe": false,
                        "listChanged": false
                    },
                    "prompts": {}
                },
                "serverInfo": {
                    "name": "fyi-mcp",
                    "version": "0.1.2",
                    "title": "FYI MCP",
                    "description": "Local FOI/OIA request tracker and Alaveteli MCP server for FYI.org.nz and multi-jurisdiction instances."
                },
                // Smithery / client routing guide (server instructions).
                "instructions": "Use fyi-mcp for local Freedom of Information / Official Information request tracking against Alaveteli instances (FYI.org.nz, RightToKnow, WhatDoTheyKnow, etc.). Preferred call order: (1) check_status to confirm the local SQLite DB is healthy; (2) list_requests or resources/list (fyi://requests) for triage; (3) retrieve_request for a single ID and correspondence; (4) create_request / update_request for local drafts (does not submit to remote authorities); (5) list_authorities / import_authorities for public-body reference data; (6) sync_monitor / sync_status / sync_conflicts for offline sync health; (7) sync_resolve_conflict only after manual reconciliation; (8) compute_deadline for statutory due dates; (9) search_corpus for demo hybrid search. Prefer resources (fyi://authorities, fyi://requests, fyi://requests/{id}) when the client wants browseable corpus context rather than tool calls. Never invent remote FOI submissions—this server is local-first."
            });
            Some(JsonRpcResponse::success(req.id, res))
        }
        "notifications/initialized" => {
            // Notifications do not return responses
            None
        }
        "tools/list" => {
            let mut tools = json!({
                "tools": [
                    {
                        "name": "list_requests",
                        "title": "List Requests",
                        "description": "List tracked Alaveteli requests from the database, newest first.",
                        "inputSchema": {
                            "type": "object",
                            "description": "Optional filters for the request list.",
                            "properties": {
                                "limit": {
                                    "type": "integer",
                                    "description": "Maximum number of requests to return."
                                }
                            },
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "A request list payload.",
                            "properties": {
                                "requests": {
                                    "type": "array",
                                    "description": "Requests ordered by most recently updated.",
                                    "items": { "type": "object" }
                                }
                            },
                            "required": ["requests"],
                            "additionalProperties": false
                        },
                        "annotations": {
                            "readOnlyHint": true,
                            "destructiveHint": false,
                            "idempotentHint": true,
                            "openWorldHint": false
                        }
                    },
                    {
                        "name": "retrieve_request",
                        "title": "Retrieve Request",
                        "description": "Retrieve an Alaveteli request and its correspondence by ID from the database.",
                        "inputSchema": {
                            "type": "object",
                            "description": "Request identifier to load.",
                            "properties": {
                                "id": {
                                    "type": "integer",
                                    "description": "The unique request ID."
                                }
                            },
                            "required": ["id"],
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "A request payload with correspondence.",
                            "properties": {
                                "request": { "type": "object" },
                                "correspondence": {
                                    "type": "array",
                                    "items": { "type": "object" }
                                }
                            },
                            "required": ["request", "correspondence"],
                            "additionalProperties": false
                        },
                        "annotations": {
                            "readOnlyHint": true,
                            "destructiveHint": false,
                            "idempotentHint": true,
                            "openWorldHint": false
                        }
                    },
                    {
                        "name": "create_request",
                        "title": "Create Request",
                        "description": "Create a new request in the database.",
                        "inputSchema": {
                            "type": "object",
                            "description": "Fields for the request to create.",
                            "properties": {
                                "title": { "type": "string", "description": "The request title." },
                                "body": { "type": "string", "description": "The request body." },
                                "user_name": { "type": "string", "description": "Name of the user." },
                                "status": { "type": "string", "description": "Status of the request." },
                                "url": { "type": "string", "description": "The URL on Alaveteli or FYI." },
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Optional list of tags."
                                }
                            },
                            "required": ["title", "body"],
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "The created request.",
                            "properties": {
                                "request": { "type": "object" }
                            },
                            "required": ["request"],
                            "additionalProperties": false
                        },
                        "annotations": {
                            "readOnlyHint": false,
                            "destructiveHint": false,
                            "idempotentHint": false,
                            "openWorldHint": false
                        }
                    },
                    {
                        "name": "update_request",
                        "title": "Update Request",
                        "description": "Update an existing request in the database.",
                        "inputSchema": {
                            "type": "object",
                            "description": "Fields for the request update.",
                            "properties": {
                                "id": { "type": "integer", "description": "The request ID." },
                                "title": { "type": "string", "description": "The request title." },
                                "body": { "type": "string", "description": "The request body." },
                                "user_name": { "type": "string", "description": "Name of the user." },
                                "status": { "type": "string", "description": "Status of the request." },
                                "url": { "type": "string", "description": "The URL on Alaveteli or FYI." },
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Optional list of tags."
                                }
                            },
                            "required": ["id", "title", "body"],
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "The updated request.",
                            "properties": {
                                "request": { "type": "object" }
                            },
                            "required": ["request"],
                            "additionalProperties": false
                        },
                        "annotations": {
                            "readOnlyHint": false,
                            "destructiveHint": false,
                            "idempotentHint": false,
                            "openWorldHint": false
                        }
                    },
                    {
                        "name": "delete_request",
                        "title": "Delete Request",
                        "description": "Delete a request and its correspondence from the database.",
                        "inputSchema": {
                            "type": "object",
                            "description": "Request identifier to delete.",
                            "properties": {
                                "id": {
                                    "type": "integer",
                                    "description": "The request ID."
                                }
                            },
                            "required": ["id"],
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "Deletion result.",
                            "properties": {
                                "deleted": { "type": "boolean" },
                                "request_id": { "type": "integer" }
                            },
                            "required": ["deleted", "request_id"],
                            "additionalProperties": false
                        },
                        "annotations": {
                            "readOnlyHint": false,
                            "destructiveHint": true,
                            "idempotentHint": false,
                            "openWorldHint": false
                        }
                    },
                    {
                        "name": "list_authorities",
                        "title": "List Authorities",
                        "description": "List authorities stored in the database.",
                        "inputSchema": {
                            "type": "object",
                            "description": "No arguments are required.",
                            "properties": {},
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "A list of authorities.",
                            "properties": {
                                "authorities": {
                                    "type": "array",
                                    "items": { "type": "object" }
                                }
                            },
                            "required": ["authorities"],
                            "additionalProperties": false
                        },
                        "annotations": {
                            "readOnlyHint": true,
                            "destructiveHint": false,
                            "idempotentHint": true,
                            "openWorldHint": false
                        }
                    },
                    {
                        "name": "import_authorities",
                        "title": "Import Authorities",
                        "description": "Import or update authorities in the database.",
                        "inputSchema": {
                            "type": "object",
                            "description": "Authorities to import or update.",
                            "properties": {
                                "authorities": {
                                    "type": "array",
                                    "description": "Authority records to import.",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "slug": { "type": "string", "description": "Authority slug." },
                                            "name": { "type": "string", "description": "Authority name." },
                                            "url": { "type": "string", "description": "Optional authority URL." }
                                        },
                                        "required": ["slug", "name"],
                                        "additionalProperties": false
                                    }
                                }
                            },
                            "required": ["authorities"],
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "Import summary.",
                            "properties": {
                                "imported": { "type": "integer" }
                            },
                            "required": ["imported"],
                            "additionalProperties": false
                        },
                        "annotations": {
                            "readOnlyHint": false,
                            "destructiveHint": false,
                            "idempotentHint": true,
                            "openWorldHint": false
                        }
                    },
                    {
                        "name": "sync_monitor",
                        "title": "Sync Monitor",
                        "description": "Show sync status, queue depth, and latest sync time.",
                        "inputSchema": {
                            "type": "object",
                            "description": "No arguments are required.",
                            "properties": {},
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "Global synchronization monitor data.",
                            "properties": {
                                "sync": { "type": "object" },
                                "queue": { "type": "object" },
                                "offline_degradation": { "type": "object" }
                            },
                            "required": ["sync", "queue", "offline_degradation"],
                            "additionalProperties": false
                        },
                        "annotations": {
                            "readOnlyHint": true,
                            "destructiveHint": false,
                            "idempotentHint": true,
                            "openWorldHint": false
                        }
                    },
                    {
                        "name": "sync_conflicts",
                        "title": "Sync Conflicts",
                        "description": "List requests currently marked as sync conflicts.",
                        "inputSchema": {
                            "type": "object",
                            "description": "Optional filters for the conflict list.",
                            "properties": {
                                "limit": {
                                    "type": "integer",
                                    "description": "Maximum number of conflicts to return."
                                }
                            },
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "A conflict list payload.",
                            "properties": {
                                "conflicts": {
                                    "type": "array",
                                    "items": { "type": "object" }
                                }
                            },
                            "required": ["conflicts"],
                            "additionalProperties": false
                        },
                        "annotations": {
                            "readOnlyHint": true,
                            "destructiveHint": false,
                            "idempotentHint": true,
                            "openWorldHint": false
                        }
                    },
                    {
                        "name": "sync_resolve_conflict",
                        "title": "Resolve Sync Conflict",
                        "description": "Resolve a sync conflict as clean or dirty.",
                        "inputSchema": {
                            "type": "object",
                            "description": "Conflict resolution parameters.",
                            "properties": {
                                "request_id": {
                                    "type": "integer",
                                    "description": "The request ID to resolve."
                                },
                                "mark_clean": {
                                    "type": "boolean",
                                    "description": "Set true to mark the conflict clean; false keeps it dirty."
                                }
                            },
                            "required": ["request_id"],
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "Conflict resolution result.",
                            "properties": {
                                "request_id": { "type": "integer" },
                                "resolved": { "type": "boolean" },
                                "sync_status": { "type": "string" }
                            },
                            "required": ["request_id", "resolved", "sync_status"],
                            "additionalProperties": false
                        },
                        "annotations": {
                            "readOnlyHint": false,
                            "destructiveHint": false,
                            "idempotentHint": false,
                            "openWorldHint": false
                        }
                    },
                    {
                        "name": "sync_status",
                        "title": "Sync Status",
                        "description": "Read offline synchronization status globally or for one request.",
                        "inputSchema": {
                            "type": "object",
                            "description": "Optional request identifier for per-request sync metadata.",
                            "properties": {
                                "request_id": {
                                    "type": "integer",
                                    "description": "Optional request ID for per-request sync metadata."
                                }
                            },
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "Synchronization metadata.",
                            "properties": {
                                "request_id": { "type": "integer" },
                                "sync_status": { "type": ["string", "null"] }
                            },
                            "required": ["request_id", "sync_status"],
                            "additionalProperties": true
                        },
                        "annotations": {
                            "readOnlyHint": true,
                            "destructiveHint": false,
                            "idempotentHint": true,
                            "openWorldHint": false
                        }
                    },
                    {
                        "name": "check_status",
                        "title": "Check Status",
                        "description": "Check database health and summarize record counts.",
                        "inputSchema": {
                            "type": "object",
                            "description": "No arguments are required.",
                            "properties": {},
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "Database health summary.",
                            "properties": {
                                "status": { "type": "string" },
                                "database": { "type": "string" },
                                "metrics": { "type": "object" }
                            },
                            "required": ["status", "database", "metrics"],
                            "additionalProperties": false
                        },
                        "annotations": {
                            "readOnlyHint": true,
                            "destructiveHint": false,
                            "idempotentHint": true,
                            "openWorldHint": false
                        }
                    },
                    {
                        "name": "compute_deadline",
                        "title": "Compute Deadline",
                        "description": "Compute a statutory FOI/OIA deadline from start date and days.",
                        "inputSchema": {
                            "type": "object",
                            "description": "Deadline calculation parameters.",
                            "properties": {
                                "start_date": {
                                    "type": "string",
                                    "description": "Start date YYYY-MM-DD."
                                },
                                "days": {
                                    "type": "integer",
                                    "description": "Statutory day count."
                                },
                                "calendar": {
                                    "type": "boolean",
                                    "description": "Use calendar days instead of weekdays only."
                                }
                            },
                            "required": ["start_date", "days"],
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "Computed StatutoryDeadline.",
                            "properties": {
                                "start_date": { "type": "string" },
                                "due_date": { "type": "string" },
                                "statutory_deadline_days": { "type": "integer" },
                                "working_day_rule": { "type": "string" },
                                "instance_id": { "type": ["string", "null"] }
                            },
                            "required": [
                                "start_date",
                                "due_date",
                                "statutory_deadline_days",
                                "working_day_rule"
                            ],
                            "additionalProperties": false
                        },
                        "annotations": {
                            "readOnlyHint": true,
                            "destructiveHint": false,
                            "idempotentHint": true,
                            "openWorldHint": false
                        }
                    },
                    {
                        "name": "search_corpus",
                        "title": "Search Corpus",
                        "description": "Search a built-in sample FOI document corpus (demo index).",
                        "inputSchema": {
                            "type": "object",
                            "description": "Search parameters.",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Free-text query."
                                },
                                "limit": {
                                    "type": "integer",
                                    "description": "Maximum hits to return."
                                }
                            },
                            "required": ["query"],
                            "additionalProperties": false
                        },
                        "outputSchema": {
                            "type": "object",
                            "description": "Ranked search results over the demo corpus.",
                            "properties": {
                                "query": { "type": "string" },
                                "document_count": { "type": "integer" },
                                "hits": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "id": { "type": "string" },
                                            "score": { "type": "number" },
                                            "title": { "type": "string" }
                                        },
                                        "required": ["id", "score", "title"],
                                        "additionalProperties": false
                                    }
                                }
                            },
                            "required": ["query", "document_count", "hits"],
                            "additionalProperties": false
                        },
                        "annotations": {
                            "readOnlyHint": true,
                            "destructiveHint": false,
                            "idempotentHint": true,
                            "openWorldHint": false
                        }
                    }
                ]
            });
            tools["tools"].as_array_mut().unwrap().push(json!({
                "name": "endorsed_route_status",
                "title": "Endorsed Route Status",
                "description": "Evaluate an operator-published endorsed-client capability document locally. Read-only, fail-closed, and never enables a route or contacts a remote service.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "capabilities": { "type": "object" },
                        "client_id": { "type": "string", "minLength": 1 },
                        "scopes": { "type": "array", "items": { "type": "string" } },
                        "now_epoch": { "type": "integer", "minimum": 0 },
                        "bulk_export": { "type": "boolean", "default": false }
                    },
                    "required": ["capabilities", "client_id", "scopes", "now_epoch"],
                    "additionalProperties": false
                },
                "outputSchema": {
                    "type": "object",
                    "properties": {
                        "status": { "type": "string", "enum": ["authorized", "denied"] },
                        "reason": { "type": "string" },
                        "instance_id": { "type": "string" },
                        "scopes": { "type": "array", "items": { "type": "string" } },
                        "quotas": { "type": "object" },
                        "bulk_export": { "type": ["object", "null"] }
                    },
                    "required": ["status"],
                    "additionalProperties": false
                },
                "annotations": {
                    "readOnlyHint": true,
                    "destructiveHint": false,
                    "idempotentHint": true,
                    "openWorldHint": false
                }
            }));
            enrich_tool_definitions(&mut tools);
            if remote_policy.status().remote_enabled {
                tools["tools"]
                    .as_array_mut()
                    .expect("tools is an array")
                    .extend([
                        remote_tool_definition("remote_health", "Read-only remote health probe"),
                        remote_tool_definition("remote_version", "Read-only remote API version"),
                        remote_tool_definition(
                            "remote_search_requests",
                            "Bounded remote request search",
                        ),
                        remote_tool_definition(
                            "remote_get_request",
                            "Read-only remote request retrieval",
                        ),
                        remote_tool_definition(
                            "remote_list_authorities",
                            "Bounded remote authority discovery",
                        ),
                    ]);
                if remote_policy
                    .status()
                    .instances
                    .iter()
                    .any(|instance| instance.write_enabled)
                {
                    tools["tools"]
                        .as_array_mut()
                        .expect("tools is an array")
                        .extend([
                            remote_tool_definition(
                                "remote_create_request",
                                "Prepare or commit a governed remote request creation",
                            ),
                            remote_tool_definition(
                                "remote_add_correspondence",
                                "Prepare or commit governed remote correspondence",
                            ),
                            remote_tool_definition(
                                "remote_update_state",
                                "Prepare or commit an optimistic remote state update",
                            ),
                        ]);
                }
            }
            Some(JsonRpcResponse::success(req.id, tools))
        }
        "resources/list" => Some(JsonRpcResponse::success(
            req.id,
            list_mcp_resources(db).await,
        )),
        "resources/read" => {
            let uri = req
                .params
                .as_ref()
                .and_then(|p| p.get("uri"))
                .and_then(|u| u.as_str())
                .unwrap_or("");
            if uri.is_empty() {
                return Some(JsonRpcResponse::error(
                    req.id,
                    -32602,
                    "Missing resource uri".to_string(),
                ));
            }
            match read_mcp_resource(db, uri).await {
                Ok(contents) => Some(JsonRpcResponse::success(
                    req.id,
                    json!({ "contents": contents }),
                )),
                Err(message) => Some(JsonRpcResponse::error(req.id, -32002, message)),
            }
        }
        "prompts/list" => Some(JsonRpcResponse::success(
            req.id,
            json!({
                "prompts": [
                    {
                        "name": "operator_daily_triage",
                        "title": "Operator daily triage",
                        "description": "Triage local FOI/OIA requests: health check, list newest requests, flag conflicts, and propose next actions.",
                        "arguments": []
                    },
                    {
                        "name": "draft_foi_request",
                        "title": "Draft FOI/OIA request",
                        "description": "Help draft a local FOI/OIA request body for a public authority using jurisdiction-aware language.",
                        "arguments": [
                            {
                                "name": "authority_name",
                                "description": "Name of the public authority receiving the request.",
                                "required": true
                            },
                            {
                                "name": "subject",
                                "description": "Short subject or topic of the request.",
                                "required": true
                            }
                        ]
                    },
                    {
                        "name": "sync_health_review",
                        "title": "Offline sync health review",
                        "description": "Review offline sync queue depth, conflicts, and degradation indicators before reconciling.",
                        "arguments": []
                    }
                ]
            }),
        )),
        "prompts/get" => {
            let name = req
                .params
                .as_ref()
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("");
            let arguments = req
                .params
                .as_ref()
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or(json!({}));
            let prompt = match name {
                "operator_daily_triage" => json!({
                    "description": "Daily triage of the local FOI tracker database.",
                    "messages": [{
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "Run an operator daily triage on the local fyi-mcp database.\n1) Call check_status.\n2) Call list_requests (limit 25).\n3) Call sync_monitor and sync_conflicts if dirty/conflict counts are non-zero.\n4) Summarize overdue-looking statuses and recommend concrete next tools (retrieve_request, update_request, sync_resolve_conflict).\nDo not invent remote submissions."
                        }
                    }]
                }),
                "draft_foi_request" => {
                    let authority = arguments
                        .get("authority_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("the public authority");
                    let subject = arguments
                        .get("subject")
                        .and_then(|v| v.as_str())
                        .unwrap_or("the requested records");
                    json!({
                        "description": "Draft a clear FOI/OIA request for a named authority.",
                        "messages": [{
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": format!(
                                    "Draft a clear, specific FOI/OIA request to {} about {}.\nUse list_authorities if authority context is needed, then create_request with title and body only after the draft is ready.\nPrefer precise document descriptions, date ranges, and format preferences. Do not claim the request was submitted remotely.",
                                    authority, subject
                                )
                            }
                        }]
                    })
                }
                "sync_health_review" => json!({
                    "description": "Review offline synchronization health.",
                    "messages": [{
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "Review offline sync health for fyi-mcp.\n1) call sync_monitor\n2) if conflicts > 0, call sync_conflicts\n3) for each conflict, retrieve_request then decide mark_clean via sync_resolve_conflict only after reconciliation notes\n4) summarize queue pending/failed and dirty_records.\nDo not push remote changes from this prompt."
                        }
                    }]
                }),
                _ => {
                    return Some(JsonRpcResponse::error(
                        req.id,
                        -32602,
                        format!("Unknown prompt: {}", name),
                    ));
                }
            };
            Some(JsonRpcResponse::success(
                req.id,
                json!({
                    "description": prompt.get("description").cloned().unwrap_or(json!("")),
                    "messages": prompt.get("messages").cloned().unwrap_or(json!([]))
                }),
            ))
        }
        "tools/call" => {
            let params = match req.params.as_ref() {
                Some(p) => p,
                None => {
                    return Some(JsonRpcResponse::error(
                        req.id,
                        -32602,
                        "Missing parameters".to_string(),
                    ))
                }
            };

            let name = match params.get("name").and_then(|n| n.as_str()) {
                Some(n) => n,
                None => {
                    return Some(JsonRpcResponse::error(
                        req.id,
                        -32602,
                        "Missing tool name".to_string(),
                    ))
                }
            };

            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

            match name {
                "list_requests" => {
                    let limit = arguments
                        .get("limit")
                        .and_then(|i| i.as_i64())
                        .unwrap_or(100);

                    match db.list_requests(limit).await {
                        Ok(requests) => Some(tool_success(
                            req.id,
                            json!({
                                "requests": requests
                            }),
                        )),
                        Err(e) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Failed to fetch requests: {}", e)
                                    }
                                ]
                            }),
                        )),
                    }
                }
                "endorsed_route_status" => match endorsed_route_status(&arguments) {
                    Ok(payload) => Some(tool_success(req.id, payload)),
                    Err(error) => Some(JsonRpcResponse::error(req.id, -32602, error)),
                },
                "sync_status" => {
                    if let Some(request_id) = arguments.get("request_id").and_then(|id| id.as_i64())
                    {
                        match db.get_request_sync_metadata(request_id).await {
                            Ok(metadata) => {
                                let payload = metadata
                                    .map(|metadata| {
                                        json!({
                                            "request_id": metadata.request_id,
                                            "sync_status": metadata.sync_status.as_str(),
                                            "last_synced_at": metadata.last_synced_at,
                                            "remote_updated_at": metadata.remote_updated_at,
                                            "local_updated_at": metadata.local_updated_at,
                                            "conflict_version": metadata.conflict_version
                                        })
                                    })
                                    .unwrap_or_else(|| {
                                        json!({
                                            "request_id": request_id,
                                            "sync_status": Value::Null
                                        })
                                    });
                                Some(tool_success(req.id, payload))
                            }
                            Err(e) => Some(JsonRpcResponse::success(
                                req.id,
                                json!({
                                    "isError": true,
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": format!("Failed to fetch sync status: {}", e)
                                        }
                                    ]
                                }),
                            )),
                        }
                    } else {
                        match db.get_global_sync_status().await {
                            Ok(status) => Some(tool_success(
                                req.id,
                                json!({
                                    "total": status.total,
                                    "clean": status.clean,
                                    "dirty": status.dirty,
                                    "pending": status.pending,
                                    "conflict": status.conflict
                                }),
                            )),
                            Err(e) => Some(JsonRpcResponse::success(
                                req.id,
                                json!({
                                    "isError": true,
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": format!("Failed to fetch sync status: {}", e)
                                        }
                                    ]
                                }),
                            )),
                        }
                    }
                }
                "sync_monitor" => {
                    let status = db.get_global_sync_status().await;
                    let queue = db.get_outgoing_queue_depth().await;
                    let latest = db.get_latest_sync_timestamp().await;
                    match (status, queue, latest) {
                        (Ok(status), Ok(queue), Ok(latest_sync)) => Some(tool_success(
                            req.id,
                            json!({
                                "sync": {
                                    "total": status.total,
                                    "clean": status.clean,
                                    "dirty": status.dirty,
                                    "pending": status.pending,
                                    "conflict": status.conflict,
                                    "latest_sync": latest_sync
                                },
                                "queue": {
                                    "pending": queue.pending,
                                    "submitted": queue.submitted,
                                    "failed": queue.failed
                                },
                                "offline_degradation": {
                                    "queued_changes": queue.pending + queue.failed,
                                    "dirty_records": status.dirty
                                }
                            }),
                        )),
                        (status, queue, latest) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!(
                                            "Failed to fetch sync monitor: status={:?}, queue={:?}, latest={:?}",
                                            status.err(),
                                            queue.err(),
                                            latest.err()
                                        )
                                    }
                                ]
                            }),
                        )),
                    }
                }
                "sync_conflicts" => {
                    let limit = arguments
                        .get("limit")
                        .and_then(|value| value.as_i64())
                        .unwrap_or(100);
                    match db.list_conflicted_requests(limit).await {
                        Ok(conflicts) => Some(tool_success(
                            req.id,
                            json!({
                                "conflicts": conflicts
                            }),
                        )),
                        Err(e) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Failed to fetch sync conflicts: {}", e)
                                    }
                                ]
                            }),
                        )),
                    }
                }
                "sync_resolve_conflict" => {
                    let request_id = match arguments.get("request_id").and_then(|id| id.as_i64()) {
                        Some(id) => id,
                        None => {
                            return Some(JsonRpcResponse::error(
                                req.id,
                                -32602,
                                "Invalid or missing 'request_id' argument".to_string(),
                            ))
                        }
                    };
                    let mark_clean = arguments
                        .get("mark_clean")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    match db.resolve_request_conflict(request_id, mark_clean).await {
                        Ok(resolved) => Some(tool_success(
                            req.id,
                            json!({
                                "request_id": request_id,
                                "resolved": resolved,
                                "sync_status": if mark_clean { "clean" } else { "dirty" }
                            }),
                        )),
                        Err(e) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Failed to resolve sync conflict: {}", e)
                                    }
                                ]
                            }),
                        )),
                    }
                }
                "retrieve_request" => {
                    let id = match arguments.get("id").and_then(|i| i.as_i64()) {
                        Some(i) => i,
                        None => {
                            return Some(JsonRpcResponse::error(
                                req.id,
                                -32602,
                                "Invalid or missing 'id' argument".to_string(),
                            ))
                        }
                    };

                    match db.get_request(id).await {
                        Ok(Some(request)) => {
                            let correspondence = db
                                .get_correspondence_for_request(id)
                                .await
                                .unwrap_or_default();
                            let result_val = json!({
                                "request": request,
                                "correspondence": correspondence
                            });
                            Some(tool_success(req.id, result_val))
                        }
                        Ok(None) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Request with ID {} not found", id)
                                    }
                                ]
                            }),
                        )),
                        Err(e) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Database error: {}", e)
                                    }
                                ]
                            }),
                        )),
                    }
                }
                "create_request" => {
                    let title = match arguments.get("title").and_then(|t| t.as_str()) {
                        Some(t) => t.to_string(),
                        None => {
                            return Some(JsonRpcResponse::error(
                                req.id,
                                -32602,
                                "Missing 'title'".to_string(),
                            ))
                        }
                    };
                    let body = match arguments.get("body").and_then(|b| b.as_str()) {
                        Some(b) => b.to_string(),
                        None => {
                            return Some(JsonRpcResponse::error(
                                req.id,
                                -32602,
                                "Missing 'body'".to_string(),
                            ))
                        }
                    };
                    let user_name = arguments
                        .get("user_name")
                        .and_then(|u| u.as_str())
                        .map(String::from);
                    let status = arguments
                        .get("status")
                        .and_then(|s| s.as_str())
                        .map(String::from);
                    let url = arguments
                        .get("url")
                        .and_then(|u| u.as_str())
                        .map(String::from);
                    let tags = arguments.get("tags").and_then(|t| {
                        if let Some(arr) = t.as_array() {
                            let parsed: Vec<String> = arr
                                .iter()
                                .filter_map(|val| val.as_str().map(String::from))
                                .collect();
                            Some(parsed)
                        } else {
                            None
                        }
                    });

                    let id = get_next_request_id(db.pool()).await;
                    let new_req = AlaveteliRequest {
                        id,
                        title,
                        body,
                        user_name,
                        status,
                        created_at: Some(chrono::Utc::now().to_rfc3339()),
                        updated_at: Some(chrono::Utc::now().to_rfc3339()),
                        url,
                        tags,
                    };

                    match db.insert_request(&new_req).await {
                        Ok(_) => Some(tool_success(
                            req.id,
                            json!({
                                "request": new_req
                            }),
                        )),
                        Err(e) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Failed to insert request: {}", e)
                                    }
                                ]
                            }),
                        )),
                    }
                }
                "update_request" => {
                    let id = match arguments.get("id").and_then(|i| i.as_i64()) {
                        Some(i) => i,
                        None => {
                            return Some(JsonRpcResponse::error(
                                req.id,
                                -32602,
                                "Invalid or missing 'id' argument".to_string(),
                            ))
                        }
                    };
                    let title = match arguments.get("title").and_then(|t| t.as_str()) {
                        Some(t) => t.to_string(),
                        None => {
                            return Some(JsonRpcResponse::error(
                                req.id,
                                -32602,
                                "Missing 'title'".to_string(),
                            ))
                        }
                    };
                    let body = match arguments.get("body").and_then(|b| b.as_str()) {
                        Some(b) => b.to_string(),
                        None => {
                            return Some(JsonRpcResponse::error(
                                req.id,
                                -32602,
                                "Missing 'body'".to_string(),
                            ))
                        }
                    };
                    let existing = match db.get_request(id).await {
                        Ok(Some(request)) => request,
                        Ok(None) => {
                            return Some(JsonRpcResponse::success(
                                req.id,
                                json!({
                                    "isError": true,
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": format!("Request with ID {} not found", id)
                                        }
                                    ]
                                }),
                            ))
                        }
                        Err(e) => {
                            return Some(JsonRpcResponse::success(
                                req.id,
                                json!({
                                    "isError": true,
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": format!("Database error: {}", e)
                                        }
                                    ]
                                }),
                            ))
                        }
                    };
                    let tags = arguments.get("tags").and_then(|t| {
                        t.as_array().map(|arr| {
                            arr.iter()
                                .filter_map(|val| val.as_str().map(String::from))
                                .collect::<Vec<String>>()
                        })
                    });
                    let updated = AlaveteliRequest {
                        id,
                        title,
                        body,
                        user_name: arguments
                            .get("user_name")
                            .and_then(|u| u.as_str())
                            .map(String::from),
                        status: arguments
                            .get("status")
                            .and_then(|s| s.as_str())
                            .map(String::from),
                        created_at: existing.created_at,
                        updated_at: Some(chrono::Utc::now().to_rfc3339()),
                        url: arguments
                            .get("url")
                            .and_then(|u| u.as_str())
                            .map(String::from),
                        tags,
                    };

                    match db.update_request(&updated).await {
                        Ok(true) => Some(tool_success(
                            req.id,
                            json!({
                                "request": updated
                            }),
                        )),
                        Ok(false) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Request with ID {} not found", id)
                                    }
                                ]
                            }),
                        )),
                        Err(e) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Failed to update request: {}", e)
                                    }
                                ]
                            }),
                        )),
                    }
                }
                "delete_request" => {
                    let id = match arguments.get("id").and_then(|i| i.as_i64()) {
                        Some(i) => i,
                        None => {
                            return Some(JsonRpcResponse::error(
                                req.id,
                                -32602,
                                "Invalid or missing 'id' argument".to_string(),
                            ))
                        }
                    };

                    match db.delete_request(id).await {
                        Ok(true) => Some(tool_success(
                            req.id,
                            json!({
                                "deleted": true,
                                "request_id": id
                            }),
                        )),
                        Ok(false) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Request with ID {} not found", id)
                                    }
                                ]
                            }),
                        )),
                        Err(e) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Failed to delete request: {}", e)
                                    }
                                ]
                            }),
                        )),
                    }
                }
                "list_authorities" => {
                    if let Err(e) = ensure_authorities_table(db.pool()).await {
                        return Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Database error ensuring authorities table: {}", e)
                                    }
                                ]
                            }),
                        ));
                    }

                    match sqlx::query_as::<_, (String, String, Option<String>)>(
                        "SELECT slug, name, url FROM authorities ORDER BY name ASC",
                    )
                    .fetch_all(db.pool())
                    .await
                    {
                        Ok(rows) => {
                            let authorities: Vec<Authority> = rows
                                .into_iter()
                                .map(|(slug, name, url)| Authority { slug, name, url })
                                .collect();
                            Some(tool_success(
                                req.id,
                                json!({
                                    "authorities": authorities
                                }),
                            ))
                        }
                        Err(e) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Failed to fetch authorities: {}", e)
                                    }
                                ]
                            }),
                        )),
                    }
                }
                "import_authorities" => {
                    if let Err(e) = ensure_authorities_table(db.pool()).await {
                        return Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Database error ensuring authorities table: {}", e)
                                    }
                                ]
                            }),
                        ));
                    }

                    let authorities: Vec<Authority> = match serde_json::from_value(
                        arguments.get("authorities").cloned().unwrap_or(json!([])),
                    ) {
                        Ok(authorities) => authorities,
                        Err(e) => {
                            return Some(JsonRpcResponse::success(
                                req.id,
                                json!({
                                    "isError": true,
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": format!("Invalid authorities payload: {}", e)
                                        }
                                    ]
                                }),
                            ))
                        }
                    };

                    let mut imported = 0;
                    for authority in authorities {
                        let slug = authority.slug.trim();
                        let name = authority.name.trim();
                        if slug.is_empty() || name.is_empty() {
                            return Some(JsonRpcResponse::success(
                                req.id,
                                json!({
                                    "isError": true,
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": "Authority slug and name are required"
                                        }
                                    ]
                                }),
                            ));
                        }

                        let url = authority
                            .url
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty());
                        if let Err(e) = sqlx::query(
                            "INSERT INTO authorities (slug, name, url) VALUES (?, ?, ?) \
                             ON CONFLICT(slug) DO UPDATE SET name = excluded.name, url = excluded.url",
                        )
                        .bind(slug)
                        .bind(name)
                        .bind(url)
                        .execute(db.pool())
                        .await
                        {
                            return Some(JsonRpcResponse::success(
                                req.id,
                                json!({
                                    "isError": true,
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": format!("Failed to import authorities: {}", e)
                                        }
                                    ]
                                })
                            ));
                        }

                        imported += 1;
                    }

                    Some(tool_success(
                        req.id,
                        json!({
                            "imported": ImportAuthoritiesResult { imported }.imported
                        }),
                    ))
                }
                "check_status" => {
                    let db_healthy = sqlx::query("SELECT 1").execute(db.pool()).await.is_ok();

                    let total_requests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM requests")
                        .fetch_one(db.pool())
                        .await
                        .unwrap_or(0);

                    let total_correspondence: i64 =
                        sqlx::query_scalar("SELECT COUNT(*) FROM correspondence")
                            .fetch_one(db.pool())
                            .await
                            .unwrap_or(0);

                    let status_info = json!({
                        "status": if db_healthy { "healthy" } else { "unhealthy" },
                        "database": if db_healthy { "connected" } else { "disconnected" },
                        "metrics": {
                            "total_requests": total_requests,
                            "total_correspondence": total_correspondence,
                            "sync": db.get_global_sync_status().await.ok().map(|sync| {
                                json!({
                                    "total": sync.total,
                                    "clean": sync.clean,
                                    "dirty": sync.dirty,
                                    "pending": sync.pending,
                                    "conflict": sync.conflict
                                })
                            })
                        }
                    });

                    Some(tool_success(req.id, status_info))
                }
                "compute_deadline" => {
                    let start_raw = arguments
                        .get("start_date")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let days = arguments.get("days").and_then(|v| v.as_u64());
                    let calendar = arguments
                        .get("calendar")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let start_date = match NaiveDate::parse_from_str(start_raw, "%Y-%m-%d") {
                        Ok(d) => d,
                        Err(e) => {
                            return Some(JsonRpcResponse::success(
                                req.id,
                                json!({
                                    "isError": true,
                                    "content": [{
                                        "type": "text",
                                        "text": format!(
                                            "Invalid start_date '{start_raw}' (expected YYYY-MM-DD): {e}"
                                        )
                                    }]
                                }),
                            ));
                        }
                    };
                    let Some(days) = days else {
                        return Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [{
                                    "type": "text",
                                    "text": "Missing or invalid days (non-negative integer required)"
                                }]
                            }),
                        ));
                    };
                    let days = days as u32;
                    let rule = if calendar {
                        WorkingDayRule::CalendarDays
                    } else {
                        WorkingDayRule::WeekdaysOnly
                    };
                    let deadline =
                        calculate_deadline(&DeadlineInput::new(start_date, days).with_rule(rule));
                    match serde_json::to_value(&deadline) {
                        Ok(payload) => Some(tool_success(req.id, payload)),
                        Err(e) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [{
                                    "type": "text",
                                    "text": format!("Failed to serialize deadline: {e}")
                                }]
                            }),
                        )),
                    }
                }
                "search_corpus" => {
                    let query = arguments
                        .get("query")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim();
                    if query.is_empty() {
                        return Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "isError": true,
                                "content": [{
                                    "type": "text",
                                    "text": "query is required and must be non-empty"
                                }]
                            }),
                        ));
                    }
                    let limit = arguments
                        .get("limit")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(10)
                        .clamp(1, 50) as usize;
                    let index = sample_search_corpus();
                    let hits = index.search(query, limit);
                    Some(tool_success(
                        req.id,
                        json!({
                            "query": query,
                            "document_count": index.document_count(),
                            "hits": hits,
                        }),
                    ))
                }
                "remote_health"
                | "remote_version"
                | "remote_search_requests"
                | "remote_get_request"
                | "remote_list_authorities" => {
                    let instance_id = arguments
                        .get("instance_id")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let correlation_id = format!(
                        "mcp-{}",
                        req.id
                            .as_ref()
                            .map(Value::to_string)
                            .unwrap_or_else(|| "notification".into())
                    );
                    match remote_read(
                        remote_policy,
                        instance_id,
                        name,
                        &arguments,
                        &correlation_id,
                    )
                    .await
                    {
                        Ok(payload) => Some(tool_success(req.id, payload)),
                        Err(error) => Some(JsonRpcResponse::error(req.id, -32003, error)),
                    }
                }
                "remote_create_request" | "remote_add_correspondence" | "remote_update_state" => {
                    let instance_id = arguments
                        .get("instance_id")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let phase = arguments
                        .get("phase")
                        .and_then(Value::as_str)
                        .unwrap_or("prepare");
                    let result = if phase == "commit" {
                        remote_commit_write(
                            remote_policy,
                            write_gate,
                            instance_id,
                            name,
                            &arguments,
                        )
                        .await
                    } else {
                        write_gate.prepare(remote_policy, instance_id, name, &arguments)
                    };
                    match result {
                        Ok(payload) => Some(tool_success(req.id, payload)),
                        Err(error) => Some(JsonRpcResponse::error(req.id, -32004, error)),
                    }
                }
                _ => Some(JsonRpcResponse::error(
                    req.id,
                    -32601,
                    format!("Tool '{}' not found", name),
                )),
            }
        }
        _ => Some(JsonRpcResponse::error(
            req.id,
            -32601,
            format!("Method '{}' not found", req.method),
        )),
    }
}

async fn remote_read(
    policy: &RemoteMcpPolicy,
    instance_id: &str,
    operation: &str,
    arguments: &Value,
    correlation_id: &str,
) -> Result<Value, String> {
    policy
        .authorize(instance_id, policy::RemoteCapability::Read)
        .map_err(|error| error.to_string())?;
    let instance = policy
        .instance(instance_id)
        .ok_or_else(|| "remote instance is not configured".to_string())?;
    let client = SyncClient::new(&instance.base_url)
        .map_err(|_| "remote client could not be initialized".to_string())?;
    let result = match operation {
        "remote_health" => {
            let health = client.health_check().await;
            Ok(
                json!({"instance_id": instance_id, "network_reachable": health.network_reachable, "api_reachable": health.api_reachable}),
            )
        }
        "remote_version" => client
            .get_api_version()
            .await
            .map(|version| json!({"instance_id": instance_id, "version": version}))
            .map_err(|_| "remote operation failed".to_string()),
        "remote_search_requests" => {
            let query = arguments
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            if query.is_empty() || query.len() > 200 {
                return Err("query must be between 1 and 200 characters".into());
            }
            let limit = arguments
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(10)
                .clamp(1, 50) as usize;
            client
                .search_requests_with_options(&SearchOptions { query: Some(query.into()), per_page: Some(limit as u32), ..Default::default() })
                .await
                .map(|requests| json!({"instance_id": instance_id, "query": query, "requests": requests.into_iter().take(limit).collect::<Vec<_>>() }))
                .map_err(|_| "remote operation failed".to_string())
        }
        "remote_get_request" => {
            let request_id = arguments
                .get("request_id")
                .and_then(Value::as_i64)
                .ok_or_else(|| "request_id must be a positive integer".to_string())?;
            if request_id < 1 {
                return Err("request_id must be a positive integer".into());
            }
            client
                .fetch_request(request_id)
                .await
                .map(|request| json!({"instance_id": instance_id, "request": request}))
                .map_err(|_| "remote operation failed".to_string())
        }
        "remote_list_authorities" => {
            let filter = arguments
                .get("filter")
                .and_then(Value::as_str)
                .unwrap_or("");
            if filter.len() > 100 {
                return Err("filter exceeds 100 characters".into());
            }
            let limit = arguments
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(50)
                .clamp(1, 200) as usize;
            client.list_authorities_matching(filter).await.map(|authorities| json!({"instance_id": instance_id, "authorities": authorities.into_iter().take(limit).collect::<Vec<_>>() })).map_err(|_| "remote operation failed".to_string())
        }
        _ => Err("unsupported remote read operation".to_string()),
    };
    match result {
        Ok(payload) => {
            let _ = policy.record_request(
                instance_id,
                serde_json::to_vec(&payload)
                    .map(|v| v.len() as u64)
                    .unwrap_or(0),
            );
            tracing::info!(
                correlation_id,
                operation,
                instance_id,
                outcome = "success",
                "remote MCP read completed"
            );
            Ok(payload)
        }
        Err(_) => {
            policy.record_failure();
            tracing::warn!(
                correlation_id,
                operation,
                instance_id,
                outcome = "failure",
                "remote MCP read failed"
            );
            Err("remote operation failed; see operator audit telemetry".into())
        }
    }
}

async fn remote_commit_write(
    policy: &RemoteMcpPolicy,
    gate: &RemoteWriteGate,
    instance_id: &str,
    operation: &str,
    arguments: &Value,
) -> Result<Value, String> {
    policy
        .authorize(instance_id, policy::RemoteCapability::Write)
        .map_err(|error| error.to_string())?;
    let token = arguments
        .get("confirmation_token")
        .and_then(Value::as_str)
        .ok_or_else(|| "confirmation_token is required for commit".to_string())?;
    let prepared = gate.take(token)?;
    if prepared.instance_id != instance_id || prepared.operation != operation {
        return Err("confirmation token does not match this write operation".into());
    }
    let instance = policy
        .instance(instance_id)
        .ok_or_else(|| "remote instance is not configured".to_string())?;
    let token_env = format!(
        "FYI_MCP_REMOTE_BOT_TOKEN_{}",
        instance_id.to_ascii_uppercase().replace('-', "_")
    );
    let bot_token = std::env::var(token_env).ok();
    let client = SyncClient::new(&instance.base_url)
        .map_err(|_| "remote client could not be initialized".to_string())?
        .with_bot_token(bot_token);
    let result = match operation {
        "remote_create_request" => {
            let payload: CreateRequestPayload = serde_json::from_value(prepared.arguments.clone())
                .map_err(|_| "remote create payload is invalid".to_string())?;
            client
                .create_request(&payload)
                .await
                .map(|response| json!({"instance_id": instance_id, "request": response}))
                .map_err(|_| "remote operation failed".to_string())
        }
        "remote_add_correspondence" => {
            let request_id = prepared
                .arguments
                .get("request_id")
                .and_then(Value::as_i64)
                .ok_or_else(|| "request_id is required".to_string())?;
            let direction = prepared
                .arguments
                .get("direction")
                .and_then(Value::as_str)
                .ok_or_else(|| "direction is required".to_string())?;
            let direction: CorrespondenceDirection = serde_json::from_value(json!(direction))
                .map_err(|_| "direction is invalid".to_string())?;
            let payload = AddCorrespondencePayload {
                direction,
                body: prepared
                    .arguments
                    .get("body")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "body is required".to_string())?
                    .to_string(),
                sent_at: prepared
                    .arguments
                    .get("sent_at")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "sent_at is required".to_string())?
                    .to_string(),
                state: prepared
                    .arguments
                    .get("state")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            };
            let attachments = validated_attachment_paths(&prepared.arguments).await?;
            if attachments.is_empty() {
                client
                    .add_correspondence(request_id, &payload)
                    .await
                    .map(|response| json!({"instance_id": instance_id, "correspondence": response}))
                    .map_err(|_| "remote operation failed".to_string())
            } else {
                client
                    .add_correspondence_with_attachments(request_id, &payload, &attachments)
                    .await
                    .map(|response| json!({"instance_id": instance_id, "correspondence": response}))
                    .map_err(|_| "remote operation failed".to_string())
            }
        }
        "remote_update_state" => {
            let request_id = prepared
                .arguments
                .get("request_id")
                .and_then(Value::as_i64)
                .ok_or_else(|| "request_id is required".to_string())?;
            let expected = prepared
                .arguments
                .get("expected_current_state")
                .and_then(Value::as_str)
                .ok_or_else(|| "expected_current_state is required".to_string())?;
            let state = prepared
                .arguments
                .get("state")
                .and_then(Value::as_str)
                .ok_or_else(|| "state is required".to_string())?;
            client
                .update_request_state_if_current(
                    request_id,
                    expected,
                    &UpdateRequestStatePayload {
                        state: state.to_string(),
                    },
                )
                .await
                .map(|response| json!({"instance_id": instance_id, "state": response}))
                .map_err(|_| "remote operation failed".to_string())
        }
        _ => Err("unsupported remote write operation".to_string()),
    };
    match result {
        Ok(payload) => {
            gate.remember_result(prepared.idempotency_key, payload.clone());
            let _ = policy.record_request(
                instance_id,
                serde_json::to_vec(&payload)
                    .map(|v| v.len() as u64)
                    .unwrap_or(0),
            );
            Ok(payload)
        }
        Err(_) => {
            policy.record_failure();
            Err("remote write failed; see operator audit telemetry".into())
        }
    }
}

async fn validated_attachment_paths(arguments: &Value) -> Result<Vec<PathBuf>, String> {
    let Some(values) = arguments.get("attachments") else {
        return Ok(Vec::new());
    };
    let values = values
        .as_array()
        .ok_or_else(|| "attachments must be an array".to_string())?;
    if values.len() > 8 {
        return Err("attachments exceed the 8 file limit".into());
    }
    let root = std::env::var_os("FYI_MCP_ATTACHMENT_ROOT")
        .ok_or_else(|| "FYI_MCP_ATTACHMENT_ROOT is required for attachment writes".to_string())?;
    let root = tokio::fs::canonicalize(root)
        .await
        .map_err(|_| "attachment root is unavailable".to_string())?;
    let mut paths = Vec::with_capacity(values.len());
    for value in values {
        let relative = value
            .as_str()
            .ok_or_else(|| "attachment paths must be strings".to_string())?;
        if relative.is_empty()
            || relative.len() > 1024
            || relative.contains('\0')
            || relative.contains("..")
        {
            return Err("attachment path is invalid".into());
        }
        let canonical = tokio::fs::canonicalize(root.join(relative))
            .await
            .map_err(|_| "attachment path is unavailable".to_string())?;
        if !canonical.starts_with(&root) {
            return Err("attachment path escapes FYI_MCP_ATTACHMENT_ROOT".into());
        }
        paths.push(canonical);
    }
    Ok(paths)
}

/// Built-in demo documents for the experimental `search_corpus` tool.
fn sample_search_corpus() -> InMemorySearchIndex {
    let mut index = InMemorySearchIndex::new();
    index.index_document(SearchDocument {
        id: "1".into(),
        title: "Budget procurement contracts".into(),
        body: "Request for copies of all procurement contracts awarded in 2024.".into(),
    });
    index.index_document(SearchDocument {
        id: "2".into(),
        title: "Police body camera policy".into(),
        body: "Please provide the operational policy for body-worn cameras.".into(),
    });
    index.index_document(SearchDocument {
        id: "3".into(),
        title: "Hospital waiting times".into(),
        body: "Monthly waiting list statistics for elective surgery.".into(),
    });
    index
}

/// Static + dynamic MCP resource catalog for corpus exposure.
///
/// URIs:
/// - `fyi://authorities` — imported public authorities
/// - `fyi://requests` — request index (ids + titles)
/// - `fyi://requests/{id}` — single request document
pub async fn list_mcp_resources(db: &DbPool) -> Value {
    let mut resources = vec![
        json!({
            "uri": "fyi://authorities",
            "name": "Public authorities",
            "description": "Imported public authority records used for FOI routing.",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "fyi://requests",
            "name": "Request index",
            "description": "Index of locally tracked FYI/Alaveteli requests.",
            "mimeType": "application/json"
        }),
    ];

    if let Ok(requests) = db.list_requests(500).await {
        for request in requests {
            resources.push(json!({
                "uri": format!("fyi://requests/{}", request.id),
                "name": format!("Request {}", request.id),
                "description": request.title,
                "mimeType": "application/json"
            }));
        }
    }

    json!({ "resources": resources })
}

/// Read a single MCP resource by URI into MCP content items.
pub async fn read_mcp_resource(db: &DbPool, uri: &str) -> Result<Vec<Value>, String> {
    if uri == "fyi://authorities" {
        ensure_authorities_table(db.pool())
            .await
            .map_err(|e| format!("Failed to ensure authorities table: {e}"))?;
        let rows = sqlx::query_as::<_, (String, String, Option<String>)>(
            "SELECT slug, name, url FROM authorities ORDER BY name",
        )
        .fetch_all(db.pool())
        .await
        .map_err(|e| format!("Failed to list authorities: {e}"))?;
        let authorities: Vec<Authority> = rows
            .into_iter()
            .map(|(slug, name, url)| Authority { slug, name, url })
            .collect();
        let text = serde_json::to_string_pretty(&authorities)
            .map_err(|e| format!("Failed to serialize authorities: {e}"))?;
        return Ok(vec![json!({
            "uri": uri,
            "mimeType": "application/json",
            "text": text
        })]);
    }

    if uri == "fyi://requests" {
        let requests = db
            .list_requests(500)
            .await
            .map_err(|e| format!("Failed to list requests: {e}"))?;
        let index: Vec<Value> = requests
            .into_iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "title": r.title,
                    "status": r.status,
                    "uri": format!("fyi://requests/{}", r.id)
                })
            })
            .collect();
        let text = serde_json::to_string_pretty(&index)
            .map_err(|e| format!("Failed to serialize request index: {e}"))?;
        return Ok(vec![json!({
            "uri": uri,
            "mimeType": "application/json",
            "text": text
        })]);
    }

    if let Some(id_str) = uri.strip_prefix("fyi://requests/") {
        let id: i64 = id_str
            .parse()
            .map_err(|_| format!("Invalid request resource uri: {uri}"))?;
        let request = db
            .get_request(id)
            .await
            .map_err(|e| format!("Failed to load request {id}: {e}"))?
            .ok_or_else(|| format!("Resource not found: {uri}"))?;
        let text = serde_json::to_string_pretty(&request)
            .map_err(|e| format!("Failed to serialize request: {e}"))?;
        return Ok(vec![json!({
            "uri": uri,
            "mimeType": "application/json",
            "text": text
        })]);
    }

    Err(format!("Resource not found: {uri}"))
}

fn truthy_env_var(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn should_use_ephemeral_database() -> bool {
    truthy_env_var("FYI_MCP_EPHEMERAL")
        || truthy_env_var("FYI_MCP_INSPECTION")
        || std::env::var_os("GLAMA_VERSION").is_some()
}

fn sqlite_url_with_create_mode(database_url: String) -> String {
    if database_url == "sqlite::memory:" || !database_url.starts_with("sqlite:") {
        return database_url;
    }

    if database_url.contains("mode=") {
        return database_url;
    }

    let separator = if database_url.contains('?') { '&' } else { '?' };
    format!("{database_url}{separator}mode=rwc")
}

fn database_url_from_env() -> String {
    if should_use_ephemeral_database() {
        "sqlite::memory:".to_string()
    } else {
        sqlite_url_with_create_mode(
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://fyi_system.db".to_string()),
        )
    }
}

async fn open_database(database_url: &str) -> Result<DbPool, sqlx::Error> {
    if database_url == "sqlite::memory:" {
        DbPool::new_in_memory().await
    } else {
        DbPool::new(database_url).await
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // MCP servers speak JSON-RPC over stdout, so all diagnostics must go to
    // stderr. `tracing` is configured with an `EnvFilter` (RUST_LOG, default
    // "info") writing to stderr to keep the stdio protocol clean while giving
    // operators structured, leveled logs.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let remote_policy = RemoteMcpPolicy::from_env()?;
    tracing::info!(
        remote_enabled = remote_policy.status().remote_enabled,
        "remote MCP policy loaded; capabilities remain disabled unless explicitly configured"
    );

    tracing::info!("FYI MCP Server starting up...");

    let db_path = database_url_from_env();

    tracing::info!(database_url = %db_path, "Connecting to database");
    let db = open_database(&db_path).await?;
    db.run_migrations().await?;
    ensure_authorities_table(db.pool()).await?;

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();
    let write_gate = RemoteWriteGate::default();

    while let Some(line) = reader.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(req) => {
                if let Some(resp) = handle_jsonrpc_request_with_policy_and_gate(
                    &db,
                    req,
                    &remote_policy,
                    &write_gate,
                )
                .await
                {
                    let resp_str = serde_json::to_string(&resp)? + "\n";
                    stdout.write_all(resp_str.as_bytes()).await?;
                    stdout.flush().await?;
                }
            }
            Err(e) => {
                let resp = JsonRpcResponse::error(None, -32700, format!("Parse error: {}", e));
                let resp_str = serde_json::to_string(&resp)? + "\n";
                stdout.write_all(resp_str.as_bytes()).await?;
                stdout.flush().await?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_database_env() {
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("FYI_MCP_EPHEMERAL");
        std::env::remove_var("FYI_MCP_INSPECTION");
        std::env::remove_var("GLAMA_VERSION");
    }

    fn structured_content(result: &Value) -> &Value {
        result
            .get("structuredContent")
            .expect("tool result should include structuredContent")
    }

    #[test]
    fn test_database_url_defaults_to_create_mode() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_database_env();

        assert_eq!(
            database_url_from_env(),
            "sqlite://fyi_system.db?mode=rwc".to_string()
        );

        clear_database_env();
    }

    #[test]
    fn test_database_url_preserves_explicit_create_mode() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_database_env();
        std::env::set_var("DATABASE_URL", "sqlite:///tmp/fyi_system.db?mode=rwc");

        assert_eq!(
            database_url_from_env(),
            "sqlite:///tmp/fyi_system.db?mode=rwc".to_string()
        );

        clear_database_env();
    }

    #[test]
    fn test_database_url_uses_memory_for_glama_inspection() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_database_env();
        std::env::set_var("DATABASE_URL", "sqlite:///tmp/fyi_system.db?mode=rwc");
        std::env::set_var("GLAMA_VERSION", "1.0.0");

        assert_eq!(database_url_from_env(), "sqlite::memory:".to_string());

        clear_database_env();
    }

    #[tokio::test]
    async fn test_initialize() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: None,
        };

        let resp = handle_jsonrpc_request(&db, req).await.unwrap();
        assert_eq!(resp.id, Some(json!(1)));
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(
            result.get("protocolVersion").unwrap().as_str().unwrap(),
            "2024-11-05"
        );
        assert!(result.pointer("/capabilities/resources").is_some());
        assert!(result.pointer("/capabilities/prompts").is_some());
        assert!(
            result
                .get("instructions")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .contains("check_status"),
            "initialize should advertise server instructions for Smithery/client routing"
        );
    }

    #[tokio::test]
    async fn test_prompts_list_and_get() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        let list_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(90)),
            method: "prompts/list".to_string(),
            params: None,
        };
        let list_resp = handle_jsonrpc_request(&db, list_req).await.unwrap();
        let prompts = list_resp
            .result
            .as_ref()
            .and_then(|r| r.get("prompts"))
            .and_then(|p| p.as_array())
            .expect("prompts array");
        assert!(prompts.len() >= 3);

        let get_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(91)),
            method: "prompts/get".to_string(),
            params: Some(json!({
                "name": "draft_foi_request",
                "arguments": {
                    "authority_name": "Ministry of Justice",
                    "subject": "OIA policies"
                }
            })),
        };
        let get_resp = handle_jsonrpc_request(&db, get_req).await.unwrap();
        let text = get_resp
            .result
            .as_ref()
            .and_then(|r| r.get("messages"))
            .and_then(|m| m.as_array())
            .and_then(|a| a.first())
            .and_then(|m| m.get("content"))
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");
        assert!(text.contains("Ministry of Justice"));
        assert!(text.contains("OIA policies"));
    }

    #[tokio::test]
    async fn test_resources_list_and_read() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        let create = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(10)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "create_request",
                "arguments": {
                    "title": "Resource test request",
                    "body": "Body for MCP resource coverage."
                }
            })),
        };
        let create_resp = handle_jsonrpc_request(&db, create).await.unwrap();
        assert!(create_resp.error.is_none());

        let list_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(11)),
            method: "resources/list".to_string(),
            params: None,
        };
        let list_resp = handle_jsonrpc_request(&db, list_req).await.unwrap();
        assert!(list_resp.error.is_none());
        let resources = list_resp
            .result
            .as_ref()
            .and_then(|r| r.get("resources"))
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(resources
            .iter()
            .any(|r| r.get("uri").and_then(|u| u.as_str()) == Some("fyi://authorities")));
        assert!(resources
            .iter()
            .any(|r| r.get("uri").and_then(|u| u.as_str()) == Some("fyi://requests")));
        assert!(resources.iter().any(|r| {
            r.get("uri")
                .and_then(|u| u.as_str())
                .map(|u| u.starts_with("fyi://requests/"))
                .unwrap_or(false)
        }));

        let read_index = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(12)),
            method: "resources/read".to_string(),
            params: Some(json!({ "uri": "fyi://requests" })),
        };
        let index_resp = handle_jsonrpc_request(&db, read_index).await.unwrap();
        assert!(index_resp.error.is_none());
        let text = index_resp
            .result
            .as_ref()
            .and_then(|r| r.get("contents"))
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");
        assert!(text.contains("Resource test request"));

        let read_one = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(13)),
            method: "resources/read".to_string(),
            params: Some(json!({ "uri": "fyi://requests/1" })),
        };
        let one_resp = handle_jsonrpc_request(&db, read_one).await.unwrap();
        assert!(one_resp.error.is_none());

        let missing = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(14)),
            method: "resources/read".to_string(),
            params: Some(json!({ "uri": "fyi://unknown" })),
        };
        let missing_resp = handle_jsonrpc_request(&db, missing).await.unwrap();
        assert!(missing_resp.error.is_some());
    }

    #[tokio::test]
    async fn test_tools_list() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: None,
        };

        let resp = handle_jsonrpc_request(&db, req).await.unwrap();
        assert_eq!(resp.id, Some(json!(2)));
        let result = resp.result.unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();
        let tool = |name: &str| {
            tools
                .iter()
                .find(|t| t.get("name").and_then(|v| v.as_str()) == Some(name))
                .unwrap()
        };
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap().as_str().unwrap() == "retrieve_request"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap().as_str().unwrap() == "list_requests"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap().as_str().unwrap() == "create_request"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap().as_str().unwrap() == "update_request"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap().as_str().unwrap() == "endorsed_route_status"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap().as_str().unwrap() == "delete_request"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap().as_str().unwrap() == "list_authorities"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap().as_str().unwrap() == "import_authorities"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap().as_str().unwrap() == "compute_deadline"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap().as_str().unwrap() == "search_corpus"));
        assert_eq!(
            tool("compute_deadline")
                .get("title")
                .and_then(|v| v.as_str()),
            Some("Compute Deadline")
        );
        assert!(tool("compute_deadline")
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("statutory"));
        assert_eq!(
            tool("search_corpus").get("title").and_then(|v| v.as_str()),
            Some("Search Corpus")
        );
        assert_eq!(
            tool("list_requests").get("title").and_then(|v| v.as_str()),
            Some("List Requests")
        );
        assert_eq!(
            tool("list_requests")
                .get("annotations")
                .and_then(|v| v.get("readOnlyHint"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        assert!(tool("list_requests").get("outputSchema").is_some());
        assert_eq!(
            tool("create_request")
                .get("inputSchema")
                .and_then(|v| v.get("properties"))
                .and_then(|v| v.get("title"))
                .and_then(|v| v.get("description"))
                .and_then(|v| v.as_str()),
            Some("Short public-facing request title (required, non-empty).")
        );
        assert_eq!(
            tool("list_requests")
                .get("outputSchema")
                .and_then(|v| v.get("properties"))
                .and_then(|v| v.get("requests"))
                .and_then(|v| v.get("items"))
                .and_then(|v| v.get("properties"))
                .and_then(|v| v.get("updated_at"))
                .and_then(|v| v.get("description"))
                .and_then(|v| v.as_str()),
            Some("ISO-8601 update timestamp used for ordering and sync.")
        );
        assert_eq!(
            tool("sync_monitor").get("title").and_then(|v| v.as_str()),
            Some("Sync Monitor")
        );
        assert_eq!(
            tool("check_status")
                .get("annotations")
                .and_then(|v| v.get("readOnlyHint"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        assert!(tool("sync_status").get("outputSchema").is_some());
        // Glama TDQS: mutative tools must disclose usage + side effects in free text.
        let resolve_desc = tool("sync_resolve_conflict")
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            resolve_desc.contains("sync_conflicts")
                && resolve_desc.contains("local SQLite")
                && resolve_desc.contains("mark_clean"),
            "sync_resolve_conflict description should cover prerequisites, side effects, and mark_clean"
        );
        let update_desc = tool("update_request")
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            update_desc.contains("create_request")
                && update_desc.contains("delete_request")
                && update_desc.contains("full replacement"),
            "update_request description should contrast siblings and document replacement semantics"
        );
        assert_eq!(
            tool("update_request")
                .get("annotations")
                .and_then(|v| v.get("idempotentHint"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            tool("sync_resolve_conflict")
                .get("annotations")
                .and_then(|v| v.get("idempotentHint"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[tokio::test]
    async fn test_endorsed_route_status_is_read_only_and_fail_closed() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        let capabilities: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/endorsed-client-route/enabled.json"
        ))
        .unwrap();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(44)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "endorsed_route_status",
                "arguments": {
                    "capabilities": capabilities,
                    "client_id": "fyi-cli-prod",
                    "scopes": ["read", "bulk_export"],
                    "now_epoch": 1700000000,
                    "bulk_export": true
                }
            })),
        };
        let response = handle_jsonrpc_request(&db, request).await.unwrap();
        let payload = structured_content(response.result.as_ref().unwrap());
        assert_eq!(payload["status"], "authorized");

        let disabled = serde_json::json!({
            "protocol": "fyi-endorsed-client/v1",
            "instance_id": "nz-fyi",
            "enabled": false,
            "kill_switch": false,
            "revoked": false,
            "expires_at": 1800000000,
            "client_allowlist": ["fyi-cli-prod"],
            "scopes": ["read"],
            "quotas": {
                "max_requests": 1,
                "max_bytes": 1,
                "max_runtime_seconds": 1,
                "max_concurrency": 1,
                "max_retries": 0
            },
            "bulk_export": {
                "enabled": false,
                "scope": "bulk_export",
                "max_items": 1,
                "max_bytes": 1
            }
        });
        let denied = endorsed_route_status(&json!({
            "capabilities": disabled,
            "client_id": "fyi-cli-prod",
            "scopes": ["read"],
            "now_epoch": 1700000000
        }))
        .unwrap();
        assert_eq!(denied["status"], "denied");
    }

    #[tokio::test]
    async fn test_create_and_retrieve_request() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        // 1. Create a request using tool
        let create_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "create_request",
                "arguments": {
                    "title": "My OIA Test",
                    "body": "This is a request body.",
                    "user_name": "Alice Developer",
                    "status": "submitted",
                    "tags": ["test", "integration"]
                }
            })),
        };

        let create_resp = handle_jsonrpc_request(&db, create_req).await.unwrap();
        assert_eq!(create_resp.id, Some(json!(3)));
        assert!(create_resp.error.is_none());

        let result = create_resp.result.unwrap();
        let created_request: AlaveteliRequest =
            serde_json::from_value(structured_content(&result).get("request").unwrap().clone())
                .unwrap();
        assert_eq!(created_request.title, "My OIA Test");
        assert_eq!(created_request.id, 1);

        // 2. Retrieve the created request
        let retrieve_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(4)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "retrieve_request",
                "arguments": {
                    "id": 1
                }
            })),
        };

        let retrieve_resp = handle_jsonrpc_request(&db, retrieve_req).await.unwrap();
        assert_eq!(retrieve_resp.id, Some(json!(4)));
        let retrieve_result = retrieve_resp.result.unwrap();

        let parsed_retrieve = structured_content(&retrieve_result);
        assert_eq!(
            parsed_retrieve
                .get("request")
                .unwrap()
                .get("title")
                .unwrap()
                .as_str()
                .unwrap(),
            "My OIA Test"
        );
    }

    #[tokio::test]
    async fn test_list_requests() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        for id in 1..=2 {
            db.insert_request(&AlaveteliRequest {
                id,
                title: format!("Request {}", id),
                body: "Body".to_string(),
                user_name: None,
                status: Some("draft".to_string()),
                created_at: Some(format!("2026-06-1{}T00:00:00Z", id)),
                updated_at: None,
                url: None,
                tags: None,
            })
            .await
            .unwrap();
        }

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(6)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "list_requests",
                "arguments": {
                    "limit": 10
                }
            })),
        };

        let resp = handle_jsonrpc_request(&db, req).await.unwrap();
        assert_eq!(resp.id, Some(json!(6)));
        let result = resp.result.unwrap();
        let requests: Vec<AlaveteliRequest> =
            serde_json::from_value(structured_content(&result).get("requests").unwrap().clone())
                .unwrap();

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].title, "Request 2");
        assert_eq!(requests[1].title, "Request 1");
    }

    #[tokio::test]
    async fn test_update_request() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        db.insert_request(&AlaveteliRequest {
            id: 1,
            title: "Original".to_string(),
            body: "Old body".to_string(),
            user_name: None,
            status: Some("draft".to_string()),
            created_at: Some("2026-06-15T00:00:00Z".to_string()),
            updated_at: None,
            url: None,
            tags: None,
        })
        .await
        .unwrap();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(7)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "update_request",
                "arguments": {
                    "id": 1,
                    "title": "Updated",
                    "body": "New body",
                    "user_name": "Alice",
                    "status": "submitted",
                    "tags": ["edited"]
                }
            })),
        };

        let resp = handle_jsonrpc_request(&db, req).await.unwrap();
        assert_eq!(resp.id, Some(json!(7)));
        let result = resp.result.unwrap();
        let request: AlaveteliRequest =
            serde_json::from_value(structured_content(&result).get("request").unwrap().clone())
                .unwrap();

        assert_eq!(request.title, "Updated");
        assert_eq!(request.body, "New body");
        assert_eq!(request.user_name, Some("Alice".to_string()));
        assert_eq!(request.status, Some("submitted".to_string()));
        assert_eq!(request.created_at, Some("2026-06-15T00:00:00Z".to_string()));
    }

    #[tokio::test]
    async fn test_delete_request() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        db.insert_request(&AlaveteliRequest {
            id: 1,
            title: "Delete me".to_string(),
            body: "Body".to_string(),
            user_name: None,
            status: Some("draft".to_string()),
            created_at: Some("2026-06-15T00:00:00Z".to_string()),
            updated_at: None,
            url: None,
            tags: None,
        })
        .await
        .unwrap();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(8)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "delete_request",
                "arguments": {
                    "id": 1
                }
            })),
        };

        let resp = handle_jsonrpc_request(&db, req).await.unwrap();
        assert_eq!(resp.id, Some(json!(8)));
        let result = resp.result.unwrap();
        assert_eq!(
            structured_content(&result)
                .get("request_id")
                .unwrap()
                .as_i64()
                .unwrap(),
            1
        );
        assert!(db.get_request(1).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_import_authorities() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        let import_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(9)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "import_authorities",
                "arguments": {
                    "authorities": [
                        {
                            "slug": "ombudsman",
                            "name": "Ombudsman",
                            "url": "https://www.ombudsman.parliament.nz"
                        },
                        {
                            "slug": "dia",
                            "name": "Department of Internal Affairs",
                            "url": null
                        },
                        {
                            "slug": "dia",
                            "name": "Department of Internal Affairs NZ",
                            "url": "https://www.dia.govt.nz"
                        }
                    ]
                }
            })),
        };

        let import_resp = handle_jsonrpc_request(&db, import_req).await.unwrap();
        assert_eq!(import_resp.id, Some(json!(9)));
        let import_result = import_resp.result.unwrap();
        assert_eq!(
            structured_content(&import_result)
                .get("imported")
                .unwrap()
                .as_u64()
                .unwrap(),
            3
        );

        let list_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(10)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "list_authorities",
                "arguments": {}
            })),
        };

        let list_resp = handle_jsonrpc_request(&db, list_req).await.unwrap();
        let list_result = list_resp.result.unwrap();
        let authorities: Vec<Authority> = serde_json::from_value(
            structured_content(&list_result)
                .get("authorities")
                .unwrap()
                .clone(),
        )
        .unwrap();

        assert_eq!(authorities.len(), 2);
        assert_eq!(authorities[0].slug, "dia");
        assert_eq!(authorities[0].name, "Department of Internal Affairs NZ");
        assert_eq!(
            authorities[0].url,
            Some("https://www.dia.govt.nz".to_string())
        );
        assert_eq!(authorities[1].slug, "ombudsman");
    }

    #[tokio::test]
    async fn test_check_status() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(5)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "check_status",
                "arguments": {}
            })),
        };

        let resp = handle_jsonrpc_request(&db, req).await.unwrap();
        assert_eq!(resp.id, Some(json!(5)));
        let result = resp.result.unwrap();
        let status_info = structured_content(&result);
        assert_eq!(
            status_info.get("status").unwrap().as_str().unwrap(),
            "healthy"
        );
    }

    #[tokio::test]
    async fn test_sync_status_tool() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        db.insert_request(&AlaveteliRequest {
            id: 77,
            title: "Dirty request".to_string(),
            body: "Body".to_string(),
            user_name: None,
            status: Some("draft".to_string()),
            created_at: None,
            updated_at: None,
            url: None,
            tags: None,
        })
        .await
        .unwrap();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(11)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "sync_status",
                "arguments": {
                    "request_id": 77
                }
            })),
        };

        let resp = handle_jsonrpc_request(&db, req).await.unwrap();
        assert_eq!(resp.id, Some(json!(11)));
        let result = resp.result.unwrap();
        let status_info = structured_content(&result);

        assert_eq!(status_info.get("request_id").unwrap().as_i64().unwrap(), 77);
        assert_eq!(
            status_info.get("sync_status").unwrap().as_str().unwrap(),
            "dirty"
        );
    }

    #[tokio::test]
    async fn test_sync_conflict_tools() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        db.insert_request(&AlaveteliRequest {
            id: 88,
            title: "Conflicted MCP request".to_string(),
            body: "Body".to_string(),
            user_name: None,
            status: Some("draft".to_string()),
            created_at: None,
            updated_at: None,
            url: None,
            tags: None,
        })
        .await
        .unwrap();
        db.mark_request_conflict(88).await.unwrap();

        let list_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(12)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "sync_conflicts",
                "arguments": {}
            })),
        };
        let list_resp = handle_jsonrpc_request(&db, list_req).await.unwrap();
        let list_result = list_resp.result.unwrap();
        let conflicts: Vec<AlaveteliRequest> = serde_json::from_value(
            structured_content(&list_result)
                .get("conflicts")
                .unwrap()
                .clone(),
        )
        .unwrap();
        assert_eq!(conflicts.len(), 1);

        let resolve_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(13)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "sync_resolve_conflict",
                "arguments": {
                    "request_id": 88,
                    "mark_clean": false
                }
            })),
        };
        let resolve_resp = handle_jsonrpc_request(&db, resolve_req).await.unwrap();
        let resolve_result = resolve_resp.result.unwrap();
        let resolved = structured_content(&resolve_result);

        assert!(resolved.get("resolved").unwrap().as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_sync_monitor_tool_reports_queue_and_status() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        db.insert_request(&AlaveteliRequest {
            id: 89,
            title: "Queued monitor request".to_string(),
            body: "Body".to_string(),
            user_name: None,
            status: Some("draft".to_string()),
            created_at: None,
            updated_at: None,
            url: None,
            tags: None,
        })
        .await
        .unwrap();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(14)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "sync_monitor",
                "arguments": {}
            })),
        };

        let resp = handle_jsonrpc_request(&db, req).await.unwrap();
        let result = resp.result.unwrap();
        let monitor = structured_content(&result);

        assert_eq!(
            monitor
                .get("sync")
                .unwrap()
                .get("dirty")
                .unwrap()
                .as_i64()
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_compute_deadline_tool() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(20)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "compute_deadline",
                "arguments": {
                    "start_date": "2026-07-03",
                    "days": 2,
                    "calendar": false
                }
            })),
        };

        let resp = handle_jsonrpc_request(&db, req).await.unwrap();
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        let deadline = structured_content(&result);
        assert_eq!(
            deadline.get("due_date").and_then(|v| v.as_str()),
            Some("2026-07-07")
        );
        assert_eq!(
            deadline
                .get("statutory_deadline_days")
                .and_then(|v| v.as_u64()),
            Some(2)
        );
    }

    #[tokio::test]
    async fn test_search_corpus_tool() {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(21)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "search_corpus",
                "arguments": {
                    "query": "procurement contracts",
                    "limit": 5
                }
            })),
        };

        let resp = handle_jsonrpc_request(&db, req).await.unwrap();
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        let payload = structured_content(&result);
        assert_eq!(
            payload.get("document_count").and_then(|v| v.as_u64()),
            Some(3)
        );
        let hits = payload.get("hits").and_then(|v| v.as_array()).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].get("id").and_then(|v| v.as_str()), Some("1"));
    }
}
