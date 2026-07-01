use fyi_core::api::AlaveteliRequest;
use fyi_core::db::DbPool;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

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
                "description": "Stable local request identifier."
            },
            "title": {
                "type": "string",
                "description": "Public title of the official information request."
            },
            "body": {
                "type": "string",
                "description": "Request body or draft correspondence text."
            },
            "user_name": {
                "type": ["string", "null"],
                "description": "Requester display name when known."
            },
            "status": {
                "type": ["string", "null"],
                "description": "Current Alaveteli/FYI request lifecycle status."
            },
            "created_at": {
                "type": ["string", "null"],
                "description": "ISO-8601 creation timestamp when recorded."
            },
            "updated_at": {
                "type": ["string", "null"],
                "description": "ISO-8601 update timestamp used for ordering and sync."
            },
            "url": {
                "type": ["string", "null"],
                "description": "Canonical public request URL when available."
            },
            "tags": {
                "type": ["array", "null"],
                "description": "Optional local tags attached to the request.",
                "items": { "type": "string" }
            }
        },
        "required": ["id", "title", "body"],
        "additionalProperties": true
    })
}

fn correspondence_schema() -> Value {
    json!({
        "type": "object",
        "description": "A correspondence item associated with an FYI request.",
        "properties": {
            "id": {
                "type": "integer",
                "description": "Stable local correspondence identifier."
            },
            "request_id": {
                "type": "integer",
                "description": "Request identifier this correspondence belongs to."
            },
            "sender": {
                "type": ["string", "null"],
                "description": "Sender name or email when captured."
            },
            "body": {
                "type": ["string", "null"],
                "description": "Message body or extracted correspondence text."
            },
            "sent_at": {
                "type": ["string", "null"],
                "description": "ISO-8601 sent timestamp when captured."
            }
        },
        "additionalProperties": true
    })
}

