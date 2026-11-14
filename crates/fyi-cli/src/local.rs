//! Thin local-only CLI handlers; no source clients or remote sync operations.
use crate::{Commands, OutputFormat};
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
        _ => return Err(LocalError::InvalidInput("unsupported local command").into()),
    }
    Ok(())
}
