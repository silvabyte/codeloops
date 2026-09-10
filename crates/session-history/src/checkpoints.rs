//! Read-only Git observation and self-contained, structurally shared manifests.
use crate::{
    Error, History, Result,
    artifacts::Artifacts,
    model::{Layer, MAX_PAGE_SIZE, Page},
    now_ms,
};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    fs,
    io::Read,
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::PermissionsExt,
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use uuid::Uuid;

const MAX_FILE: usize = 8 * 1024 * 1024;
const MAX_LIST: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct File {
    mode: String,
    hash: Option<String>,
    size: usize,
    coverage: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    stages: BTreeMap<String, Box<File>>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Node {
    File(File),
    Directory { hash: String },
}
type Manifest = BTreeMap<String, Node>;
type Files = BTreeMap<Vec<u8>, File>;
type ObjectCache = BTreeMap<(String, String), File>;

fn git(root: &Path, args: &[&str], limit: usize) -> Result<Vec<u8>> {
    // No optional index refresh, replacement objects, prompts or lazy fetching.
    let mut child = Command::new("git")
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut bytes = Vec::new();
    let read = child
        .stdout
        .take()
        .ok_or_else(|| Error::Invalid("missing Git stdout".into()))?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes);
    if read.is_err() || bytes.len() > limit {
        let _ = child.kill();
        let _ = child.wait();
        read?;
        return Err(Error::Invalid("Git output exceeds capture limit".into()));
    }
    if !child.wait()?.success() {
        return Err(Error::Invalid(format!(
            "Git observation failed: {}",
            args[0]
        )));
    }
    Ok(bytes)
}

fn text(bytes: &[u8]) -> Result<&str> {
    std::str::from_utf8(bytes).map_err(|_| Error::Invalid("invalid Git metadata".into()))
}

fn unavailable(mode: &str, reason: &str) -> File {
    File {
        mode: mode.into(),
        hash: None,
        size: 0,
        coverage: reason.into(),
        stages: BTreeMap::new(),
    }
}

fn bytes(a: &Artifacts, mode: &str, data: &[u8]) -> Result<File> {
    if data.len() > MAX_FILE {
        return Ok(unavailable(mode, "file_size_limit_8_mib"));
    }
    Ok(File {
        mode: mode.into(),
        hash: Some(a.put(data)?),
        size: data.len(),
        coverage: if data.starts_with(b"version https://git-lfs.github.com/spec/v1\n") {
            "lfs_pointer_only"
        } else {
            "complete"
        }
        .into(),
        stages: BTreeMap::new(),
    })
}

fn object(
    root: &Path,
    a: &Artifacts,
    mode: &str,
    oid: &str,
    cache: &mut ObjectCache,
) -> Result<File> {
    let key = (mode.into(), oid.into());
    if let Some(file) = cache.get(&key) {
        return Ok(file.clone());
    }
    if mode == "160000" {
        return Ok(unavailable(mode, "submodule_not_captured"));
    }
    let size = git(root, &["cat-file", "-s", oid], 128)?;
    if text(&size)?.trim().parse::<usize>().unwrap_or(usize::MAX) > MAX_FILE {
        return Ok(unavailable(mode, "file_size_limit_8_mib"));
    }
    let file = bytes(a, mode, &git(root, &["cat-file", "blob", oid], MAX_FILE)?)?;
    cache.insert(key, file.clone());
    Ok(file)
}

fn valid_path(path: &[u8]) -> Result<()> {
    if path.is_empty()
        || path.split(|b| *b == b'/').count() > 256
        || path
            .split(|b| *b == b'/')
            .any(|p| p.is_empty() || p == b"." || p == b".." || p == b".git")
        || path.contains(&0)
    {
        return Err(Error::Invalid("invalid repository-relative path".into()));
    }
    Ok(())
}