fn authority_schema() -> Value {
    json!({
        "type": "object",
        "description": "Public authority record used to route or classify requests.",
        "properties": {
            "slug": {
                "type": "string",
                "description": "Stable authority slug used as an import key."
            },
            "name": {
                "type": "string",
                "description": "Human-readable public authority name."
            },
            "url": {
                "type": ["string", "null"],
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
                    "List locally tracked FYI/Alaveteli official information requests, ordered newest first, for dashboards, audits, and follow-up triage."
                );
                tool["inputSchema"]["properties"]["limit"]["description"] = json!(
                    "Maximum number of request records to return. Defaults to 100 when omitted."
                );
                tool["outputSchema"]["properties"]["requests"]["items"] =
                    request_schema("One tracked FYI/Alaveteli request.");
            }
            "retrieve_request" => {
                tool["description"] = json!(
                    "Retrieve one locally tracked FYI/Alaveteli request and its stored correspondence by request ID."
                );
                tool["inputSchema"]["properties"]["id"]["description"] = json!(
                    "Stable local request ID to load, matching the ID returned by list_requests."
                );
                tool["outputSchema"]["properties"]["request"] =
                    request_schema("The requested FYI/Alaveteli request record.");
                tool["outputSchema"]["properties"]["correspondence"]["description"] =
                    json!("Stored correspondence items linked to the request, in database order.");
                tool["outputSchema"]["properties"]["correspondence"]["items"] =
                    correspondence_schema();
            }
            "create_request" => {
                tool["description"] = json!(
                    "Create a local draft or tracked FYI/Alaveteli official information request record without submitting it to a remote authority."
                );
                tool["inputSchema"]["properties"]["title"]["description"] =
                    json!("Short public-facing request title.");
                tool["inputSchema"]["properties"]["body"]["description"] =
                    json!("Full request body or draft text to store locally.");
                tool["inputSchema"]["properties"]["user_name"]["description"] =
                    json!("Optional requester display name for local tracking.");
                tool["inputSchema"]["properties"]["status"]["description"] = json!(
                    "Optional initial local status such as draft, waiting_response, or successful."
                );
                tool["inputSchema"]["properties"]["url"]["description"] =
                    json!("Optional FYI/Alaveteli URL if the request already exists online.");
                tool["inputSchema"]["properties"]["tags"]["description"] =
                    json!("Optional local classification tags for filtering or reporting.");
                tool["outputSchema"]["properties"]["request"] =
                    request_schema("The newly created local request record.");
            }
            "update_request" => {
                tool["description"] = json!(
                    "Replace editable fields on an existing local FYI/Alaveteli request record and mark changed fields for offline sync."
                );
                tool["inputSchema"]["properties"]["id"]["description"] =
                    json!("Stable local request ID to update.");
                tool["inputSchema"]["properties"]["title"]["description"] =
                    json!("Replacement request title.");
                tool["inputSchema"]["properties"]["body"]["description"] =
                    json!("Replacement request body or draft text.");
                tool["inputSchema"]["properties"]["user_name"]["description"] =
                    json!("Optional replacement requester display name.");
                tool["inputSchema"]["properties"]["status"]["description"] =
                    json!("Optional replacement local lifecycle status.");
                tool["inputSchema"]["properties"]["url"]["description"] =
                    json!("Optional replacement FYI/Alaveteli URL.");
                tool["inputSchema"]["properties"]["tags"]["description"] =
                    json!("Optional replacement tag list.");
                tool["outputSchema"]["properties"]["request"] =
                    request_schema("The updated local request record.");
            }
            "delete_request" => {
                tool["description"] = json!(
                    "Delete a local request record and its stored correspondence from the FYI database."
                );
                tool["inputSchema"]["properties"]["id"]["description"] =
                    json!("Stable local request ID to delete.");
                tool["outputSchema"]["properties"]["deleted"]["description"] =
                    json!("True when the local delete operation completed.");
                tool["outputSchema"]["properties"]["request_id"]["description"] =
                    json!("Request ID that was targeted for deletion.");
            }
            "list_authorities" => {
                tool["description"] = json!(
                    "List imported public authority records that can be used to route, classify, or validate FYI requests."
                );
                tool["outputSchema"]["properties"]["authorities"]["description"] =
                    json!("Imported public authority records.");
                tool["outputSchema"]["properties"]["authorities"]["items"] = authority_schema();
            }
            "import_authorities" => {
                tool["description"] = json!(
                    "Import or update local public authority reference records by slug for request routing and discovery."
                );
                tool["inputSchema"]["properties"]["authorities"]["description"] =
                    json!("Authority records to upsert into the local reference table.");
                tool["inputSchema"]["properties"]["authorities"]["items"] = authority_schema();
                tool["outputSchema"]["properties"]["imported"]["description"] =
                    json!("Number of authority records accepted for import or update.");
            }
            "sync_monitor" => {
                tool["description"] = json!(
                    "Summarize offline synchronization health, including clean/dirty/conflicted request counts and outgoing queue depth."
                );
                tool["outputSchema"]["properties"]["sync"]["description"] =
                    json!("Aggregate request sync counts and latest sync timestamp.");
                tool["outputSchema"]["properties"]["queue"]["description"] = json!(
                    "Outgoing offline queue counts by pending, submitted, and failed status."
                );
                tool["outputSchema"]["properties"]["offline_degradation"]["description"] =
                    json!("Operational indicators showing queued local changes and dirty records.");
            }
            "sync_conflicts" => {
                tool["description"] = json!(
                    "List locally tracked requests whose offline sync metadata is currently marked as conflicted."
                );
                tool["inputSchema"]["properties"]["limit"]["description"] = json!(
                    "Maximum number of conflicted request records to return. Defaults to 100 when omitted."
                );
                tool["outputSchema"]["properties"]["conflicts"]["description"] =
                    json!("Requests with sync_status set to conflict.");
                tool["outputSchema"]["properties"]["conflicts"]["items"] =
                    request_schema("A request currently marked as a sync conflict.");
            }
            "sync_resolve_conflict" => {
                tool["description"] = json!(
                    "Resolve a local offline-sync conflict by marking the request clean after reconciliation or dirty for later push."
                );
                tool["inputSchema"]["properties"]["request_id"]["description"] =
                    json!("Stable local request ID whose conflict metadata should be updated.");
                tool["inputSchema"]["properties"]["mark_clean"]["description"] = json!(
                    "Set true after manual reconciliation; set false to keep the request dirty for a later push."
                );
                tool["outputSchema"]["properties"]["request_id"]["description"] =
                    json!("Request ID whose conflict state was updated.");
                tool["outputSchema"]["properties"]["resolved"]["description"] =
                    json!("True when the conflict metadata was changed.");
                tool["outputSchema"]["properties"]["sync_status"]["description"] =
                    json!("Resulting sync status, usually clean or dirty.");
            }
            "sync_status" => {
                tool["description"] = json!(
                    "Read global offline-sync status or detailed sync metadata for one locally tracked FYI request."
                );
                tool["inputSchema"]["properties"]["request_id"]["description"] =
                    json!("Optional local request ID. Omit it to return aggregate sync counts.");
                tool["outputSchema"]["properties"]["request_id"]["description"] = json!(
                    "Request ID when a per-request lookup was requested; absent from aggregate responses."
                );
                tool["outputSchema"]["properties"]["sync_status"]["description"] = json!(
                    "Per-request sync status, or null when that request has no sync metadata."
                );
                tool["outputSchema"]["properties"]["total"] = json!({
                    "type": "integer",
                    "description": "Total number of requests represented in aggregate sync status."
                });
                tool["outputSchema"]["properties"]["clean"] = json!({
                    "type": "integer",
                    "description": "Number of requests with clean sync metadata."
                });
                tool["outputSchema"]["properties"]["dirty"] = json!({
                    "type": "integer",
                    "description": "Number of requests with unsynced local changes."
                });
                tool["outputSchema"]["properties"]["pending"] = json!({
                    "type": "integer",
                    "description": "Number of requests queued or pending sync."
                });
                tool["outputSchema"]["properties"]["conflict"] = json!({
                    "type": "integer",
                    "description": "Number of requests currently in conflict."
                });
            }
            "check_status" => {
                tool["description"] = json!(
                    "Check FYI MCP database readiness and return record-count metrics for requests, correspondence, and authorities."
                );
                tool["outputSchema"]["properties"]["status"]["description"] = json!(
                    "Overall service health, reported as healthy when database queries succeed."
                );
                tool["outputSchema"]["properties"]["database"]["description"] =
                    json!("Database connection state used by the MCP server.");
                tool["outputSchema"]["properties"]["metrics"]["description"] =
                    json!("Record-count metrics for core FYI tables.");
            }
            _ => {}
        }
    }
}

