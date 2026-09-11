//! User installation and reversible client registration. A write-ahead ownership
//! record makes interrupted setup repeatable without restoring whole client files.
use crate::{
    AppResult,
    config::Config,
    config_edits::{self, Edit},
};
use clap::Args;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    net::SocketAddr,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

const INSTALL_RECORD: &str = "share/codeloops/install.json";
const SETUP_RECORD: &str = "share/codeloops/setup.json";
const ADAPTER: &str = "share/codeloops/adapters/opencode/history.ts";
const CURSOR: &str = "share/codeloops/adapters/cursor/hooks.example.json";
const WRAPPER: &str = "share/codeloops/configured-history.ts";

#[derive(Args)]
pub struct SetupArgs {
    #[arg(long, default_value = "preview")]
    pub profile: String,
    /// Override the global OpenCode config location (also accepts JSONC).
    #[arg(long)]
    pub opencode_config: Option<PathBuf>,
    /// Override ~/.cursor for an isolated client environment.
    #[arg(long)]
    pub cursor_config_dir: Option<PathBuf>,
    #[arg(long, env = "CODELOOPS_OPENCODE_VERSION")]
    pub opencode_version: Option<String>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub root: PathBuf,
    pub address: SocketAddr,
}

#[derive(Serialize, Deserialize)]
struct Installed {
    schema_version: u32,
    assets: BTreeMap<PathBuf, String>,
}

#[derive(Serialize, Deserialize)]
struct Setup {
    schema_version: u32,
    profile: String,
    settings: Settings,
    edits: Vec<Edit>,
    wrapper: String,
    #[serde(default)]
    previous_wrapper: Option<String>,
}

pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn prefix() -> AppResult<PathBuf> {
    let executable = std::env::current_exe()?;
    Ok(executable
        .parent()
        .and_then(Path::parent)
        .ok_or("invalid executable path")?
        .to_owned())
}

pub fn settings() -> AppResult<Option<Settings>> {
    let path = prefix()?.join(SETUP_RECORD);
    Ok(read_json::<Setup>(&path)?.map(|setup| setup.settings))
}

pub(crate) fn profile_settings(prefix: &Path) -> AppResult<(String, Settings)> {
    let setup = read_json::<Setup>(&prefix.join(SETUP_RECORD))?
        .ok_or("installation is not configured; run make start from the checkout")?;
    Ok((setup.profile, setup.settings))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> AppResult<Option<T>> {
    Ok(read(path)?
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()?)
}

pub(crate) fn read(path: &Path) -> AppResult<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            Err(format!("expected regular file: {}", path.display()).into())
        }
        Ok(_) => Ok(Some(fs::read(path)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8], executable: bool) -> AppResult<()> {
    let parent = path.parent().ok_or("file has no parent")?;
    fs::create_dir_all(parent)?;
    let permissions = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => metadata.permissions(),
        Ok(_) => return Err(format!("expected regular file: {}", path.display()).into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::Permissions::from_mode(if executable { 0o755 } else { 0o600 })
        }
        Err(error) => return Err(error.into()),
    };
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.as_file().set_permissions(permissions)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

pub(crate) fn lock(prefix: &Path) -> AppResult<File> {
    fs::create_dir_all(prefix)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(prefix.join(".codeloops-install.lock"))?;
    file.lock()?;
    Ok(file)
}

fn config_locks(edits: &[Edit]) -> AppResult<Vec<File>> {
    let mut parents = std::collections::BTreeSet::new();
    for parent in edits.iter().filter_map(|edit| edit.file.parent()) {
        fs::create_dir_all(parent)?;
        parents.insert(fs::canonicalize(parent)?);
    }
    let mut locks = Vec::new();
    for parent in parents {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(parent.join(".codeloops-setup.lock"))?;
        file.lock()?;
        locks.push(file);
    }
    Ok(locks)
}

