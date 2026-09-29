//! Native user-service lifecycle. The installed profile owns the definition;
//! systemd/launchd owns process supervision and lifetime outside the terminal.
use crate::{AppResult, config::Config, http, installation};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Output, Stdio},
    time::Duration,
};
use tokio::process::Command as Process;

const RECORD: &str = "share/codeloops/service.json";

#[derive(Subcommand)]
pub enum Command {
    /// Enable login startup and start/restart the background service.
    Start,
    /// Stop the background service and disable login startup.
    Stop,
    /// Show managed process state and archive availability.
    Status,
    /// Show recent service logs.
    Logs,
}

#[derive(Clone, Serialize, Deserialize)]
enum Manager {
    Systemd,
    Launchd { uid: u32 },
}

#[derive(Serialize, Deserialize)]
struct Service {
    manager: Manager,
    name: String,
    path: PathBuf,
    contents: String,
    log: PathBuf,
    #[serde(default)]
    previous: Option<String>,
}

async fn output(program: &str, args: &[&str]) -> AppResult<Output> {
    let mut command = Process::new(program);
    command.args(args).stdin(Stdio::null()).kill_on_drop(true);
    Ok(
        tokio::time::timeout(Duration::from_secs(20), command.output())
            .await
            .map_err(|_| format!("{program} timed out"))?
            .map_err(|error| format!("could not run {program}: {error}"))?,
    )
}