/// Handles a single incoming JSON-RPC request and produces the response.
pub async fn handle_jsonrpc_request(db: &DbPool, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
    match req.method.as_str() {
        "initialize" => {
            let res = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "fyi-mcp",
                    "version": "0.1.2"
                }
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
                    }
                ]
            });
            enrich_tool_definitions(&mut tools);
            Some(JsonRpcResponse::success(req.id, tools))
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
                        Ok(requests) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&requests).unwrap()
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
                                        "text": format!("Failed to fetch requests: {}", e)
                                    }
                                ]
                            }),
                        )),
                    }
                }
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
                                Some(JsonRpcResponse::success(
                                    req.id,
                                    json!({
                                        "content": [
                                            {
                                                "type": "text",
                                                "text": serde_json::to_string_pretty(&payload).unwrap()
                                            }
                                        ]
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
                                            "text": format!("Failed to fetch sync status: {}", e)
                                        }
                                    ]
                                }),
                            )),
                        }
                    } else {
                        match db.get_global_sync_status().await {
                            Ok(status) => Some(JsonRpcResponse::success(
                                req.id,
                                json!({
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": serde_json::to_string_pretty(&json!({
                                                "total": status.total,
                                                "clean": status.clean,
                                                "dirty": status.dirty,
                                                "pending": status.pending,
                                                "conflict": status.conflict
                                            })).unwrap()
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
                        (Ok(status), Ok(queue), Ok(latest_sync)) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&json!({
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
                                        })).unwrap()
                                    }
                                ]
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
                        Ok(conflicts) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&conflicts).unwrap()
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
                        Ok(resolved) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&json!({
                                            "request_id": request_id,
                                            "resolved": resolved,
                                            "sync_status": if mark_clean { "clean" } else { "dirty" }
                                        })).unwrap()
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
                            Some(JsonRpcResponse::success(
                                req.id,
                                json!({
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": serde_json::to_string_pretty(&result_val).unwrap()
                                        }
                                    ]
                                }),
                            ))
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
                        Ok(_) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&new_req).unwrap()
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
                        Ok(true) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&updated).unwrap()
                                    }
                                ]
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
                        Ok(true) => Some(JsonRpcResponse::success(
                            req.id,
                            json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&json!({
                                            "deleted": true,
                                            "id": id
                                        })).unwrap()
                                    }
                                ]
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
                            Some(JsonRpcResponse::success(
                                req.id,
                                json!({
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": serde_json::to_string_pretty(&authorities).unwrap()
                                        }
                                    ]
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

                    Some(JsonRpcResponse::success(
                        req.id,
                        json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&ImportAuthoritiesResult { imported }).unwrap()
                                }
                            ]
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

                    Some(JsonRpcResponse::success(
                        req.id,
                        json!({
                            "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&status_info).unwrap()
                                    }
                            ]
                        }),
                    ))
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
    eprintln!("FYI MCP Server starting up...");

    let db_path = database_url_from_env();

    eprintln!("Connecting to database at: {}", db_path);
    let db = open_database(&db_path).await?;
    db.run_migrations().await?;
    ensure_authorities_table(db.pool()).await?;

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = reader.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(req) => {
                if let Some(resp) = handle_jsonrpc_request(&db, req).await {
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
            .any(|t| t.get("name").unwrap().as_str().unwrap() == "delete_request"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap().as_str().unwrap() == "list_authorities"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap().as_str().unwrap() == "import_authorities"));
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
            Some("Short public-facing request title.")
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
        let content = result.get("content").unwrap().as_array().unwrap();
        let text = content[0].get("text").unwrap().as_str().unwrap();
        let created_request: AlaveteliRequest = serde_json::from_str(text).unwrap();
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
        let retrieve_content = retrieve_result.get("content").unwrap().as_array().unwrap();
        let retrieve_text = retrieve_content[0].get("text").unwrap().as_str().unwrap();

        let parsed_retrieve: Value = serde_json::from_str(retrieve_text).unwrap();
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
        let content = result.get("content").unwrap().as_array().unwrap();
        let text = content[0].get("text").unwrap().as_str().unwrap();
        let requests: Vec<AlaveteliRequest> = serde_json::from_str(text).unwrap();

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
        let content = result.get("content").unwrap().as_array().unwrap();
        let text = content[0].get("text").unwrap().as_str().unwrap();
        let request: AlaveteliRequest = serde_json::from_str(text).unwrap();

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
        let import_content = import_result.get("content").unwrap().as_array().unwrap();
        let import_text = import_content[0].get("text").unwrap().as_str().unwrap();
        let imported: ImportAuthoritiesResult = serde_json::from_str(import_text).unwrap();
        assert_eq!(imported.imported, 3);

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
        let list_content = list_result.get("content").unwrap().as_array().unwrap();
        let list_text = list_content[0].get("text").unwrap().as_str().unwrap();
        let authorities: Vec<Authority> = serde_json::from_str(list_text).unwrap();

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
        let content = result.get("content").unwrap().as_array().unwrap();
        let text = content[0].get("text").unwrap().as_str().unwrap();
        let status_info: Value = serde_json::from_str(text).unwrap();
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
        let content = result.get("content").unwrap().as_array().unwrap();
        let text = content[0].get("text").unwrap().as_str().unwrap();
        let status_info: Value = serde_json::from_str(text).unwrap();

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
        let list_content = list_result.get("content").unwrap().as_array().unwrap();
        let list_text = list_content[0].get("text").unwrap().as_str().unwrap();
        let conflicts: Vec<AlaveteliRequest> = serde_json::from_str(list_text).unwrap();
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
        let resolve_content = resolve_result.get("content").unwrap().as_array().unwrap();
        let resolve_text = resolve_content[0].get("text").unwrap().as_str().unwrap();
        let resolved: Value = serde_json::from_str(resolve_text).unwrap();

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
        let content = result.get("content").unwrap().as_array().unwrap();
        let text = content[0].get("text").unwrap().as_str().unwrap();
        let monitor: Value = serde_json::from_str(text).unwrap();

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
}