pub fn install(prefix: &Path) -> AppResult<Value> {
    let _lock = lock(prefix)?;
    let prefix = fs::canonicalize(prefix)?;
    let files = BTreeMap::from([
        (
            PathBuf::from("bin/codeloops"),
            fs::read(std::env::current_exe()?)?,
        ),
        (
            PathBuf::from(ADAPTER),
            include_bytes!("../../../adapters/opencode/history.ts").to_vec(),
        ),
        (
            PathBuf::from(CURSOR),
            include_bytes!("../../../adapters/cursor/hooks.example.json").to_vec(),
        ),
    ]);
    let old = read_json::<Installed>(&prefix.join(INSTALL_RECORD))?;
    for (path, bytes) in &files {
        if let Some(existing) = read(&prefix.join(path))? {
            let existing_hash = hash(&existing);
            let previous = old.as_ref().and_then(|old| old.assets.get(path));
            if previous != Some(&existing_hash) && existing != *bytes {
                return Err(
                    format!("unowned or modified install asset: {}", path.display()).into(),
                );
            }
        }
    }
    let installed = Installed {
        schema_version: 1,
        assets: files
            .iter()
            .map(|(path, bytes)| (path.clone(), hash(bytes)))
            .collect(),
    };
    // The pending record accepts either side of an interrupted binary upgrade.
    let pending_path = prefix.join("share/codeloops/install-pending.json");
    atomic_write(
        &pending_path,
        &serde_json::to_vec_pretty(&installed)?,
        false,
    )?;
    for (path, bytes) in files {
        atomic_write(
            &prefix.join(&path),
            &bytes,
            path == Path::new("bin/codeloops"),
        )?;
    }
    atomic_write(
        &prefix.join(INSTALL_RECORD),
        &serde_json::to_vec_pretty(&installed)?,
        false,
    )?;
    fs::remove_file(pending_path)?;
    Ok(json!({
        "installed": true,
        "prefix": prefix,
        "executable": prefix.join("bin/codeloops"),
    }))
}

fn home() -> AppResult<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is required".into())
}

struct OpenCodePaths {
    config: PathBuf,
    plugins: PathBuf,
}

fn opencode_paths(server_name: &str) -> AppResult<OpenCodePaths> {
    let root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or(home()?.join(".config"))
        .join("opencode");
    let json = root.join("opencode.json");
    let jsonc = root.join("opencode.jsonc");
    let preferred = if jsonc.exists() {
        jsonc.clone()
    } else {
        json.clone()
    };
    let mut config = None;
    let mut plugins = None;
    // OpenCode loads config.json, opencode.json, then opencode.jsonc. Plugin
    // arrays replace earlier arrays. Extend the highest-priority existing list
    // rather than creating a new list that would hide inherited plugins.
    for path in [jsonc, json, root.join("config.json")] {
        let Some(bytes) = read(&path)?.filter(|bytes| !bytes.is_empty()) else {
            continue;
        };
        let text = std::str::from_utf8(&bytes)?;
        if plugins.is_none() && config_edits::contains(text, &["plugin"])? {
            plugins = Some(path.clone());
        }
        // Keep existing ownership/conflict checks at the active MCP entry's
        // source rather than silently overriding it in a higher-priority file.
        if config.is_none() && config_edits::contains(text, &["mcp", server_name])? {
            config = Some(path);
        }
    }
    Ok(OpenCodePaths {
        plugins: plugins.unwrap_or_else(|| preferred.clone()),
        config: config.unwrap_or(preferred),
    })
}

fn absolute(path: &Path) -> AppResult<PathBuf> {
    let absolute = std::path::absolute(path)?;
    if absolute
        .components()
        .any(|part| part == std::path::Component::ParentDir)
    {
        return Err("configuration paths must not contain '..'".into());
    }
    Ok(absolute)
}

fn shell_quote(path: &str) -> String {
    format!("'{}'", path.replace('\'', "'\\''"))
}

fn edit(file: &Path, path: &[&str], value: Value, array: bool) -> Edit {
    Edit {
        file: file.into(),
        path: path.iter().map(|s| (*s).into()).collect(),
        value,
        array,
    }
}