async fn checked(program: &str, args: &[&str]) -> AppResult<String> {
    let result = output(program, args).await?;
    if !result.status.success() {
        return Err(format!(
            "{program} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&result.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8(result.stdout)?)
}

fn text(path: &Path) -> AppResult<&str> {
    let value = path.to_str().ok_or("service paths must be UTF-8")?;
    if value.chars().any(char::is_control) {
        return Err("service paths must not contain control characters".into());
    }
    Ok(value)
}

fn systemd_quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
            .replace('$', "$$")
    )
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

impl Service {
    fn definition(
        manager: Manager,
        prefix: &Path,
        profile: &str,
        settings: &installation::Settings,
        home: &Path,
    ) -> AppResult<Self> {
        let hash = installation::hash(text(prefix)?.as_bytes());
        let name = format!("codeloops-history-{profile}-{}", &hash[..12]);
        let binary = prefix.join("bin/codeloops");
        let address = settings.address.to_string();
        let args = [
            text(&binary)?,
            "--data-dir",
            text(&settings.root)?,
            "--address",
            &address,
            "serve",
        ];
        let log = prefix.join("share/codeloops/service.log");
        let (path, contents) = match manager {
            Manager::Systemd => {
                let config = std::env::var_os("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home.join(".config"));
                let path = config.join("systemd/user").join(format!("{name}.service"));
                // systemd rejects some otherwise-valid filename characters in
                // the executable position. env execs the absolute binary path
                // without a shell and preserves the managed PID.
                let command = std::iter::once("/usr/bin/env")
                    .chain(args.iter().copied())
                    .map(systemd_quote)
                    .collect::<Vec<_>>()
                    .join(" ");
                (
                    path,
                    format!(
                        "[Unit]\nDescription=CodeLoops conversation history ({profile})\n\n\
                     [Service]\nType=exec\nExecStart={command}\nRestart=on-failure\nRestartSec=2\n\
                     KillSignal=SIGINT\nTimeoutStopSec=15\nUMask=0077\n\n\
                     [Install]\nWantedBy=default.target\n"
                    ),
                )
            }
            Manager::Launchd { .. } => {
                let arguments = args
                    .iter()
                    .map(|arg| format!("<string>{}</string>", xml(arg)))
                    .collect::<String>();
                let contents = format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                     <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
                     <plist version=\"1.0\"><dict>\n\
                     <key>Label</key><string>{name}</string>\n\
                     <key>ProgramArguments</key><array>{arguments}</array>\n\
                     <key>RunAtLoad</key><true/>\n\
                     <key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict>\n\
                     <key>ThrottleInterval</key><integer>2</integer>\n\
                     <key>ExitTimeOut</key><integer>15</integer>\n\
                     <key>Umask</key><integer>63</integer>\n\
                     <key>StandardOutPath</key><string>{log}</string>\n\
                     <key>StandardErrorPath</key><string>{log}</string>\n\
                     </dict></plist>\n",
                    log = xml(text(&log)?)
                );
                (
                    home.join("Library/LaunchAgents")
                        .join(format!("{name}.plist")),
                    contents,
                )
            }
        };
        let path = std::path::absolute(path)?;
        text(&path)?;
        Ok(Self {
            manager,
            name,
            path,
            contents,
            log,
            previous: None,
        })
    }

    fn unit(&self) -> String {
        format!("{}.service", self.name)
    }

    fn target(&self, uid: u32) -> String {
        format!("gui/{uid}/{}", self.name)
    }

    async fn available(&self) -> AppResult<()> {
        match self.manager {
            Manager::Systemd => {
                checked("systemctl", &["--user", "show-environment"]).await
                    .map_err(|error| format!("A systemd user session is required: {error}. For foreground development, use make run."))?;
            }
            Manager::Launchd { uid } => {
                checked("launchctl", &["print", &format!("gui/{uid}")]).await?;
            }
        }
        Ok(())
    }

    async fn running(&self) -> AppResult<bool> {
        match self.manager {
            Manager::Systemd => Ok(checked(
                "systemctl",
                &[
                    "--user",
                    "show",
                    &self.unit(),
                    "--property=ActiveState",
                    "--value",
                ],
            )
            .await?
            .trim()
                == "active"),
            Manager::Launchd { uid } => {
                self.available().await?;
                let result = output("launchctl", &["print", &self.target(uid)]).await?;
                Ok(result.status.success()
                    && String::from_utf8_lossy(&result.stdout)
                        .lines()
                        .any(|line| line.trim() == "state = running"))
            }
        }
    }

    async fn start(&self) -> AppResult<()> {
        match self.manager {
            Manager::Systemd => {
                checked("systemctl", &["--user", "daemon-reload"]).await?;
                checked("systemctl", &["--user", "enable", text(&self.path)?]).await?;
                checked("systemctl", &["--user", "restart", &self.unit()]).await?;
            }
            Manager::Launchd { uid } => {
                checked("launchctl", &["enable", &self.target(uid)]).await?;
                checked(
                    "launchctl",
                    &["bootstrap", &format!("gui/{uid}"), text(&self.path)?],
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn stop(&self) -> AppResult<()> {
        self.available().await?;
        match self.manager {
            Manager::Systemd => {
                let loaded = checked(
                    "systemctl",
                    &[
                        "--user",
                        "show",
                        &self.unit(),
                        "--property=LoadState",
                        "--value",
                    ],
                )
                .await?;
                if loaded.trim() != "not-found" {
                    if loaded.trim() == "loaded" {
                        checked("systemctl", &["--user", "stop", &self.unit()]).await?;
                    }
                    checked("systemctl", &["--user", "disable", &self.unit()]).await?;
                }
            }
            Manager::Launchd { uid } => {
                if output("launchctl", &["print", &self.target(uid)])
                    .await?
                    .status
                    .success()
                {
                    checked("launchctl", &["bootout", &self.target(uid)]).await?;
                }
                checked("launchctl", &["disable", &self.target(uid)]).await?;
            }
        }
        Ok(())
    }

    async fn logs(&self) -> AppResult<String> {
        match self.manager {
            Manager::Systemd => {
                checked(
                    "journalctl",
                    &["--user", "--unit", &self.unit(), "--lines=50", "--no-pager"],
                )
                .await
            }
            Manager::Launchd { .. } => {
                let mut file = match fs::File::open(&self.log) {
                    Ok(file) => file,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        return Ok("No service logs yet.\n".into());
                    }
                    Err(error) => return Err(error.into()),
                };
                let length = file.metadata()?.len().min(65536);
                file.seek(SeekFrom::End(-(length as i64)))?;
                let mut bytes = Vec::new();
                file.take(length).read_to_end(&mut bytes)?;
                Ok(String::from_utf8_lossy(&bytes).into_owned())
            }
        }
    }
}

fn load(prefix: &Path) -> AppResult<Option<Service>> {
    installation::read(&prefix.join(RECORD))?
        .map(|bytes| Ok(serde_json::from_slice(&bytes)?))
        .transpose()
}

fn verify(service: &Service) -> AppResult<()> {
    if let Some(bytes) = installation::read(&service.path)?
        && bytes != service.contents.as_bytes()
        && service.previous.as_deref().map(str::as_bytes) != Some(bytes.as_slice())
    {
        return Err(format!(
            "service definition was modified: {}",
            service.path.display()
        )
        .into());
    }
    Ok(())
}

async fn ready(service: &Service, settings: &installation::Settings) -> AppResult<()> {
    let client = http::Client::new(Config::open(settings.root.clone(), settings.address)?)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while tokio::time::Instant::now() < deadline {
        if service.running().await?
            && matches!(
                tokio::time::timeout(
                    Duration::from_secs(1),
                    client.post("/v1/health", &json!({}))
                )
                .await,
                Ok(Ok(_))
            )
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err("background service did not become healthy; run make logs for diagnostics".into())
}

async fn start(
    prefix: &Path,
    existing: Option<Service>,
) -> AppResult<(Service, installation::Settings)> {
    let (profile, settings) = installation::profile_settings(prefix)?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME is required")?);
    let manager = if cfg!(target_os = "linux") {
        Manager::Systemd
    } else if cfg!(target_os = "macos") {
        Manager::Launchd {
            uid: checked("id", &["-u"]).await?.trim().parse()?,
        }
    } else {
        return Err(
            "managed services support Linux and macOS; use make run for foreground development"
                .into(),
        );
    };
    let mut service = Service::definition(manager, prefix, &profile, &settings, &home)?;
    if let Some(old) = &existing {
        verify(old)?;
        if old.path != service.path {
            return Err(
                "service location changed; uninstall it using the original home/config paths first"
                    .into(),
            );
        }
        service.previous = Some(old.contents.clone());
        old.stop().await?;
    } else if installation::read(&service.path)?.is_some() {
        return Err(format!("unowned service definition: {}", service.path.display()).into());
    }
    service.available().await?;
    let listener = tokio::net::TcpListener::bind(settings.address).await.map_err(|error| format!(
                "cannot start background service at {}: {error}. If the old foreground copy is running, press Ctrl+C in that terminal once, then run make start again.", settings.address
            ))?;
    drop(listener);
    installation::atomic_write(
        &prefix.join(RECORD),
        &serde_json::to_vec_pretty(&service)?,
        false,
    )?;
    installation::atomic_write(&service.path, service.contents.as_bytes(), false)?;
    if matches!(service.manager, Manager::Launchd { .. }) {
        match fs::symlink_metadata(&service.log) {
            Ok(metadata) if metadata.file_type().is_file() => {}
            Ok(_) => return Err("service log must be a regular file".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                installation::atomic_write(&service.log, b"", false)?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    let result = async {
        service.start().await?;
        ready(&service, &settings).await
    }
    .await;
    if let Err(error) = result {
        if let Err(cleanup) = service.stop().await {
            return Err(format!("{error}; stopping failed service also failed: {cleanup}").into());
        }
        return Err(error);
    }
    service.previous = None;
    installation::atomic_write(
        &prefix.join(RECORD),
        &serde_json::to_vec_pretty(&service)?,
        false,
    )?;
    Ok((service, settings))
}

pub async fn run(command: Command, json_output: bool) -> AppResult<()> {
    let prefix = installation::prefix()?;
    let _lock = installation::lock(&prefix)?;
    let existing = load(&prefix)?;
    let report = match command {
        Command::Start => {
            let (service, settings) = start(&prefix, existing).await?;
            if !json_output {
                println!(
                    "CodeLoops is running in the background at http://{}.",
                    settings.address
                );
                println!("You can close this terminal. It will start again when you log in.");
                println!(
                    "Restart OpenCode and Codex, or open a new Cursor Agent Chat, to load the integration."
                );
                println!("Manage it with make status, make logs, and make stop.");
                return Ok(());
            }
            json!({"running": true, "login_startup": true, "service": service.name, "address": settings.address})
        }
        Command::Stop => {
            if let Some(service) = existing {
                verify(&service)?;
                service.stop().await?;
            }
            if !json_output {
                println!("The managed CodeLoops service is stopped. Login startup is disabled.");
                return Ok(());
            }
            json!({"running": false, "login_startup": false})
        }
        Command::Status => {
            let running = match &existing {
                Some(service) => service.running().await?,
                None => false,
            };
            let healthy = if running {
                let (_, settings) = installation::profile_settings(&prefix)?;
                let client = http::Client::new(Config::open(settings.root, settings.address)?)?;
                matches!(
                    tokio::time::timeout(
                        Duration::from_secs(1),
                        client.post("/v1/health", &json!({}))
                    )
                    .await,
                    Ok(Ok(_))
                )
            } else {
                false
            };
            if !json_output {
                println!(
                    "CodeLoops is {}.",
                    if healthy {
                        "healthy and running in the background"
                    } else if running {
                        "running but not responding; run make logs for diagnostics"
                    } else {
                        "not running as a managed service"
                    }
                );
                return Ok(());
            }
            json!({"running": running, "healthy": healthy, "service": existing.map(|service| service.name)})
        }
        Command::Logs => {
            let logs = match existing {
                Some(service) => service.logs().await?,
                None => "No managed service installed. Run make start.\n".into(),
            };
            if !json_output {
                print!("{logs}");
                return Ok(());
            }
            json!({"logs": logs})
        }
    };
    crate::print(report, true)
}

/// Called under the installation lock, before any installed assets are removed.
pub async fn remove(prefix: &Path) -> AppResult<()> {
    let Some(service) = load(prefix)? else {
        return Ok(());
    };
    verify(&service)?;
    service.stop().await?;
    if installation::read(&service.path)?.is_some() {
        fs::remove_file(&service.path)?;
    }
    match service.manager {
        Manager::Systemd => {
            checked("systemctl", &["--user", "daemon-reload"]).await?;
        }
        Manager::Launchd { uid } => {
            checked("launchctl", &["enable", &service.target(uid)]).await?;
        }
    }
    fs::remove_file(prefix.join(RECORD))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launchd_plist_parser_preserves_literal_paths_and_arguments() {
        let root = tempfile::tempdir().unwrap();
        let prefix = root.path().join("install 'quoted' & <tag> % $ ü");
        let settings = installation::Settings {
            root: root.path().join("history 'quoted' & <tag> % $ ü"),
            address: "127.0.0.1:47824".parse().unwrap(),
        };
        let service = Service::definition(
            Manager::Launchd { uid: 501 },
            &prefix,
            "fixture",
            &settings,
            root.path(),
        )
        .unwrap();
        let path = root.path().join("service.plist");
        fs::write(&path, service.contents).unwrap();
        let result = std::process::Command::new("python3")
            .args(["-c", "import json,plistlib,sys; print(json.dumps(plistlib.load(open(sys.argv[1], 'rb'))))"])
            .arg(path).output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let plist: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(
            plist["ProgramArguments"],
            json!([
                prefix.join("bin/codeloops"),
                "--data-dir",
                settings.root,
                "--address",
                "127.0.0.1:47824",
                "serve"
            ])
        );
        assert_eq!(plist["StandardErrorPath"], json!(service.log));
    }
}
