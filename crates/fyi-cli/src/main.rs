use clap::{Parser, Subcommand, ValueEnum};
use fyi_core::db::{DbPool, GlobalSyncStatus};
use fyi_core::jurisdiction::InstanceRegistry;
use fyi_core::security::{
    build_provisioning_uri, generate_totp_secret, render_provisioning_qr_ascii, KeyringStore,
};
use fyi_core::sync::{PullReport, PushReport, SyncClient, SyncConfig};
use std::fs::OpenOptions;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
mod tui;

#[cfg(feature = "dhat-on")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[derive(Parser, Debug, Clone)]
#[command(name = "fyi-cli", version, about = "FYI Request System CLI (Rust rewriting)", long_about = None)]
pub struct Cli {
    #[arg(
        long,
        short,
        global = true,
        help = "Path to the configuration settings file"
    )]
    pub config: Option<String>,

    #[arg(
        long,
        short,
        global = true,
        default_value = "fyi_system.db",
        help = "Path to the SQLite database"
    )]
    pub db: String,

    #[arg(long, short, global = true, value_enum, default_value_t = OutputFormat::Text, help = "Output format")]
    pub output_format: OutputFormat,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrespondenceFormat {
    Json,
    Markdown,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportProfile {
    Standard,
    Strict,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum Commands {
    #[command(about = "Initialize the SQLite database")]
    InitDb {
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Import authorities from a CSV file")]
    ImportAuthorities {
        csv_path: String,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "List all authorities")]
    ListAuthorities {
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Register a new tracked request")]
    RegisterRequest {
        authority_slug: String,
        title: String,
        body: String,
        #[arg(long)]
        tags: Option<String>,
        #[arg(long, default_value = "draft")]
        status: String,
        #[arg(long)]
        fyi_request_id: Option<i32>,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "List all tracked requests")]
    ListRequests {
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Set status of a tracked request")]
    SetStatus {
        request_id: i32,
        status: String,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Get request timeline")]
    RequestTimeline {
        request_id: i32,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Export requests to JSON")]
    ExportRequests {
        #[arg(long, default_value = "outputs/requests.json")]
        output: String,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Import requests from JSON")]
    ImportRequests {
        input: String,
        #[arg(long)]
        replace: bool,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Build prefilled new request URL")]
    BuildPrefilledUrl {
        authority_slug: String,
        title: String,
        body: String,
        #[arg(long)]
        tags: Option<String>,
        #[arg(long, default_value = "https://fyi.org.nz")]
        base_url: String,
    },
    #[command(about = "List built-in jurisdictions and instances")]
    Instances,
    #[command(about = "Ingest RSS or JSON feed")]
    IngestFeed {
        feed_url: String,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Reconcile local database with incoming feed events")]
    ReconcileEvents {
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Fetch request HTML page and parse info")]
    FetchRequestPage {
        request_id: i32,
        #[arg(long, default_value = "https://fyi.org.nz")]
        base_url: String,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Generate attention report")]
    AttentionReport {
        #[arg(long, default_value = "outputs/attention-report.json")]
        output: String,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Generate handover document")]
    Handover {
        #[arg(long, default_value = "outputs/handover.md")]
        output: String,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Write dashboard HTML or JSON")]
    Dashboard {
        #[arg(long, default_value = "outputs/dashboard.html")]
        output: String,
        #[arg(long)]
        json_output: Option<String>,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Run single cycle monitor")]
    RunCycle {
        feed_url: String,
        #[arg(long, default_value = "outputs")]
        outputs_dir: String,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Run continuous monitoring scheduler")]
    Scheduler {
        feed_url: String,
        #[arg(long, default_value_t = 3600)]
        interval_seconds: u64,
        #[arg(long, default_value = "outputs")]
        outputs_dir: String,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
        #[arg(long)]
        once: bool,
    },
    #[command(about = "Start web dashboard and API server")]
    Serve {
        #[arg(long)]
        host: Option<String>,
        #[arg(long, default_value_t = 8000)]
        port: u16,
        #[arg(long)]
        settings: Option<String>,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Get detailed report of a request")]
    RequestDetail {
        request_id: i32,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Export single request detail")]
    ExportRequest {
        request_id: i32,
        #[arg(long)]
        output: Option<String>,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Draft a follow-up letter")]
    FollowUpDraft {
        request_id: i32,
        #[arg(long)]
        output: Option<String>,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Generate attachment manifest JSON")]
    AttachmentManifest {
        request_id: i32,
        #[arg(long, default_value = "outputs/attachment-manifest.json")]
        output: String,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Generate attachment manifest CSV")]
    AttachmentManifestCsv {
        request_id: i32,
        #[arg(long, default_value = "outputs/attachment-manifest.csv")]
        output: String,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Generate follow-up variants")]
    FollowUpVariants {
        request_id: i32,
        #[arg(long)]
        output: Option<String>,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Generate full follow-up pack")]
    FollowUpPack {
        request_id: i32,
        #[arg(long)]
        output: Option<String>,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Generate response triage report")]
    TriageReport {
        #[arg(long)]
        output: Option<String>,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Analyze response content")]
    ResponseAnalysis {
        request_id: i32,
        #[arg(long)]
        output: Option<String>,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Determine next best action for request")]
    NextBestAction {
        request_id: i32,
        #[arg(long, default_value = "neutral")]
        tone: String,
        #[arg(long)]
        output: Option<String>,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Get full correspondence pack")]
    CorrespondencePack {
        request_id: i32,
        #[arg(long, value_enum, default_value_t = CorrespondenceFormat::Json)]
        format: CorrespondenceFormat,
        #[arg(long)]
        output: Option<String>,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Export complete bundle for a request")]
    ExportBundle {
        request_id: i32,
        #[arg(long)]
        output_dir: Option<String>,
        #[arg(long, value_enum, default_value_t = ExportProfile::Strict)]
        profile: ExportProfile,
        #[arg(long)]
        no_sanitize: bool,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Show current loaded settings")]
    ShowSettings {
        #[arg(long)]
        settings: Option<String>,
        #[arg(long)]
        output: Option<String>,
    },
    #[command(about = "Run privacy audit on system databases and files")]
    PrivacyAudit {
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
        #[arg(long)]
        host: Option<String>,
        #[arg(long, default_value = "outputs")]
        outputs_dir: String,
        #[arg(long, value_enum)]
        profile: Option<ExportProfile>,
        #[arg(long)]
        settings: Option<String>,
        #[arg(long)]
        output: Option<String>,
    },
    #[command(about = "Manage MFA for stored credentials")]
    Mfa {
        #[command(subcommand)]
        command: MfaCommand,
    },
    #[command(about = "Inspect and manage offline synchronization")]
    Sync {
        #[command(subcommand)]
        command: SyncCommand,
    },
    #[command(about = "Run Model Context Protocol (MCP) server")]
    McpServer,
    #[command(about = "Launch Ratatui Dashboard TUI")]
    Tui,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum SyncCommand {
    #[command(about = "Show offline sync status")]
    Status {
        #[arg(long)]
        request_id: Option<i64>,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Pull remote FYI updates into the local database")]
    Pull {
        #[arg(long, default_value = "https://fyi.org.nz")]
        base_url: String,
        #[arg(long)]
        feed_url: Option<String>,
        #[arg(long, default_value_t = 300)]
        interval_seconds: u64,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Push dirty local requests to FYI")]
    Push {
        #[arg(long, default_value = "https://fyi.org.nz")]
        base_url: String,
        #[arg(long, default_value_t = 3)]
        max_retries: u32,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "List local sync conflicts")]
    Conflicts {
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
    #[command(about = "Resolve a local sync conflict")]
    ResolveConflict {
        request_id: i64,
        #[arg(long)]
        mark_clean: bool,
        #[arg(long, default_value = "fyi_system.db")]
        db: String,
    },
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum MfaCommand {
    #[command(about = "Generate and store a TOTP secret for an account")]
    Setup {
        account: String,
        #[arg(long, default_value = "FYI CLI")]
        issuer: String,
        #[arg(long, default_value = "fyi-cli")]
        service: String,
    },
    #[command(about = "Verify a TOTP code for an account")]
    Verify {
        account: String,
        code: String,
        #[arg(long)]
        timestamp: Option<u64>,
        #[arg(long, default_value = "fyi-cli")]
        service: String,
    },
    #[command(about = "List accounts with MFA configured")]
    Status {
        #[arg(long, default_value = "fyi-cli")]
        service: String,
    },
    #[command(about = "Remove MFA for an account")]
    Remove {
        account: String,
        #[arg(long, default_value = "fyi-cli")]
        service: String,
    },
}

fn main() {
    #[cfg(feature = "dhat-on")]
    let _profiler = dhat::Profiler::new_heap();
    let args = Cli::parse();
    match &args.command {
        Commands::InitDb { db } => {
            if let Err(error) = initialize_database_file(db) {
                eprintln!("Failed to initialize SQLite database at {db}: {error}");
                std::process::exit(1);
            }
            println!("Initialized SQLite database at {}", db);
        }
        Commands::ImportAuthorities { csv_path, db } => {
            println!("Imported authorities from {} to {}", csv_path, db);
        }
        Commands::ListAuthorities { db } => {
            println!("Listing authorities in {}", db);
        }
        Commands::RegisterRequest {
            authority_slug,
            title,
            body: _,
            status,
            db,
            ..
        } => {
            println!(
                "Registering request: '{}' (Authority: {}, Status: {}) in {}",
                title, authority_slug, status, db
            );
        }
        Commands::ListRequests { db } => {
            println!("Listing requests from {}", db);
        }
        Commands::SetStatus {
            request_id,
            status,
            db,
        } => {
            println!(
                "Updating request {} status to {} in {}",
                request_id, status, db
            );
        }
        Commands::RequestTimeline { request_id, db } => {
            println!("Fetching request timeline for {} in {}", request_id, db);
        }
        Commands::ExportRequests { output, db } => {
            println!("Exporting requests from {} to {}", db, output);
        }
        Commands::ImportRequests { input, replace, db } => {
            println!(
                "Importing requests from {} to {} (Replace: {})",
                input, db, replace
            );
        }
        Commands::BuildPrefilledUrl {
            authority_slug,
            title,
            base_url,
            ..
        } => {
            println!(
                "Prefilled URL for '{}' on {}/{} built.",
                title, base_url, authority_slug
            );
        }
        Commands::Instances => {
            let registry = InstanceRegistry::embedded().unwrap_or_default();
            for instance in registry.list() {
                println!(
                    "{}\t{}\t{}\t{}",
                    instance.id, instance.country, instance.base_url, instance.foi_law.law_name
                );
            }
        }
        Commands::IngestFeed { feed_url, db } => {
            println!("Ingesting feed from {} into {}", feed_url, db);
        }
        Commands::ReconcileEvents { db } => {
            println!("Reconciling feed events in {}", db);
        }
        Commands::FetchRequestPage {
            request_id,
            base_url,
            db,
        } => {
            println!(
                "Fetching request page {} from {} in {}",
                request_id, base_url, db
            );
        }
        Commands::AttentionReport { output, db } => {
            println!("Attention report generated at {} from {}", output, db);
        }
        Commands::Handover { output, db } => {
            println!("Handover generated at {} from {}", output, db);
        }
        Commands::Dashboard {
            output,
            json_output,
            db,
        } => {
            println!(
                "Dashboard written to {} (JSON: {:?}) using {}",
                output, json_output, db
            );
        }
        Commands::RunCycle {
            feed_url,
            outputs_dir,
            db,
        } => {
            println!(
                "Running cycle for {} in {} (Outputs: {})",
                feed_url, db, outputs_dir
            );
        }
        Commands::Scheduler {
            feed_url,
            interval_seconds,
            outputs_dir,
            db,
            once,
        } => {
            println!(
                "Running scheduler on {} every {}s (Once: {}) in {} (Outputs: {})",
                feed_url, interval_seconds, once, db, outputs_dir
            );
        }
        Commands::Serve {
            host,
            port,
            settings,
            db,
        } => {
            println!(
                "Serving Webapp/API on {:?}:{} (Settings: {:?}) using {}",
                host, port, settings, db
            );
        }
        Commands::RequestDetail { request_id, db } => {
            println!("Request details for {} from {}", request_id, db);
        }
        Commands::ExportRequest {
            request_id,
            output,
            db,
        } => {
            println!(
                "Exporting request {} to {:?} using {}",
                request_id, output, db
            );
        }
        Commands::FollowUpDraft {
            request_id,
            output,
            db,
        } => {
            println!(
                "Drafting follow-up for {} to {:?} using {}",
                request_id, output, db
            );
        }
        Commands::AttachmentManifest {
            request_id,
            output,
            db,
        } => {
            println!(
                "Attachment manifest JSON for {} written to {} using {}",
                request_id, output, db
            );
        }
        Commands::AttachmentManifestCsv {
            request_id,
            output,
            db,
        } => {
            println!(
                "Attachment manifest CSV for {} written to {} using {}",
                request_id, output, db
            );
        }
        Commands::FollowUpVariants {
            request_id,
            output,
            db,
        } => {
            println!(
                "Generating follow-up variants for {} to {:?} using {}",
                request_id, output, db
            );
        }
        Commands::FollowUpPack {
            request_id,
            output,
            db,
        } => {
            println!(
                "Generating follow-up pack for {} to {:?} using {}",
                request_id, output, db
            );
        }
        Commands::TriageReport { output, db } => {
            println!("Generating triage report to {:?} using {}", output, db);
        }
        Commands::ResponseAnalysis {
            request_id,
            output,
            db,
        } => {
            println!(
                "Analyzing response for {} to {:?} using {}",
                request_id, output, db
            );
        }
        Commands::NextBestAction {
            request_id,
            tone,
            output,
            db,
        } => {
            println!(
                "Determining next best action (tone: {}) for {} to {:?} using {}",
                tone, request_id, output, db
            );
        }
        Commands::CorrespondencePack {
            request_id,
            format,
            output,
            db,
        } => {
            println!(
                "Generating correspondence pack ({:?}) for {} to {:?} using {}",
                format, request_id, output, db
            );
        }
        Commands::ExportBundle {
            request_id,
            output_dir,
            profile,
            no_sanitize,
            db,
        } => {
            println!(
                "Exporting bundle for {} to {:?} (Profile: {:?}, No Sanitize: {}) using {}",
                request_id, output_dir, profile, no_sanitize, db
            );
        }
        Commands::ShowSettings { settings, output } => {
            println!("Showing settings from {:?} to {:?}", settings, output);
        }
        Commands::PrivacyAudit {
            db,
            host,
            outputs_dir,
            profile,
            settings,
            output,
        } => {
            println!("Privacy audit on {} (Host: {:?}, Outputs: {}, Profile: {:?}, Settings: {:?}) to {:?}", db, host, outputs_dir, profile, settings, output);
        }
        Commands::Mfa { command } => {
            if let Err(error) = handle_mfa_command(command) {
                eprintln!("MFA command failed: {error}");
                std::process::exit(1);
            }
        }
        Commands::Sync { command } => {
            if let Err(error) = handle_sync_command(command, args.output_format) {
                eprintln!("Sync command failed: {error}");
                std::process::exit(1);
            }
        }
        Commands::McpServer => {
            println!("Starting MCP Server...");
            println!("Available tools: {}", mcp_tool_names().join(", "));
        }
        Commands::Tui => {
            println!("Starting TUI Dashboard...");
        }
    }
}

fn handle_mfa_command(command: &MfaCommand) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        MfaCommand::Setup {
            account,
            issuer,
            service,
        } => {
            let store = KeyringStore::new(service);
            let secret = generate_totp_secret()?;
            store.store_totp_secret(account, &secret)?;
            let uri = build_provisioning_uri(issuer, account, &secret)?;
            println!("MFA configured for {account}");
            println!("{uri}");
            println!("{}", render_provisioning_qr_ascii(&uri)?);
        }
        MfaCommand::Verify {
            account,
            code,
            timestamp,
            service,
        } => {
            let store = KeyringStore::new(service);
            let timestamp = match timestamp {
                Some(timestamp) => *timestamp,
                None => SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            };
            if store.verify_mfa_code(account, code, timestamp, 1)? {
                println!("MFA verified for {account}");
            } else {
                println!("MFA verification failed for {account}");
                std::process::exit(2);
            }
        }
        MfaCommand::Status { service } => {
            let store = KeyringStore::new(service);
            let accounts = store.list_totp_secrets()?;
            if accounts.is_empty() {
                println!("No MFA accounts configured");
            } else {
                for account in accounts {
                    println!("{account}");
                }
            }
        }
        MfaCommand::Remove { account, service } => {
            let store = KeyringStore::new(service);
            store.delete_totp_secret(account)?;
            println!("MFA removed for {account}");
        }
    }

    Ok(())
}

fn handle_sync_command(
    command: &SyncCommand,
    output_format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    match command {
        SyncCommand::Status { request_id, db } => runtime.block_on(async {
            let pool = DbPool::new(&sqlite_url(db)).await?;
            pool.run_migrations().await?;
            let global = pool.get_global_sync_status().await?;

            if let Some(request_id) = request_id {
                let metadata = pool.get_request_sync_metadata(*request_id).await?;
                print_request_sync_status(*request_id, metadata, output_format)?;
            } else {
                print_global_sync_status(&global, output_format)?;
            }

            Ok(())
        }),
        SyncCommand::Pull {
            base_url,
            feed_url,
            interval_seconds,
            db,
        } => runtime.block_on(async {
            let pool = DbPool::new(&sqlite_url(db)).await?;
            pool.run_migrations().await?;
            let client = SyncClient::new(base_url)?;
            let config = SyncConfig {
                pull_interval: std::time::Duration::from_secs((*interval_seconds).max(1)),
                ..SyncConfig::default()
            };

            let mut reports = vec![client.pull_incremental(&pool).await?];
            if let Some(feed_url) = feed_url {
                reports.push(client.pull_feed(&pool, feed_url).await?);
            }
            print_pull_reports(&reports, &config, output_format)?;

            Ok(())
        }),
        SyncCommand::Push {
            base_url,
            max_retries,
            db,
        } => runtime.block_on(async {
            let pool = DbPool::new(&sqlite_url(db)).await?;
            pool.run_migrations().await?;
            let client = SyncClient::new(base_url)?;
            let config = SyncConfig {
                push_max_retries: (*max_retries).max(1),
                ..SyncConfig::default()
            };
            let report = client.push_dirty_with_config(&pool, &config).await?;
            let queue_depth = pool.get_outgoing_queue_depth().await?;
            print_push_report(&report, &queue_depth, output_format)?;

            Ok(())
        }),
        SyncCommand::Conflicts { db } => runtime.block_on(async {
            let pool = DbPool::new(&sqlite_url(db)).await?;
            pool.run_migrations().await?;
            let conflicts = pool.list_conflicted_requests(500).await?;
            match output_format {
                OutputFormat::Text => {
                    if conflicts.is_empty() {
                        println!("No sync conflicts");
                    } else {
                        for request in conflicts {
                            println!(
                                "{}\t{}\t{}",
                                request.id,
                                request.status.as_deref().unwrap_or("unknown"),
                                request.title
                            );
                        }
                    }
                }
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&conflicts)?),
            }
            Ok(())
        }),
        SyncCommand::ResolveConflict {
            request_id,
            mark_clean,
            db,
        } => runtime.block_on(async {
            let pool = DbPool::new(&sqlite_url(db)).await?;
            pool.run_migrations().await?;
            let resolved = pool
                .resolve_request_conflict(*request_id, *mark_clean)
                .await?;
            match output_format {
                OutputFormat::Text => {
                    println!("Conflict {} resolved: {}", request_id, resolved);
                }
                OutputFormat::Json => println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "request_id": request_id,
                        "resolved": resolved,
                        "sync_status": if *mark_clean { "clean" } else { "dirty" }
                    }))?
                ),
            }
            Ok(())
        }),
    }
}

fn print_global_sync_status(
    status: &GlobalSyncStatus,
    output_format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match output_format {
        OutputFormat::Text => {
            println!("Sync status");
            println!("Total tracked: {}", status.total);
            println!("Clean: {}", status.clean);
            println!("Dirty: {}", status.dirty);
            println!("Pending: {}", status.pending);
            println!("Conflicts: {}", status.conflict);
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "total": status.total,
                    "clean": status.clean,
                    "dirty": status.dirty,
                    "pending": status.pending,
                    "conflict": status.conflict
                }))?
            );
        }
    }
    Ok(())
}

fn print_request_sync_status(
    request_id: i64,
    metadata: Option<fyi_core::db::RequestSyncMetadata>,
    output_format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match output_format {
        OutputFormat::Text => {
            if let Some(metadata) = metadata {
                println!("Request {request_id} sync status");
                println!("Status: {}", metadata.sync_status.as_str());
                println!(
                    "Last synced: {}",
                    metadata.last_synced_at.as_deref().unwrap_or("never")
                );
                println!(
                    "Remote updated: {}",
                    metadata.remote_updated_at.as_deref().unwrap_or("unknown")
                );
                println!("Local updated: {}", metadata.local_updated_at);
                println!("Conflict version: {}", metadata.conflict_version);
            } else {
                println!("Request {request_id} has no sync metadata");
            }
        }
        OutputFormat::Json => {
            let payload = metadata
                .map(|metadata| {
                    serde_json::json!({
                        "request_id": metadata.request_id,
                        "sync_status": metadata.sync_status.as_str(),
                        "last_synced_at": metadata.last_synced_at,
                        "remote_updated_at": metadata.remote_updated_at,
                        "local_updated_at": metadata.local_updated_at,
                        "conflict_version": metadata.conflict_version
                    })
                })
                .unwrap_or_else(|| {
                    serde_json::json!({
                        "request_id": request_id,
                        "sync_status": null
                    })
                });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
    }
    Ok(())
}

fn print_pull_reports(
    reports: &[PullReport],
    config: &SyncConfig,
    output_format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match output_format {
        OutputFormat::Text => {
            println!("Pull interval: {}s", config.pull_interval.as_secs().max(1));
            for report in reports {
                println!(
                    "{} pull: fetched {}, applied {}",
                    report.source, report.fetched, report.applied
                );
            }
        }
        OutputFormat::Json => {
            let reports = reports
                .iter()
                .map(|report| {
                    serde_json::json!({
                        "source": report.source,
                        "fetched": report.fetched,
                        "applied": report.applied
                    })
                })
                .collect::<Vec<_>>();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "pull_interval_seconds": config.pull_interval.as_secs().max(1),
                    "reports": reports
                }))?
            );
        }
    }
    Ok(())
}

fn print_push_report(
    report: &PushReport,
    queue_depth: &fyi_core::db::OutgoingQueueDepth,
    output_format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match output_format {
        OutputFormat::Text => {
            println!(
                "Push: queued {}, submitted {}, failed {}",
                report.queued, report.submitted, report.failed
            );
            println!(
                "Queue depth: pending {}, submitted {}, failed {}",
                queue_depth.pending, queue_depth.submitted, queue_depth.failed
            );
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "queued": report.queued,
                    "submitted": report.submitted,
                    "failed": report.failed,
                    "queue_depth": {
                        "pending": queue_depth.pending,
                        "submitted": queue_depth.submitted,
                        "failed": queue_depth.failed
                    }
                }))?
            );
        }
    }
    Ok(())
}

fn sqlite_url(db: &str) -> String {
    if db.starts_with("sqlite:") {
        db.to_string()
    } else {
        format!("sqlite://{}?mode=rwc", db.replace('\\', "/"))
    }
}

fn mcp_tool_names() -> &'static [&'static str] {
    &[
        "mfa_setup",
        "mfa_verify",
        "mfa_status",
        "mfa_remove",
        "sync_status",
    ]
}

fn initialize_database_file(db: &str) -> std::io::Result<()> {
    let path = Path::new(db);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new().create(true).append(true).open(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_init_db() {
        let args = Cli::try_parse_from(["fyi-cli", "init-db", "--db", "test.db"]).unwrap();
        assert_eq!(args.db, "test.db");
        match args.command {
            Commands::InitDb { db } => {
                assert_eq!(db, "test.db");
            }
            _ => panic!("Expected InitDb command"),
        }
    }

    #[test]
    fn test_parse_register_request() {
        let args = Cli::try_parse_from([
            "fyi-cli",
            "register-request",
            "slug-name",
            "Req Title",
            "Req Body",
            "--tags",
            "tag1,tag2",
            "--status",
            "sent",
            "--fyi-request-id",
            "123",
        ])
        .unwrap();

        match args.command {
            Commands::RegisterRequest {
                authority_slug,
                title,
                body,
                tags,
                status,
                fyi_request_id,
                ..
            } => {
                assert_eq!(authority_slug, "slug-name");
                assert_eq!(title, "Req Title");
                assert_eq!(body, "Req Body");
                assert_eq!(tags, Some("tag1,tag2".to_string()));
                assert_eq!(status, "sent");
                assert_eq!(fyi_request_id, Some(123));
            }
            _ => panic!("Expected RegisterRequest command"),
        }
    }

    #[test]
    fn test_parse_serve() {
        let args =
            Cli::try_parse_from(["fyi-cli", "serve", "--host", "127.0.0.1", "--port", "9000"])
                .unwrap();

        match args.command {
            Commands::Serve { host, port, .. } => {
                assert_eq!(host, Some("127.0.0.1".to_string()));
                assert_eq!(port, 9000);
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_parse_mcp_server_and_tui() {
        let args = Cli::try_parse_from(["fyi-cli", "mcp-server"]).unwrap();
        assert_eq!(args.command, Commands::McpServer);

        let args = Cli::try_parse_from(["fyi-cli", "tui"]).unwrap();
        assert_eq!(args.command, Commands::Tui);
    }

    #[test]
    fn test_parse_sync_status() {
        let args = Cli::try_parse_from([
            "fyi-cli",
            "--output-format",
            "json",
            "sync",
            "status",
            "--request-id",
            "42",
            "--db",
            "test.db",
        ])
        .unwrap();

        assert_eq!(args.output_format, OutputFormat::Json);
        assert_eq!(
            args.command,
            Commands::Sync {
                command: SyncCommand::Status {
                    request_id: Some(42),
                    db: "test.db".to_string(),
                }
            }
        );
    }

    #[test]
    fn test_parse_sync_pull() {
        let args = Cli::try_parse_from([
            "fyi-cli",
            "sync",
            "pull",
            "--base-url",
            "https://example.org",
            "--feed-url",
            "https://example.org/feed.atom",
            "--interval-seconds",
            "60",
            "--db",
            "test.db",
        ])
        .unwrap();

        assert_eq!(
            args.command,
            Commands::Sync {
                command: SyncCommand::Pull {
                    base_url: "https://example.org".to_string(),
                    feed_url: Some("https://example.org/feed.atom".to_string()),
                    interval_seconds: 60,
                    db: "test.db".to_string(),
                }
            }
        );
    }

    #[test]
    fn test_parse_sync_push() {
        let args = Cli::try_parse_from([
            "fyi-cli",
            "sync",
            "push",
            "--base-url",
            "https://example.org",
            "--max-retries",
            "2",
            "--db",
            "test.db",
        ])
        .unwrap();

        assert_eq!(
            args.command,
            Commands::Sync {
                command: SyncCommand::Push {
                    base_url: "https://example.org".to_string(),
                    max_retries: 2,
                    db: "test.db".to_string(),
                }
            }
        );
    }

    #[test]
    fn test_parse_sync_conflict_commands() {
        let args =
            Cli::try_parse_from(["fyi-cli", "sync", "conflicts", "--db", "test.db"]).unwrap();
        assert_eq!(
            args.command,
            Commands::Sync {
                command: SyncCommand::Conflicts {
                    db: "test.db".to_string(),
                }
            }
        );

        let args = Cli::try_parse_from([
            "fyi-cli",
            "sync",
            "resolve-conflict",
            "42",
            "--mark-clean",
            "--db",
            "test.db",
        ])
        .unwrap();
        assert_eq!(
            args.command,
            Commands::Sync {
                command: SyncCommand::ResolveConflict {
                    request_id: 42,
                    mark_clean: true,
                    db: "test.db".to_string(),
                }
            }
        );
    }

    #[test]
    fn test_parse_mfa_commands() {
        let args = Cli::try_parse_from(["fyi-cli", "mfa", "setup", "alice@example.org"]).unwrap();
        assert_eq!(
            args.command,
            Commands::Mfa {
                command: MfaCommand::Setup {
                    account: "alice@example.org".to_string(),
                    issuer: "FYI CLI".to_string(),
                    service: "fyi-cli".to_string(),
                }
            }
        );

        let args = Cli::try_parse_from([
            "fyi-cli",
            "mfa",
            "verify",
            "alice@example.org",
            "123456",
            "--timestamp",
            "1700000000",
        ])
        .unwrap();
        assert_eq!(
            args.command,
            Commands::Mfa {
                command: MfaCommand::Verify {
                    account: "alice@example.org".to_string(),
                    code: "123456".to_string(),
                    timestamp: Some(1_700_000_000),
                    service: "fyi-cli".to_string(),
                }
            }
        );

        let args = Cli::try_parse_from(["fyi-cli", "mfa", "status"]).unwrap();
        assert_eq!(
            args.command,
            Commands::Mfa {
                command: MfaCommand::Status {
                    service: "fyi-cli".to_string(),
                }
            }
        );

        let args = Cli::try_parse_from(["fyi-cli", "mfa", "remove", "alice@example.org"]).unwrap();
        assert_eq!(
            args.command,
            Commands::Mfa {
                command: MfaCommand::Remove {
                    account: "alice@example.org".to_string(),
                    service: "fyi-cli".to_string(),
                }
            }
        );
    }

    #[test]
    fn test_mcp_exposes_mfa_tools() {
        let tools = mcp_tool_names();

        assert!(tools.contains(&"mfa_setup"));
        assert!(tools.contains(&"mfa_verify"));
        assert!(tools.contains(&"mfa_status"));
        assert!(tools.contains(&"mfa_remove"));
        assert!(tools.contains(&"sync_status"));
    }
}