fn plan(config: &Config, args: SetupArgs, prefix: &Path) -> AppResult<Setup> {
    if args.profile.is_empty()
        || args.profile.len() > 40
        || !args
            .profile
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err("profile must be 1..40 lowercase letters, digits or hyphens".into());
    }
    let name = format!("codeloops-history-{}", args.profile);
    let opencode = match args.opencode_config {
        Some(path) => OpenCodePaths {
            config: path.clone(),
            plugins: path,
        },
        None => opencode_paths(&name)?,
    };
    let opencode_config = absolute(&opencode.config)?;
    let opencode_plugins = absolute(&opencode.plugins)?;
    let cursor = absolute(&args.cursor_config_dir.unwrap_or(home()?.join(".cursor")))?;
    let binary = prefix.join("bin/codeloops");
    let root = fs::canonicalize(&config.root)?;
    let command = json!([
        binary,
        "--data-dir",
        root,
        "--address",
        config.address.to_string(),
        "mcp",
    ]);
    let url =
        reqwest::Url::from_file_path(prefix.join(WRAPPER)).map_err(|_| "invalid plugin path")?;
    let mut edits = vec![
        edit(&opencode_plugins, &["plugin"], json!(url.as_str()), true),
        edit(
            &opencode_config,
            &["mcp", &name],
            json!({
                "type": "local",
                "command": command,
                "enabled": true,
            }),
            false,
        ),
        edit(
            &cursor.join("mcp.json"),
            &["mcpServers", &name],
            json!({
                "command": binary,
                "args": [
                    "--data-dir", root,
                    "--address", config.address.to_string(),
                    "mcp",
                ],
            }),
            false,
        ),
    ];
    let hooks: Value =
        serde_json::from_str(include_str!("../../../adapters/cursor/hooks.example.json"))?;
    let hook_command = format!(
        "{} --data-dir {} --address {} capture-cursor",
        shell_quote(binary.to_str().ok_or("executable path must be UTF-8")?),
        shell_quote(root.to_str().ok_or("data path must be UTF-8")?),
        shell_quote(&config.address.to_string()),
    );
    for hook in hooks["hooks"]
        .as_object()
        .ok_or("invalid shipped hook template")?
        .keys()
    {
        edits.push(edit(
            &cursor.join("hooks.json"),
            &["hooks", hook],
            json!({
                "command": hook_command,
                "timeout": 60,
            }),
            true,
        ));
    }
    let version = args.opencode_version.unwrap_or_else(|| {
        std::process::Command::new("opencode")
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .unwrap_or_else(|| "unknown".into())
    });
    let options = serde_json::to_string_pretty(&json!({
        "executable": binary,
        "dataDir": root,
        "address": config.address.to_string(),
        "sourceVersion": version,
    }))?;
    let wrapper = format!(
        "import history from './adapters/opencode/history.ts';\n\n\
         export default (input) => history(input, {options});\n"
    );
    Ok(Setup {
        schema_version: 1,
        profile: args.profile,
        settings: Settings {
            root,
            address: config.address,
        },
        edits,
        wrapper,
        previous_wrapper: None,
    })
}

type Documents = BTreeMap<PathBuf, (Vec<u8>, Vec<u8>)>;

fn documents(edits: &[Edit], removing: bool, owned: bool) -> AppResult<Documents> {
    let mut documents = BTreeMap::new();
    for edit in edits {
        let (before, after) = documents
            .entry(edit.file.clone())
            .or_insert_with(|| (Vec::new(), Vec::new()));
        if after.is_empty() {
            *before = read(&edit.file)?.unwrap_or_default();
            *after = if before.is_empty() {
                if removing {
                    continue;
                }
                let initial = if edit
                    .file
                    .file_name()
                    .is_some_and(|name| name == "hooks.json")
                {
                    "{\n  \"version\": 1\n}\n"
                } else if edit.path.first().is_some_and(|name| name == "plugin") {
                    "{\n  \"$schema\": \"https://opencode.ai/config.json\"\n}\n"
                } else {
                    "{\n}\n"
                };
                initial.as_bytes().to_vec()
            } else {
                before.clone()
            };
            if !removing {
                let opencode = edit.path.first().is_some_and(|name| name == "plugin");
                let hooks = edit
                    .file
                    .file_name()
                    .is_some_and(|name| name == "hooks.json");
                *after = config_edits::defaults(std::str::from_utf8(after)?, opencode, hooks)?
                    .into_bytes();
            }
        }
        *after =
            config_edits::apply(std::str::from_utf8(after)?, edit, removing, owned)?.into_bytes();
    }
    Ok(documents)
}

fn write_documents(documents: Documents) -> AppResult<()> {
    for (path, (before, after)) in documents {
        if before == after {
            continue;
        }
        if read(&path)?.unwrap_or_default() != before {
            return Err(format!(
                "configuration changed during setup; retry: {}",
                path.display()
            )
            .into());
        }
        atomic_write(&path, &after, false)?;
    }
    Ok(())
}

