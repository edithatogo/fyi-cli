//! Thin local-only CLI handlers; no source clients or remote sync operations.
use crate::{Commands, OutputFormat};
use fyi_core::db::local::requests::NewLocalRequest;
use fyi_core::db::local::{parse_authorities_csv, LocalError, MAX_LOCAL_INPUT_BYTES};
use fyi_core::db::DbPool;
use std::io::Read;
use std::path::Path;
use tokio::runtime::Runtime;

pub fn run(command: &Commands, format: OutputFormat) {
    if let Err(error) = execute(command, format) {
        if format == OutputFormat::Json {
            eprintln!(
                "{}",
                serde_json::json!({"error": {"kind": "local", "message": error.to_string()}})
            );
        } else {
            eprintln!("Local operation failed: {error}");
        }
        std::process::exit(1);
    }
}

fn execute(command: &Commands, format: OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::new()?;
    match command {
        Commands::ImportAuthorities { csv_path, db } => {
            let file = std::fs::File::open(csv_path)
                .map_err(|_| LocalError::InvalidInput("unable to read authority CSV"))?;
            let mut bytes = Vec::new();
            file.take((MAX_LOCAL_INPUT_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|_| LocalError::InvalidInput("unable to read authority CSV"))?;
            let authorities = parse_authorities_csv(&bytes)?;
            let count = runtime.block_on(async {
                let db = DbPool::open_local_existing(Path::new(db), false).await?;
                let result = db.import_local_authorities(&authorities).await;
                db.pool().close().await;
                result
            })?;
            if format == OutputFormat::Json {
                println!("{}", serde_json::json!({"imported_authorities": count}));
            } else {
                println!("Imported {count} authorities");
            }
        }
        Commands::ListAuthorities { db } => {
            let authorities = runtime.block_on(async {
                let db = DbPool::open_local_existing(Path::new(db), true).await?;
                let result = db.list_local_authorities().await;
                db.pool().close().await;
                result
            })?;
            if format == OutputFormat::Json {
                println!("{}", serde_json::to_string_pretty(&authorities)?);
            } else {
                for authority in authorities {
                    println!(
                        "{}\t{}\t{}",
                        authority.slug,
                        authority.name,
                        authority.url.as_deref().unwrap_or_default()
                    );
                }
            }
        }
        Commands::RegisterRequest {
            authority_slug,
            title,
            body,
            tags,
            status,
            fyi_request_id,
            db,
        } => {
            let request = NewLocalRequest {
                authority_slug: authority_slug.clone(),
                title: title.clone(),
                body: body.clone(),
                tags: tags.clone().unwrap_or_default(),
                status: status.clone(),
                fyi_request_id: *fyi_request_id,
            };
            let id = runtime.block_on(async {
                let db = DbPool::open_local_existing(Path::new(db), false).await?;
                let result = db.register_local_request(&request).await;
                db.pool().close().await;
                result
            })?;
            if format == OutputFormat::Json {
                println!("{}", serde_json::json!({"id": id}));
            } else {
                println!("Registered request");
            }
        }
        Commands::ListRequests { db } => {
            let requests = runtime.block_on(async {
                let db = DbPool::open_local_existing(Path::new(db), true).await?;
                let result = db.list_local_requests().await;
                db.pool().close().await;
                result
            })?;
            if format == OutputFormat::Json {
                println!("{}", serde_json::to_string_pretty(&requests)?);
            } else {
                for request in requests {
                    println!(
                        "{}\t{}\t{}\t{}\t{}",
                        request.id,
                        request.authority_slug,
                        request.title,
                        request.status.as_deref().unwrap_or("None"),
                        request.fyi_request_id.map(|id| id.to_string()).unwrap_or_default()
                    );
                }
            }
        }
        Commands::SetStatus {
            request_id,
            status,
            db,
        } => {
            let changed = runtime.block_on(async {
                let db = DbPool::open_local_existing(Path::new(db), false).await?;
                let result = db.set_local_request_status(*request_id, status).await;
                db.pool().close().await;
                result
            })?;
            if format == OutputFormat::Json {
                println!("{}", serde_json::json!({"id": request_id, "status": status, "changed": changed}));
            } else {
                println!("Updated request {request_id} to {status}");
            }
        }
        Commands::RequestTimeline { request_id, db } => {
            let timeline = runtime.block_on(async {
                let db = DbPool::open_local_existing(Path::new(db), true).await?;
                let result = db.local_request_timeline(*request_id).await;
                db.pool().close().await;
                result
            })?;
            println!("{}", serde_json::to_string_pretty(&timeline)?);
        }
        Commands::RequestDetail { request_id, db } => {
            let detail = runtime.block_on(async {
                let db = DbPool::open_local_existing(Path::new(db), true).await?;
                let result = db.local_request_detail(*request_id).await;
                db.pool().close().await;
                result
            })?;
            println!("{}", serde_json::to_string_pretty(&detail)?);
        }
        _ => return Err(LocalError::InvalidInput("unsupported local command").into()),
    }
    Ok(())
}
