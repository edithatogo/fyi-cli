use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use fyi_core::db::DbPool;
use fyi_core::api::AlaveteliRequest;

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
        )"
    )
    .execute(pool)
    .await?;
    Ok(())
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
                    "version": "0.1.0"
                }
            });
            Some(JsonRpcResponse::success(req.id, res))
        }
        "notifications/initialized" => {
            // Notifications do not return responses
            None
        }
        "tools/list" => {
            let tools = json!({
                "tools": [
                    {
                        "name": "list_requests",
                        "description": "List tracked Alaveteli requests from the database",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "limit": {
                                    "type": "integer",
                                    "description": "Maximum number of requests to return"
                                }
                            }
                        }
                    },
                    {
                        "name": "retrieve_request",
                        "description": "Retrieve an Alaveteli request (and its correspondence) by ID from the database",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "id": {
                                    "type": "integer",
                                    "description": "The unique request ID"
                                }
                            },
                            "required": ["id"]
                        }
                    },
                    {
                        "name": "create_request",
                        "description": "Create a new request in the database",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string", "description": "The request title" },
                                "body": { "type": "string", "description": "The request body" },
                                "user_name": { "type": "string", "description": "Name of the user" },
                                "status": { "type": "string", "description": "Status of the request" },
                                "url": { "type": "string", "description": "The URL on Alaveteli/FYI" },
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Optional list of tags"
                                }
                            },
                            "required": ["title", "body"]
                        }
                    },
                    {
                        "name": "update_request",
                        "description": "Update an existing request in the database",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "integer", "description": "The request ID" },
                                "title": { "type": "string", "description": "The request title" },
                                "body": { "type": "string", "description": "The request body" },
                                "user_name": { "type": "string", "description": "Name of the user" },
                                "status": { "type": "string", "description": "Status of the request" },
                                "url": { "type": "string", "description": "The URL on Alaveteli/FYI" },
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Optional list of tags"
                                }
                            },
                            "required": ["id", "title", "body"]
                        }
                    },
                    {
                        "name": "list_authorities",
                        "description": "List authorities stored in the database",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "check_status",
                        "description": "Check database status and other components",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    }
                ]
            });
            Some(JsonRpcResponse::success(req.id, tools))
        }
        "tools/call" => {
            let params = match req.params.as_ref() {
                Some(p) => p,
                None => return Some(JsonRpcResponse::error(req.id, -32602, "Missing parameters".to_string())),
            };

            let name = match params.get("name").and_then(|n| n.as_str()) {
                Some(n) => n,
                None => return Some(JsonRpcResponse::error(req.id, -32602, "Missing tool name".to_string())),
            };

            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

            match name {
                "list_requests" => {
                    let limit = arguments.get("limit").and_then(|i| i.as_i64()).unwrap_or(100);

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
                            })
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
                            })
                        )),
                    }
                }
                "retrieve_request" => {
                    let id = match arguments.get("id").and_then(|i| i.as_i64()) {
                        Some(i) => i,
                        None => return Some(JsonRpcResponse::error(req.id, -32602, "Invalid or missing 'id' argument".to_string())),
                    };

                    match db.get_request(id).await {
                        Ok(Some(request)) => {
                            let correspondence = db.get_correspondence_for_request(id).await.unwrap_or_default();
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
                                })
                            ))
                        }
                        Ok(None) => {
                            Some(JsonRpcResponse::success(
                                req.id,
                                json!({
                                    "isError": true,
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": format!("Request with ID {} not found", id)
                                        }
                                    ]
                                })
                            ))
                        }
                        Err(e) => {
                            Some(JsonRpcResponse::success(
                                req.id,
                                json!({
                                    "isError": true,
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": format!("Database error: {}", e)
                                        }
                                    ]
                                })
                            ))
                        }
                    }
                }
                "create_request" => {
                    let title = match arguments.get("title").and_then(|t| t.as_str()) {
                        Some(t) => t.to_string(),
                        None => return Some(JsonRpcResponse::error(req.id, -32602, "Missing 'title'".to_string())),
                    };
                    let body = match arguments.get("body").and_then(|b| b.as_str()) {
                        Some(b) => b.to_string(),
                        None => return Some(JsonRpcResponse::error(req.id, -32602, "Missing 'body'".to_string())),
                    };
                    let user_name = arguments.get("user_name").and_then(|u| u.as_str()).map(String::from);
                    let status = arguments.get("status").and_then(|s| s.as_str()).map(String::from);
                    let url = arguments.get("url").and_then(|u| u.as_str()).map(String::from);
                    let tags = arguments.get("tags").and_then(|t| {
                        if let Some(arr) = t.as_array() {
                            let parsed: Vec<String> = arr.iter().filter_map(|val| val.as_str().map(String::from)).collect();
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
                            })
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
                            })
                        )),
                    }
                }
                "update_request" => {
                    let id = match arguments.get("id").and_then(|i| i.as_i64()) {
                        Some(i) => i,
                        None => return Some(JsonRpcResponse::error(req.id, -32602, "Invalid or missing 'id' argument".to_string())),
                    };
                    let title = match arguments.get("title").and_then(|t| t.as_str()) {
                        Some(t) => t.to_string(),
                        None => return Some(JsonRpcResponse::error(req.id, -32602, "Missing 'title'".to_string())),
                    };
                    let body = match arguments.get("body").and_then(|b| b.as_str()) {
                        Some(b) => b.to_string(),
                        None => return Some(JsonRpcResponse::error(req.id, -32602, "Missing 'body'".to_string())),
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
                                })
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
                                })
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
                        user_name: arguments.get("user_name").and_then(|u| u.as_str()).map(String::from),
                        status: arguments.get("status").and_then(|s| s.as_str()).map(String::from),
                        created_at: existing.created_at,
                        updated_at: Some(chrono::Utc::now().to_rfc3339()),
                        url: arguments.get("url").and_then(|u| u.as_str()).map(String::from),
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
                            })
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
                            })
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
                            })
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
                            })
                        ));
                    }

                    match sqlx::query_as::<_, (String, String, Option<String>)>("SELECT slug, name, url FROM authorities ORDER BY name ASC")
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
                                })
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
                            })
                        )),
                    }
                }
                "check_status" => {
                    let db_healthy = sqlx::query("SELECT 1")
                        .execute(db.pool())
                        .await
                        .is_ok();

                    let total_requests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM requests")
                        .fetch_one(db.pool())
                        .await
                        .unwrap_or(0);

                    let total_correspondence: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM correspondence")
                        .fetch_one(db.pool())
                        .await
                        .unwrap_or(0);

                    let status_info = json!({
                        "status": if db_healthy { "healthy" } else { "unhealthy" },
                        "database": if db_healthy { "connected" } else { "disconnected" },
                        "metrics": {
                            "total_requests": total_requests,
                            "total_correspondence": total_correspondence
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
                        })
                    ))
                }
                _ => Some(JsonRpcResponse::error(req.id, -32601, format!("Tool '{}' not found", name))),
            }
        }
        _ => Some(JsonRpcResponse::error(req.id, -32601, format!("Method '{}' not found", req.method))),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("FYI MCP Server starting up...");

    let db_path = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://fyi_system.db".to_string());

    eprintln!("Connecting to database at: {}", db_path);
    let db = DbPool::new(&db_path).await?;
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
        assert_eq!(result.get("protocolVersion").unwrap().as_str().unwrap(), "2024-11-05");
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
        assert!(tools.iter().any(|t| t.get("name").unwrap().as_str().unwrap() == "retrieve_request"));
        assert!(tools.iter().any(|t| t.get("name").unwrap().as_str().unwrap() == "list_requests"));
        assert!(tools.iter().any(|t| t.get("name").unwrap().as_str().unwrap() == "create_request"));
        assert!(tools.iter().any(|t| t.get("name").unwrap().as_str().unwrap() == "update_request"));
        assert!(tools.iter().any(|t| t.get("name").unwrap().as_str().unwrap() == "list_authorities"));
        assert!(tools.iter().any(|t| t.get("name").unwrap().as_str().unwrap() == "check_status"));
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
        assert_eq!(parsed_retrieve.get("request").unwrap().get("title").unwrap().as_str().unwrap(), "My OIA Test");
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
        assert_eq!(status_info.get("status").unwrap().as_str().unwrap(), "healthy");
    }
}
