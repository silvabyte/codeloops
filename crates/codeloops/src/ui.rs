use std::sync::Arc;

use anyhow::{Context, Result};

use codeloops_db::Database;
use codeloops_sessions::{SessionStore, SessionWatcher};

use crate::api;

pub async fn handle_ui_command(dev: bool, api_port: u16, ui_port: u16) -> Result<()> {
    use colored::Colorize;

    let store = Arc::new(SessionStore::new()?);
    let watcher = Arc::new(SessionWatcher::new().context("Failed to start session watcher")?);
    let working_dir = std::env::current_dir().context("Failed to get current directory")?;
    let db = Arc::new(Database::open().context("Failed to initialize database")?);

    let router = api::create_router(store, watcher, working_dir, db);

    // Start the API server
    let api_addr = format!("0.0.0.0:{}", api_port);
    let listener = tokio::net::TcpListener::bind(&api_addr)
        .await
        .with_context(|| format!("Failed to bind API server to {}", api_addr))?;

    // Start the frontend
    let mut ui_child = if dev {
        start_dev_server(ui_port, api_port).await?
    } else {
        start_prod_server(ui_port).await?
    };

    // Print clean startup message
    eprintln!();
    eprintln!(
        "  {} {}",
        "->".bright_green(),
        format!("Open http://localhost:{}", ui_port).bold()
    );
    eprintln!("  {} Press {} to stop", "->".dimmed(), "Ctrl+C".bold());
    eprintln!();

    // Open browser
    let ui_url = format!("http://localhost:{}", ui_port);
    if let Err(e) = open::that(&ui_url) {
        eprintln!("Failed to open browser: {} (open {} manually)", e, ui_url);
    }

    // Run API server until interrupted
    let result = axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await;

    // Cleanup child process — send SIGTERM first, then SIGKILL after timeout
    kill_child(&mut ui_child).await;

    result.context("API server error")
}

async fn kill_child(child: &mut tokio::process::Child) {
    // Start kill (sends SIGKILL on unix)
    let _ = child.start_kill();

    // Wait for it to actually exit, with a timeout
    match tokio::time::timeout(std::time::Duration::from_secs(3), child.wait()).await {
        Ok(_) => {}
        Err(_) => {
            eprintln!("Warning: UI process did not exit in time");
        }
    }
}

async fn start_dev_server(ui_port: u16, api_port: u16) -> Result<tokio::process::Child> {
    let ui_dir = find_ui_dir()?;

    eprintln!("Starting dev server on http://localhost:{}", ui_port);

    let mut cmd = tokio::process::Command::new("bun");
    cmd.args(["run", "dev", "--port", &ui_port.to_string()])
        .env("VITE_API_URL", format!("http://localhost:{}", api_port))
        .current_dir(&ui_dir)
        .kill_on_drop(true)
        .process_group(0); // New process group — Ctrl+C won't propagate

    let child = cmd.spawn().with_context(|| {
        format!(
            "Failed to start bun dev server. Make sure 'bun' is installed and {} exists.",
            ui_dir.display()
        )
    })?;

    Ok(child)
}

async fn start_prod_server(ui_port: u16) -> Result<tokio::process::Child> {
    let ui_binary = find_ui_binary()?;

    eprintln!("Starting UI server on http://localhost:{}", ui_port);

    let mut cmd = tokio::process::Command::new(&ui_binary);
    cmd.env("PORT", ui_port.to_string())
        .kill_on_drop(true)
        .process_group(0); // New process group — Ctrl+C won't propagate

    let child = cmd
        .spawn()
        .with_context(|| format!("Failed to start UI binary: {}", ui_binary.display()))?;

    Ok(child)
}

fn find_ui_dir() -> Result<std::path::PathBuf> {
    // Check CODELOOPS_UI_DIR env var
    if let Ok(dir) = std::env::var("CODELOOPS_UI_DIR") {
        let path = std::path::PathBuf::from(dir);
        if path.exists() {
            return Ok(path);
        }
    }

    // Check relative to the binary
    if let Ok(exe) = std::env::current_exe() {
        // Development: binary is in target/debug or target/release
        // ui/ is at the workspace root
        if let Some(workspace_root) = exe
            .parent() // target/debug
            .and_then(|p| p.parent()) // target
            .and_then(|p| p.parent())
        // workspace root
        {
            let ui_dir = workspace_root.join("ui");
            if ui_dir.exists() {
                return Ok(ui_dir);
            }
        }
    }

    // Check ~/.local/share/codeloops/ui/
    if let Some(data_dir) = dirs::data_dir() {
        let ui_dir = data_dir.join("codeloops").join("ui");
        if ui_dir.exists() {
            return Ok(ui_dir);
        }
    }

    anyhow::bail!(
        "Could not find UI directory. Set CODELOOPS_UI_DIR or run from the workspace root."
    )
}

fn find_ui_binary() -> Result<std::path::PathBuf> {
    // Check same directory as codeloops binary
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidate = parent.join("codeloops-ui");
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        // Development: binary is in target/debug or target/release,
        // codeloops-ui is in ui/ at the workspace root
        if let Some(workspace_root) = exe
            .parent() // target/debug
            .and_then(|p| p.parent()) // target
            .and_then(|p| p.parent())
        // workspace root
        {
            let candidate = workspace_root.join("ui").join("codeloops-ui");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    // Check PATH
    if let Ok(output) = std::process::Command::new("which")
        .arg("codeloops-ui")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok(std::path::PathBuf::from(path));
        }
    }

    // Check ~/.local/bin/
    if let Some(home) = dirs::home_dir() {
        let candidate = home.join(".local").join("bin").join("codeloops-ui");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "Could not find 'codeloops-ui' binary. Build the UI first with 'bun run compile' in the ui/ directory."
    )
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");
    eprintln!("\nShutting down...");
}
