mod boundary;
mod codex_config;
mod config;
mod config_edits;
mod cursor;
mod export;
mod http;
mod installation;
mod mcp;
mod opencode;
mod outbox;
mod service;

use clap::{Args, Parser, Subcommand};
use config::Config;
use serde_json::{Value, json};
use session_history::model::{Capture, Filter, MAX_CAPTURE_BYTES, Page, Query};
use std::{io::Read, net::SocketAddr, path::PathBuf};

pub type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Parser)]
#[command(version, about = "Durable local conversation history")]
struct Cli {
    #[arg(long, env = "CODELOOPS_DATA_DIR", global = true)]
    data_dir: Option<PathBuf>,
    #[arg(long, env = "CODELOOPS_ADDRESS", global = true)]
    address: Option<SocketAddr>,
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Install this binary and embedded client assets into a separate prefix.
    Install {
        #[arg(long)]
        prefix: PathBuf,
    },
    /// Register capture and MCP in supported clients' user-global settings.
    Setup(installation::SetupArgs),
    /// Manage the background history service.
    Service {
        #[command(subcommand)]
        command: service::Command,
    },
    /// Remove this installation's owned registration and assets; keep history.
    Uninstall {
        /// Recover even when interrupted removal deleted the installed binary.
        #[arg(long)]
        prefix: Option<PathBuf>,
    },
    /// Verify a portable export without the service or original source checkout.
    VerifyExport {
        directory: PathBuf,
    },
    Serve,
    Mcp,
    /// Read one versioned envelope from stdin and submit it to the service.
    Capture,
    /// Durably queue one native OpenCode event from stdin (works while offline).
    CaptureOpencode,
    /// Observe one Cursor command-hook payload; always emit neutral hook output.
    CaptureCursor,
    /// Drain the durable adapter queue into the archive.
    Flush,
    Health,
    History {
        #[command(subcommand)]
        command: Box<HistoryCommand>,
    },
}

#[derive(Args)]
struct Paging {
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long)]
    cursor: Option<String>,
}
impl From<Paging> for Page {
    fn from(v: Paging) -> Self {
        Self {
            limit: v.limit,
            cursor: v.cursor,
        }
    }
}

#[derive(Args)]
struct Filtering {
    #[arg(long)]
    project_id: Option<String>,
    #[arg(long)]
    source: Option<String>,
    #[arg(long)]
    device_id: Option<String>,
    #[arg(long)]
    session_id: Option<String>,
    #[arg(long)]
    role: Option<String>,
    #[arg(long)]
    kind: Option<String>,
    #[arg(long)]
    since: Option<u64>,
    #[arg(long)]
    until: Option<u64>,
}
impl From<Filtering> for Filter {
    fn from(v: Filtering) -> Self {
        Self {
            project_id: v.project_id,
            source: v.source,
            device_id: v.device_id,
            session_id: v.session_id,
            role: v.role,
            kind: v.kind,
            since: v.since,
            until: v.until,
        }
    }
}

#[derive(Subcommand)]
enum HistoryCommand {
    Export {
        session_id: String,
        /// Download a self-contained bundle into a new directory.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Checkpoint {
        checkpoint_id: String,
    },
    Compare {
        before: String,
        after: String,
        #[arg(long, value_enum, default_value = "worktree")]
        before_layer: CliLayer,
        #[arg(long, value_enum, default_value = "worktree")]
        after_layer: CliLayer,
        #[command(flatten)]
        page: Paging,
    },
    Changes {
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        entry_id: Option<String>,
        #[arg(long)]
        workspace_id: String,
        #[command(flatten)]
        page: Paging,
    },
    File {
        checkpoint_id: String,
        /// Base64 path returned by a comparison.
        path: String,
        #[arg(long, value_enum, default_value = "worktree")]
        layer: CliLayer,
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long, default_value_t = 65536)]
        limit: usize,
    },
    List {
        #[command(flatten)]
        filter: Filtering,
        #[command(flatten)]
        page: Paging,
    },
    Search {
        text: String,
        #[command(flatten)]
        filter: Filtering,
        #[command(flatten)]
        page: Paging,
    },
    Show {
        session_id: String,
        #[command(flatten)]
        page: Paging,
    },
    Entry {
        entry_id: String,
    },
    Captures {
        session_id: String,
        #[command(flatten)]
        page: Paging,
    },
    Artifact {
        hash: String,
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long, default_value_t = 65536)]
        limit: usize,
    },
}