pub fn setup(config: &Config, args: SetupArgs) -> AppResult<Value> {
    let prefix = prefix()?;
    let _lock = lock(&prefix)?;
    if read_json::<Installed>(&prefix.join(INSTALL_RECORD))?.is_none() {
        return Err("run make install before setup".into());
    }
    let mut planned = plan(config, args, &prefix)?;
    let record_path = prefix.join(SETUP_RECORD);
    let existing = read_json::<Setup>(&record_path)?;
    let _config_locks = config_locks(&planned.edits)?;
    if let Some(existing) = &existing
        && (existing.profile != planned.profile
            || existing.settings != planned.settings
            || existing.edits != planned.edits)
    {
        return Err("installation already configured differently; \
            uninstall before changing profile or paths"
            .into());
    }
    if let Some(wrapper) = read(&prefix.join(WRAPPER))?
        && existing.as_ref().is_none_or(|old| {
            old.wrapper.as_bytes() != wrapper
                && old.previous_wrapper.as_deref().map(str::as_bytes) != Some(wrapper.as_slice())
        })
    {
        return Err("configured adapter was modified or is unowned".into());
    }
    let documents = documents(&planned.edits, false, existing.is_some())?;
    planned.previous_wrapper = existing.as_ref().map(|old| old.wrapper.clone());
    // Durable intent precedes any client changes. Repeating setup finishes this
    // same plan; uninstall can remove partially registered entries after a crash.
    atomic_write(&record_path, &serde_json::to_vec_pretty(&planned)?, false)?;
    atomic_write(&prefix.join(WRAPPER), planned.wrapper.as_bytes(), false)?;
    write_documents(documents)?;
    planned.previous_wrapper = None;
    atomic_write(&record_path, &serde_json::to_vec_pretty(&planned)?, false)?;
    Ok(json!({
        "configured": true,
        "scope": "user_global",
        "profile": planned.profile,
        "executable": prefix.join("bin/codeloops"),
        "data_dir": planned.settings.root,
        "address": planned.settings.address,
        "configuration_files": planned.edits.iter().map(|edit| &edit.file).collect::<std::collections::BTreeSet<_>>(),
        "reload": "Quit and restart OpenCode; open a new Cursor Agent Chat and check MCP connection.",
    }))
}

pub async fn uninstall(selected: Option<&Path>) -> AppResult<Value> {
    let prefix = match selected {
        Some(path) => absolute(path)?,
        None => prefix()?,
    };
    if !prefix.exists() {
        return Ok(json!({
            "uninstalled": true,
            "already_absent": true,
        }));
    }
    let _lock = lock(&prefix)?;
    crate::service::remove(&prefix).await?;
    let installed = read_json::<Installed>(&prefix.join(INSTALL_RECORD))?;
    let pending = read_json::<Installed>(&prefix.join("share/codeloops/install-pending.json"))?;
    let setup = read_json::<Setup>(&prefix.join(SETUP_RECORD))?;
    if let Some(setup) = &setup {
        let _config_locks = config_locks(&setup.edits)?;
        if let Some(bytes) = read(&prefix.join(WRAPPER))?
            && bytes != setup.wrapper.as_bytes()
            && setup.previous_wrapper.as_deref().map(str::as_bytes) != Some(bytes.as_slice())
        {
            return Err("configured adapter was modified; preserve it before uninstalling".into());
        }
        write_documents(documents(&setup.edits, true, true)?)?;
    }
    let mut preserved = Vec::new();
    let installed = installed.or_else(|| {
        pending.as_ref().map(|pending| Installed {
            schema_version: pending.schema_version,
            assets: pending.assets.clone(),
        })
    });
    if let Some(installed) = installed {
        for (path, expected) in installed.assets {
            // Never interpret arbitrary manifest paths as recursive removal roots.
            if !["bin/codeloops", ADAPTER, CURSOR].contains(&path.to_str().unwrap_or("")) {
                return Err("invalid owned asset path".into());
            }
            let installed_path = prefix.join(&path);
            if let Some(bytes) = read(&installed_path)? {
                let actual = hash(&bytes);
                let pending_hash = pending
                    .as_ref()
                    .and_then(|pending| pending.assets.get(&path));
                if actual == expected || pending_hash == Some(&actual) {
                    fs::remove_file(&installed_path)?;
                } else {
                    preserved.push(installed_path);
                }
            }
        }
    }
    for relative in [
        WRAPPER,
        SETUP_RECORD,
        INSTALL_RECORD,
        "share/codeloops/install-pending.json",
    ] {
        if relative == WRAPPER && setup.is_none() {
            continue;
        }
        let path = prefix.join(relative);
        if read(&path)?.is_some() {
            fs::remove_file(path)?;
        }
    }
    Ok(json!({
        "uninstalled": true,
        "data_preserved": setup.map(|setup| setup.settings.root),
        "modified_assets_preserved": preserved,
        "reload": "Restart clients to unload the integration.",
    }))
}
