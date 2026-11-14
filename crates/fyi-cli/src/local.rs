//! Thin local-only CLI handlers; no source clients or remote sync operations.
use crate::{Commands, OutputFormat};
use fyi_core::db::local::requests::NewLocalRequest;
use fyi_core::db::local::transfer::{encode_local_export, parse_local_import};
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
        Commands::InitDb { db, migrate_legacy } => {
            if !migrate_legacy {
                create_private_database(Path::new(db))?;
            }
            runtime.block_on(async {
                let pool = DbPool::open_local_for_initialization(Path::new(db)).await?;
                let result = if *migrate_legacy {
                    pool.migrate_legacy_local_schema().await
                } else {
                    pool.initialize_rust_schema().await.map_err(|error| {
                        error.downcast::<LocalError>().unwrap_or(LocalError::Schema)
                    })
                };
                pool.pool().close().await;
                result
            })?;
            if format == OutputFormat::Json {
                println!(
                    "{}",
                    serde_json::json!({"database": db, "migrated_legacy": migrate_legacy})
                );
            } else {
                println!("Initialized SQLite database at {db}");
            }
        }
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
                        request
                            .fyi_request_id
                            .map(|id| id.to_string())
                            .unwrap_or_default()
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
                println!(
                    "{}",
                    serde_json::json!({"id": request_id, "status": status, "changed": changed})
                );
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
        Commands::ExportRequests { output, db } => {
            let document = runtime.block_on(async {
                let pool = DbPool::open_local_existing(Path::new(db), true).await?;
                let result = pool.export_local_requests().await;
                pool.pool().close().await;
                result
            })?;
            let bytes = encode_local_export(&document)?;
            publish_private_export(Path::new(output), &bytes)?;
            if format == OutputFormat::Json {
                println!(
                    "{}",
                    serde_json::json!({
                        "output": output,
                        "exported_requests": document.requests.len(),
                        "schema_version": document.schema_version
                    })
                );
            } else {
                println!("{output}");
            }
        }
        Commands::ImportRequests { input, replace, db } => {
            let file = std::fs::File::open(input)
                .map_err(|_| LocalError::InvalidInput("unable to read local request import"))?;
            let mut bytes = Vec::new();
            file.take((MAX_LOCAL_INPUT_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|_| LocalError::InvalidInput("unable to read local request import"))?;
            let input = parse_local_import(&bytes)?;
            let count = runtime.block_on(async {
                let pool = DbPool::open_local_existing(Path::new(db), false).await?;
                let result = pool.import_local_requests(&input, *replace).await;
                pool.pool().close().await;
                result
            })?;
            if format == OutputFormat::Json {
                println!("{}", serde_json::json!({"imported_requests": count}));
            } else {
                println!("{count}");
            }
        }
        _ => return Err(LocalError::InvalidInput("unsupported local command").into()),
    }
    Ok(())
}

fn create_private_database(path: &Path) -> Result<(), LocalError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|_| LocalError::Open)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(_) => Err(LocalError::Open),
    }
}

fn publish_private_export(path: &Path, bytes: &[u8]) -> Result<(), LocalError> {
    let invalid = || {
        LocalError::InvalidInput(
            "unable to publish private request export without overwriting existing data",
        )
    };
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() != bytes.len() as u64
            {
                return Err(invalid());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o077 != 0 {
                    return Err(invalid());
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(invalid()),
    }
    fyi_core::acquisition_receipts::write_evidence_bytes_atomic(path, bytes).map_err(|_| invalid())
}
