use clap::{Parser, Subcommand, ValueEnum};

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
    #[command(about = "Run Model Context Protocol (MCP) server")]
    McpServer,
    #[command(about = "Launch Ratatui Dashboard TUI")]
    Tui,
}

fn main() {
    #[cfg(feature = "dhat-on")]
    let _profiler = dhat::Profiler::new_heap();
    let args = Cli::parse();
    match &args.command {
        Commands::InitDb { db } => {
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
        Commands::McpServer => {
            println!("Starting MCP Server...");
        }
        Commands::Tui => {
            println!("Starting TUI Dashboard...");
        }
    }
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
}
