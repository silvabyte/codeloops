/// Filesystem watcher for detecting file changes during actor execution.
///
/// Uses `notify` for filesystem events and `ignore` for gitignore-aware filtering.
/// Sends classified file change events through a tokio mpsc channel so they
/// can be consumed from async tasks.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use codeloops_logging::FileChangeType;

/// A classified file change event.
#[derive(Debug, Clone)]
pub struct WatcherEvent {
    pub path: PathBuf,
    pub change_type: FileChangeType,
    /// Path relative to the working directory for display
    pub relative_path: String,
}

/// Handle to the file watcher. Dropping stops watching.
pub struct FileWatcherHandle {
    _watcher: RecommendedWatcher,
}

/// Start watching a directory for file changes.
///
/// Returns `(handle, receiver)` — the handle keeps the watcher alive,
/// and the receiver yields events. Drop the handle to stop watching.
///
/// Returns `None` if the watcher fails to start (non-fatal).
pub fn start_watching(
    working_dir: &Path,
) -> Option<(FileWatcherHandle, mpsc::UnboundedReceiver<WatcherEvent>)> {
    let gitignore = build_gitignore(working_dir);
    let working_dir_owned = working_dir.to_path_buf();
    let (tx, rx) = mpsc::unbounded_channel();

    // Debounce state: track last-seen time per file
    let debounce = std::sync::Arc::new(std::sync::Mutex::new(
        HashMap::<PathBuf, Instant>::new(),
    ));
    let debounce_duration = Duration::from_millis(200);

    let debounce_clone = debounce.clone();
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            for path in &event.paths {
                // Skip .git directory
                if path.components().any(|c| c.as_os_str() == ".git") {
                    continue;
                }

                // Skip if gitignored (use matched_path_or_any_parents to handle
                // files inside ignored directories like target/)
                let is_dir = path.is_dir();
                if let Some(ref gi) = gitignore {
                    if gi.matched_path_or_any_parents(path, is_dir).is_ignore() {
                        continue;
                    }
                }

                // Skip directories themselves
                if is_dir {
                    continue;
                }

                // Debounce: skip if we saw this file recently
                if let Ok(mut map) = debounce_clone.lock() {
                    let now = Instant::now();
                    if let Some(last) = map.get(path) {
                        if now.duration_since(*last) < debounce_duration {
                            continue;
                        }
                    }
                    map.insert(path.clone(), now);
                }

                // Classify the event
                let change_type = match event.kind {
                    EventKind::Create(_) => FileChangeType::Created,
                    EventKind::Modify(_) => FileChangeType::Modified,
                    EventKind::Remove(_) => FileChangeType::Deleted,
                    _ => continue,
                };

                // Compute relative path for display
                let relative = path
                    .strip_prefix(&working_dir_owned)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();

                let _ = tx.send(WatcherEvent {
                    path: path.clone(),
                    change_type,
                    relative_path: relative,
                });
            }
        }
    })
    .ok()?;

    watcher
        .watch(working_dir, RecursiveMode::Recursive)
        .ok()?;

    Some((FileWatcherHandle { _watcher: watcher }, rx))
}

/// Build a gitignore matcher from the working directory.
fn build_gitignore(working_dir: &Path) -> Option<Gitignore> {
    let mut builder = GitignoreBuilder::new(working_dir);

    // Walk up directory tree to find gitignore files
    let mut dir = Some(working_dir);
    while let Some(d) = dir {
        let gitignore_path = d.join(".gitignore");
        if gitignore_path.exists() {
            let _ = builder.add(&gitignore_path);
        }
        dir = d.parent();
    }

    builder.build().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn build_gitignore_from_dir() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".gitignore"), "*.log\ntarget/\n").unwrap();

        let gi = build_gitignore(tmp.path()).unwrap();
        // matched() expects paths relative to the gitignore root
        assert!(gi.matched(tmp.path().join("debug.log"), false).is_ignore());
        // target/ pattern matches the directory itself
        assert!(gi.matched(tmp.path().join("target"), true).is_ignore());
        // Files inside target are matched via matched_path_or_any_parents
        assert!(gi.matched_path_or_any_parents(tmp.path().join("target/release/binary"), false).is_ignore());
        assert!(!gi.matched(tmp.path().join("main.rs"), false).is_ignore());
    }
}
