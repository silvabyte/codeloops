mod config;
mod http;
mod mcp;
mod opencode;

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
    #[arg(
        long,
        env = "CODELOOPS_ADDRESS",
        default_value = "127.0.0.1:47823",
        global = true
    )]
    address: SocketAddr,
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Serve,
    Mcp,
    /// Read one versioned envelope from stdin and submit it to the service.
    Capture,
    /// Durably queue one native OpenCode event from stdin (works while offline).
    CaptureOpencode,
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
    let config = Config::open(
        cli.data_dir.unwrap_or_else(config::default_root),
        cli.address,
    )?;
    let client = http::Client::new(config.clone())?;
    let value: Value = match cli.command {
        Command::Serve => return http::serve(config).await,
        Command::Mcp => return mcp::serve(client).await,
        Command::Capture => {
            client
                .post("/v1/history/ingest", &stdin::<Capture>()?)
                .await?
        }
        Command::CaptureOpencode => match stdin()
            .and_then(|input| opencode::enqueue(&config, input))
        {
            Ok(receipt) => receipt,
            Err(error) => {
                if let Err(health_error) = opencode::record_failure(&config, &error.to_string()) {
                    eprintln!("could not record capture failure: {health_error}");
                }
                return Err(error);
            }
        },
        Command::Flush => {
            opencode::flush(&config)?;
            opencode::health(&config)?
        }
        Command::Health => client.post("/v1/health", &json!({})).await?,
        Command::History { command } => {
            client
                .post("/v1/history/query", &Query::from(*command))
                .await?
        }
    };
    if cli.json {
        println!("{}", serde_json::to_string(&value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&value)?);
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run(Cli::parse()).await {
        if let Some(api) = error.downcast_ref::<http::ApiError>() {
            eprintln!("{}", json!({"error":api}));
        } else {
            eprintln!(
                "{}",
                json!({"error":{"code":"command_failed","message":error.to_string()}})
            );
        }
        std::process::exit(1);
    }
}