fn work_file(root: &Path, path: &[u8], a: &Artifacts) -> Result<Option<File>> {
    valid_path(path)?;
    use rustix::fs::{AtFlags, FileType, Mode, OFlags, open, openat, readlinkat, statat};
    // Directory handles plus NOFOLLOW protect every component, including when
    // a concurrent writer replaces an ancestor with a symlink between syscalls.
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut parent = open(root, flags, Mode::empty()).map_err(std::io::Error::from)?;
    let parts: Vec<_> = path.split(|b| *b == b'/').collect();
    for part in &parts[..parts.len() - 1] {
        parent = match openat(&parent, OsStr::from_bytes(part), flags, Mode::empty()) {
            Ok(fd) => fd,
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(_) => return Ok(Some(unavailable("unknown", "ancestor_not_directory"))),
        };
    }
    let name = OsStr::from_bytes(parts[parts.len() - 1]);
    let stat = match statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) => stat,
        Err(rustix::io::Errno::NOENT) => return Ok(None),
        Err(_) => return Ok(Some(unavailable("unknown", "unreadable"))),
    };
    if FileType::from_raw_mode(stat.st_mode) == FileType::Symlink {
        return match readlinkat(&parent, name, Vec::new()) {
            Ok(target) => Ok(Some(bytes(a, "120000", target.to_bytes())?)),
            Err(_) => Ok(Some(unavailable("120000", "unreadable_or_changed"))),
        };
    }
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
        return Ok(Some(unavailable("unknown", "unsupported_file_kind")));
    }
    let fd = match openat(
        &parent,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(_) => return Ok(Some(unavailable("unknown", "unreadable_or_changed"))),
    };
    let file = fs::File::from(fd);
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Ok(Some(unavailable("unknown", "changed_file_kind")));
    }
    let mode = if meta.permissions().mode() & 0o111 != 0 {
        "100755"
    } else {
        "100644"
    };
    if meta.len() > MAX_FILE as u64 {
        return Ok(Some(unavailable(mode, "file_size_limit_8_mib")));
    }
    let mut data = Vec::new();
    file.take(MAX_FILE as u64 + 1).read_to_end(&mut data)?;
    Ok(Some(bytes(a, mode, &data)?))
}

#[derive(PartialEq, Eq)]
struct Scan {
    head: Option<String>,
    branch: Option<String>,
    index_raw: Vec<u8>,
    head_files: Files,
    index_files: Files,
    work_files: Files,
}

fn scan(root: &Path, a: &Artifacts, cache: &mut ObjectCache) -> Result<Scan> {
    // Only an unresolved HEAD in a symbolic branch is an unborn repository.
    let branch = git(root, &["symbolic-ref", "-q", "HEAD"], 4096)
        .ok()
        .map(|v| text(&v).map(|s| s.trim().to_string()))
        .transpose()?;
    let head = match git(root, &["rev-parse", "--verify", "HEAD"], 128) {
        Ok(v) => Some(text(&v)?.trim().to_string()),
        Err(_) if branch.is_some() => None,
        Err(e) => return Err(e),
    };
    let mut head_files = Files::new();
    if let Some(head) = &head {
        for record in git(root, &["ls-tree", "-r", "-z", head], MAX_LIST)?
            .split(|b| *b == 0)
            .filter(|p| !p.is_empty())
        {
            let tab = record
                .iter()
                .position(|b| *b == b'\t')
                .ok_or_else(|| Error::Invalid("invalid tree record".into()))?;
            let fields: Vec<_> = text(&record[..tab])?.split(' ').collect();
            if fields.len() != 3 {
                return Err(Error::Invalid("invalid tree fields".into()));
            }
            valid_path(&record[tab + 1..])?;
            head_files.insert(
                record[tab + 1..].to_vec(),
                object(root, a, fields[0], fields[2], cache)?,
            );
        }
    }
    let index_raw = git(root, &["ls-files", "--stage", "-z"], MAX_LIST)?;
    let mut index_files = Files::new();
    for record in index_raw.split(|b| *b == 0).filter(|p| !p.is_empty()) {
        let tab = record
            .iter()
            .position(|b| *b == b'\t')
            .ok_or_else(|| Error::Invalid("invalid index record".into()))?;
        let fields: Vec<_> = text(&record[..tab])?.split(' ').collect();
        if fields.len() != 3 {
            return Err(Error::Invalid("invalid index fields".into()));
        }
        let path = record[tab + 1..].to_vec();
        valid_path(&path)?;
        let file = object(root, a, fields[0], fields[1], cache)?;
        if fields[2] == "0" {
            index_files.insert(path, file);
        } else {
            index_files
                .entry(path)
                .or_insert_with(|| unavailable(fields[0], "conflicted_index"))
                .stages
                .insert(fields[2].into(), Box::new(file));
        }
    }
    let mut paths: BTreeSet<Vec<u8>> = index_files.keys().cloned().collect();
    paths.extend(
        git(
            root,
            &["ls-files", "--others", "--exclude-standard", "-z"],
            MAX_LIST,
        )?
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(Vec::from),
    );
    let mut work_files = Files::new();
    for path in paths {
        if index_files.get(&path).is_some_and(|f| f.mode == "160000") {
            work_files.insert(path, unavailable("160000", "submodule_not_captured"));
        } else if let Some(file) = work_file(root, &path, a)? {
            work_files.insert(path, file);
        }
    }
    Ok(Scan {
        head,
        branch,
        index_raw,
        head_files,
        index_files,
        work_files,
    })
}