impl From<HistoryCommand> for Query {
    fn from(c: HistoryCommand) -> Self {
        match c {
            HistoryCommand::Export { session_id, .. } => Self::Export { session_id },
            HistoryCommand::Checkpoint { checkpoint_id } => Self::Checkpoint { checkpoint_id },
            HistoryCommand::Compare {
                before,
                after,
                before_layer,
                after_layer,
                page,
            } => Self::Compare {
                before,
                after,
                before_layer: before_layer.into(),
                after_layer: after_layer.into(),
                page: page.into(),
            },
            HistoryCommand::Changes {
                session_id,
                entry_id,
                workspace_id,
                page,
            } => Self::Changes {
                session_id,
                entry_id,
                workspace_id,
                page: page.into(),
            },
            HistoryCommand::File {
                checkpoint_id,
                path,
                layer,
                offset,
                limit,
            } => Self::File {
                checkpoint_id,
                path,
                layer: layer.into(),
                offset,
                limit,
            },
            HistoryCommand::List { filter, page } => Self::List {
                filter: filter.into(),
                page: page.into(),
            },
            HistoryCommand::Search { text, filter, page } => Self::Search {
                text,
                filter: filter.into(),
                page: page.into(),
            },
            HistoryCommand::Show { session_id, page } => Self::Show {
                session_id,
                page: page.into(),
            },
            HistoryCommand::Entry { entry_id } => Self::Entry { entry_id },
            HistoryCommand::Captures { session_id, page } => Self::Captures {
                session_id,
                page: page.into(),
            },
            HistoryCommand::Artifact {
                hash,
                offset,
                limit,
            } => Self::Artifact {
                hash,
                offset,
                limit,
            },
        }
    }
}

#[derive(Clone, clap::ValueEnum)]
enum CliLayer {
    Head,
    Index,
    Worktree,
}
impl From<CliLayer> for session_history::model::Layer {
    fn from(value: CliLayer) -> Self {
        match value {
            CliLayer::Head => Self::Head,
            CliLayer::Index => Self::Index,
            CliLayer::Worktree => Self::Worktree,
        }
    }
}

fn stdin<T: serde::de::DeserializeOwned>() -> AppResult<T> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(MAX_CAPTURE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_CAPTURE_BYTES {
        return Err("stdin exceeds 4 MiB".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

async fn run(cli: Cli) -> AppResult<()> {
    match &cli.command {
        Command::Install { prefix } => return print(installation::install(prefix)?, cli.json),
        Command::Uninstall { prefix } => {
            return print(installation::uninstall(prefix.as_deref()).await?, cli.json);
        }
        Command::VerifyExport { directory } => return print(export::verify(directory)?, cli.json),
        _ => {}
    }
    if let Command::Service { command } = cli.command {
        return service::run(command, cli.json).await;
    }
    let settings = installation::settings()?;
    let config = Config::open(
        cli.data_dir
            .or_else(|| settings.as_ref().map(|settings| settings.root.clone()))
            .unwrap_or_else(config::default_root),
        cli.address
            .or_else(|| settings.as_ref().map(|settings| settings.address))
            .unwrap_or_else(|| "127.0.0.1:47823".parse().expect("fixed loopback address")),
    )?;
    let client = http::Client::new(config.clone())?;
    let value: Value = match cli.command {
        Command::Install { .. }
        | Command::Uninstall { .. }
        | Command::VerifyExport { .. }
        | Command::Service { .. } => {
            unreachable!()
        }
        Command::Setup(args) => {
            let mut report = installation::setup(&config, args)?;
            report["capture_health"] = outbox::health(&config)?;
            report["service_health"] = match client.post("/v1/health", &json!({})).await {
                Ok(health) => health,
                Err(error) => json!({
                    "available": false,
                    "error": error,
                }),
            };
            report
        }
        Command::Serve => return http::serve(config).await,
        Command::Mcp => return mcp::serve(client).await,
        Command::Capture => {
            client
                .post("/v1/history/ingest", &stdin::<Capture>()?)
                .await?
        }
        Command::CaptureOpencode => {
            match stdin().and_then(|input| opencode::enqueue(&config, input)) {
                Ok(receipt) => receipt,
                Err(error) => {
                    if let Err(health_error) =
                        outbox::record_failure(&config, "opencode", &error.to_string())
                    {
                        eprintln!("could not record capture failure: {health_error}");
                    }
                    return Err(error);
                }
            }
        }
        Command::CaptureCursor => {
            if let Err(error) = stdin().and_then(|input| cursor::enqueue(&config, input)) {
                if let Err(health_error) =
                    outbox::record_failure(&config, "cursor", &error.to_string())
                {
                    eprintln!("could not record capture failure: {health_error}");
                }
                return Err(error);
            }
            json!({})
        }
        Command::Flush => outbox::drain(&config)?,
        Command::Health => client.post("/v1/health", &json!({})).await?,
        Command::History { command } => {
            let output = match command.as_ref() {
                HistoryCommand::Export { output, .. } => output.clone(),
                _ => None,
            };
            let result = client
                .post("/v1/history/query", &Query::from(*command))
                .await?;
            match output {
                Some(output) => export::download_bundle(&client, result, &output).await?,
                None => result,
            }
        }
    };
    print(value, cli.json)
}

fn print(value: Value, compact: bool) -> AppResult<()> {
    if compact {
        println!("{}", serde_json::to_string(&value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&value)?);
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let cursor_hook = matches!(cli.command, Command::CaptureCursor);
    if let Err(error) = run(cli).await {
        if let Some(api) = error.downcast_ref::<http::ApiError>() {
            eprintln!("{}", json!({"error": api}));
        } else {
            eprintln!(
                "{}",
                json!({
                    "error": {
                        "code": "command_failed",
                        "message": error.to_string(),
                    },
                })
            );
        }
        if cursor_hook {
            // Even initialization/parse/storage failures must not gate a prompt,
            // supply context, or schedule another turn. Diagnostics stay on stderr.
            println!("{{}}");
        } else {
            std::process::exit(1);
        }
    }
}