fn manifest(a: &Artifacts, files: &Files) -> Result<String> {
    let mut node = Manifest::new();
    let mut dirs: BTreeMap<Vec<u8>, Files> = BTreeMap::new();
    for (path, file) in files {
        if let Some(slash) = path.iter().position(|b| *b == b'/') {
            dirs.entry(path[..slash].to_vec())
                .or_default()
                .insert(path[slash + 1..].to_vec(), file.clone());
        } else {
            node.insert(B64.encode(path), Node::File(file.clone()));
        }
    }
    for (name, children) in dirs {
        node.insert(
            B64.encode(name),
            Node::Directory {
                hash: manifest(a, &children)?,
            },
        );
    }
    let data = serde_json::to_vec(&node)?;
    if data.len() > MAX_FILE {
        return Err(Error::Invalid("directory manifest exceeds 8 MiB".into()));
    }
    a.put(&data)
}

fn flatten(a: &Artifacts, hash: &str, prefix: &[u8], out: &mut Files, depth: usize) -> Result<()> {
    if depth > 256 {
        return Err(Error::Invalid("manifest nesting limit".into()));
    }
    let node: Manifest = serde_json::from_slice(&a.get(hash)?)?;
    for (name, node) in node {
        let mut path = prefix.to_vec();
        path.extend(
            B64.decode(name)
                .map_err(|_| Error::Invalid("invalid manifest name".into()))?,
        );
        match node {
            Node::File(file) => {
                out.insert(path, file);
            }
            Node::Directory { hash } => {
                path.push(b'/');
                flatten(a, &hash, &path, out, depth + 1)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn capture(h: &mut History, directory: &Path, workspace: &str) -> Result<Value> {
    Uuid::parse_str(workspace).map_err(|_| Error::Invalid("workspace must be a UUID".into()))?;
    let started = now_ms();
    let root_bytes = git(directory, &["rev-parse", "--show-toplevel"], MAX_LIST)?;
    let root = PathBuf::from(OsString::from_vec(
        root_bytes
            .strip_suffix(b"\n")
            .unwrap_or(&root_bytes)
            .to_vec(),
    ));
    if h.connection
        .path()
        .is_some_and(|p| Path::new(p).starts_with(&root))
    {
        return Err(Error::Invalid(
            "archive must reside outside the observed Git worktree".into(),
        ));
    }
    let mut cache = ObjectCache::new();
    let first = scan(&root, &h.artifacts, &mut cache)?;
    let second = scan(&root, &h.artifacts, &mut cache)?;
    let unstable = first != second;
    let mut issues = BTreeSet::new();
    for files in [&first.head_files, &first.index_files, &first.work_files] {
        for file in files.values() {
            if file.coverage != "complete" {
                issues.insert(file.coverage.clone());
            }
        }
    }
    if unstable {
        issues.insert("changed_during_scan".into());
    }
    let record = json!({
        "id":Uuid::new_v4().to_string(), "workspace_id":workspace, "started_at":started, "finished_at":now_ms(),
        "status":if unstable { "unstable" } else if issues.is_empty() { "complete" } else { "partial" },
        "issues":issues, "head":first.head, "branch":first.branch,
        "manifests":{"head":manifest(&h.artifacts, &first.head_files)?, "index":manifest(&h.artifacts, &first.index_files)?, "worktree":manifest(&h.artifacts, &first.work_files)?},
        "coverage":{"atomic":false,"authorship":"not_attributed","intermediate_writes":"not_observed","untracked":"non_ignored","max_file_bytes":MAX_FILE}
    });
    h.connection.execute(
        "INSERT INTO checkpoints VALUES(?,?,?)",
        params![
            record["id"].as_str(),
            workspace,
            serde_json::to_string(&record)?
        ],
    )?;
    Ok(record)
}

pub(crate) fn get(h: &History, id: &str) -> Result<Value> {
    let record: Option<String> = h
        .connection
        .query_row("SELECT record FROM checkpoints WHERE id=?", [id], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(serde_json::from_str(
        &record.ok_or_else(|| Error::Unknown(id.into()))?,
    )?)
}

fn files(h: &History, checkpoint: &Value, layer: &Layer) -> Result<Files> {
    let key = match layer {
        Layer::Head => "head",
        Layer::Index => "index",
        Layer::Worktree => "worktree",
    };
    let hash = checkpoint["manifests"][key]
        .as_str()
        .ok_or_else(|| Error::Invalid("missing manifest".into()))?;
    let mut out = Files::new();
    flatten(&h.artifacts, hash, &[], &mut out, 0)?;
    Ok(out)
}

pub(crate) fn file(
    h: &History,
    id: &str,
    path: &str,
    layer: Layer,
    offset: usize,
    limit: usize,
) -> Result<Value> {
    if limit == 0 || limit > 65536 {
        return Err(Error::Invalid("file limit must be 1..65536".into()));
    }
    let raw = B64
        .decode(path)
        .map_err(|_| Error::Invalid("path must be base64".into()))?;
    valid_path(&raw)?;
    let checkpoint = get(h, id)?;
    let files = files(h, &checkpoint, &layer)?;
    let file = files
        .get(&raw)
        .ok_or_else(|| Error::Unknown(format!("file {path} in {id}")))?;
    let Some(hash) = &file.hash else {
        return Ok(json!({"file":file,"status":"unavailable","checkpoint":checkpoint}));
    };
    let mut chunk = crate::query::query(
        h,
        crate::model::Query::Artifact {
            hash: hash.clone(),
            offset,
            limit,
        },
    )?;
    chunk["file"] = serde_json::to_value(file)?;
    chunk["checkpoint_status"] = checkpoint["status"].clone();
    Ok(chunk)
}

fn patch(a: &Artifacts, before: Option<&File>, after: Option<&File>) -> Result<Value> {
    if before.into_iter().chain(after).any(|f| f.hash.is_none()) {
        return Ok(json!({"status":"unavailable"}));
    }
    let old = before
        .and_then(|f| f.hash.as_ref())
        .map(|h| a.get(h))
        .transpose()?
        .unwrap_or_default();
    let new = after
        .and_then(|f| f.hash.as_ref())
        .map(|h| a.get(h))
        .transpose()?
        .unwrap_or_default();
    let (Ok(old), Ok(new)) = (std::str::from_utf8(&old), std::str::from_utf8(&new)) else {
        return Ok(json!({"status":"binary"}));
    };
    if old.contains('\0') || new.contains('\0') {
        return Ok(json!({"status":"binary"}));
    }
    if old == new {
        return Ok(json!({"status":"metadata_only"}));
    }
    let count = |s: &str| s.split_inclusive('\n').count();
    let range = |n: usize| {
        if n == 0 {
            "0,0".into()
        } else {
            format!("1,{n}")
        }
    };
    let mut patch = format!(
        "--- before\n+++ after\n@@ -{} +{} @@\n",
        range(count(old)),
        range(count(new))
    );
    for (prefix, text) in [('-', old), ('+', new)] {
        for line in text.split_inclusive('\n') {
            patch.push(prefix);
            patch.push_str(line);
            if !line.ends_with('\n') {
                patch.push_str("\n\\ No newline at end of file\n");
            }
        }
    }
    if patch.len() > MAX_FILE {
        return Ok(json!({"status":"patch_size_limit","file_bytes_available":true}));
    }
    Ok(
        json!({"status":"available","hash":a.put(patch.as_bytes())?,"bytes":patch.len(),"format":"unified_full_file"}),
    )
}

pub(crate) fn compare(
    h: &History,
    before: &str,
    after: &str,
    before_layer: Layer,
    after_layer: Layer,
    page: Page,
) -> Result<Value> {
    if page.limit == 0 || page.limit > MAX_PAGE_SIZE {
        return Err(Error::Invalid("page limit must be 1..100".into()));
    }
    let first = get(h, before)?;
    let last = get(h, after)?;
    if first["workspace_id"] != last["workspace_id"] {
        return Err(Error::Invalid(
            "checkpoints belong to different workspaces".into(),
        ));
    }
    let fingerprint = crate::artifacts::hash(&serde_json::to_vec(&(
        before,
        after,
        &before_layer,
        &after_layer,
    ))?);
    let start = match page.cursor.as_deref() {
        None => 0,
        Some(s) => {
            let (hash, offset) = s
                .split_once(':')
                .ok_or_else(|| Error::Invalid("invalid comparison cursor".into()))?;
            if hash != fingerprint {
                return Err(Error::Invalid("comparison cursor scope mismatch".into()));
            }
            offset
                .parse::<usize>()
                .map_err(|_| Error::Invalid("invalid cursor offset".into()))?
        }
    };
    let old = files(h, &first, &before_layer)?;
    let new = files(h, &last, &after_layer)?;
    let paths: BTreeSet<_> = old.keys().chain(new.keys()).collect();
    let changed: Vec<_> = paths
        .into_iter()
        .filter(|p| old.get(*p) != new.get(*p))
        .collect();
    if start > changed.len() {
        return Err(Error::Invalid("comparison offset exceeds length".into()));
    }
    let end = start.saturating_add(page.limit).min(changed.len());
    let mut items = Vec::new();
    for path in &changed[start..end] {
        let before = old.get(*path);
        let after = new.get(*path);
        items.push(json!({"path":B64.encode(path),"path_encoding":"base64","display_path":String::from_utf8_lossy(path),"before":before,"after":after,"patch":patch(&h.artifacts,before,after)?}));
    }
    Ok(
        json!({"before":first,"after":last,"items":items,"has_more":end<changed.len(),"next_cursor":if end<changed.len(){Some(format!("{fingerprint}:{end}"))}else{None},"limit":page.limit,"authorship":"not_attributed"}),
    )
}
