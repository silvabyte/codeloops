use base64::{Engine, engine::general_purpose::STANDARD as B64};
use serde_json::Value;
use session_history::{
    Error, History,
    model::{Layer, Page, Query},
};
use std::{
    ffi::OsString,
    fs,
    os::unix::{
        ffi::OsStringExt,
        fs::{PermissionsExt, symlink},
    },
    path::Path,
    process::Command,
};
use uuid::Uuid;

fn git_command(root: &Path) -> Command {
    let mut command = Command::new("git");
    // Strip hook-exported Git context before creating or mutating test repos.
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    command
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.com");
    command
}

fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = git_command(root).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}
fn id(v: &Value) -> String {
    v["id"].as_str().unwrap().into()
}
fn read(h: &History, checkpoint: &Value, layer: Layer, path: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let v = h
            .query(Query::File {
                checkpoint_id: id(checkpoint),
                path: B64.encode(path),
                layer: layer.clone(),
                offset: bytes.len(),
                limit: 3,
            })
            .unwrap();
        bytes.extend(B64.decode(v["data"].as_str().unwrap()).unwrap());
        if v["next_offset"].is_null() {
            return bytes;
        }
    }
}
fn compare(h: &History, a: &Value, b: &Value) -> Value {
    h.query(Query::Compare {
        before: id(a),
        after: id(b),
        before_layer: Layer::Worktree,
        after_layer: Layer::Worktree,
        page: Page::default(),
    })
    .unwrap()
}

#[test]
fn dirty_staged_binary_symlink_and_path_bytes_survive_repository_removal() {
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    git(root, &["init", "-q"]);
    fs::create_dir(root.join("stable")).unwrap();
    fs::write(root.join("stable/keep"), b"unchanged\n").unwrap();
    fs::write(root.join("dirty"), b"HEAD\n").unwrap();
    fs::write(root.join("delete"), b"keep deletion bytes").unwrap();
    fs::write(root.join(".gitignore"), b"ignored\ntracked\n").unwrap();
    fs::write(root.join("tracked"), b"tracked despite ignore").unwrap();
    fs::write(root.join("binary"), [0, 255, 4]).unwrap();
    symlink("missing target", root.join("link")).unwrap();
    let raw_path = b"raw-\xff";
    fs::write(
        root.join(OsString::from_vec(raw_path.to_vec())),
        b"raw name",
    )
    .unwrap();
    git(root, &["add", "."]);
    git(root, &["add", "-f", "tracked"]);
    git(root, &["commit", "-qm", "baseline"]);
    git(root, &["checkout", "--detach", "-q"]);
    fs::write(root.join("dirty"), b"INDEX\n").unwrap();
    git(root, &["add", "dirty"]);
    fs::write(root.join("dirty"), b"WORKTREE\n").unwrap();
    fs::write(root.join("ignored"), b"excluded").unwrap();
    fs::write(root.join("new"), b"untracked before tool").unwrap();
    let index_before = fs::read(root.join(".git/index")).unwrap();
    let status_before = git(root, &["status", "--porcelain=v1", "-z"]);
    let archive = tempfile::tempdir().unwrap();
    let workspace = Uuid::new_v4().to_string();
    let mut h = History::open(archive.path()).unwrap();
    let before = h.checkpoint(root, &workspace).unwrap();
    assert_eq!(before["status"], "complete");
    assert!(before["branch"].is_null());
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index_before);
    assert_eq!(
        git(root, &["status", "--porcelain=v1", "-z"]),
        status_before
    );
    let artifact_count = fs::read_dir(archive.path().join("artifacts"))
        .unwrap()
        .count();
    let unchanged = h.checkpoint(root, &workspace).unwrap();
    assert_eq!(before["manifests"], unchanged["manifests"]);
    assert_eq!(
        fs::read_dir(archive.path().join("artifacts"))
            .unwrap()
            .count(),
        artifact_count
    );
    fs::write(root.join("dirty"), b"AFTER\n").unwrap();
    fs::write(root.join("binary"), [0, 254, 5, 255]).unwrap();
    fs::remove_file(root.join("delete")).unwrap();
    fs::remove_file(root.join("link")).unwrap();
    symlink("new target", root.join("link")).unwrap();
    fs::write(root.join("created"), b"new file\n").unwrap();
    fs::set_permissions(root.join("created"), fs::Permissions::from_mode(0o755)).unwrap();
    let after = h.checkpoint(root, &workspace).unwrap();
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index_before);
    drop(h);
    repo.close().unwrap();
    let h = History::open(archive.path()).unwrap();
    assert_eq!(read(&h, &before, Layer::Head, b"dirty"), b"HEAD\n");
    assert_eq!(read(&h, &before, Layer::Index, b"dirty"), b"INDEX\n");
    assert_eq!(read(&h, &before, Layer::Worktree, b"dirty"), b"WORKTREE\n");
    assert_eq!(read(&h, &after, Layer::Worktree, b"dirty"), b"AFTER\n");
    assert_eq!(
        read(&h, &before, Layer::Worktree, b"delete"),
        b"keep deletion bytes"
    );
    assert_eq!(read(&h, &before, Layer::Worktree, b"binary"), [0, 255, 4]);
    assert_eq!(
        read(&h, &after, Layer::Worktree, b"binary"),
        [0, 254, 5, 255]
    );
    assert_eq!(
        read(&h, &before, Layer::Worktree, b"link"),
        b"missing target"
    );
    assert_eq!(read(&h, &after, Layer::Worktree, b"link"), b"new target");
    assert_eq!(read(&h, &before, Layer::Worktree, raw_path), b"raw name");
    assert_eq!(
        read(&h, &before, Layer::Worktree, b"tracked"),
        b"tracked despite ignore"
    );
    assert!(matches!(
        h.query(Query::File {
            checkpoint_id: id(&before),
            path: B64.encode(b"ignored"),
            layer: Layer::Worktree,
            offset: 0,
            limit: 10
        }),
        Err(Error::Unknown(_))
    ));
    let changes = compare(&h, &before, &after);
    assert_eq!(changes["items"].as_array().unwrap().len(), 5);
    assert_eq!(changes["authorship"], "not_attributed");
    let item = changes["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["display_path"] == "dirty")
        .unwrap();
    let patch = h
        .query(Query::Artifact {
            hash: item["patch"]["hash"].as_str().unwrap().into(),
            offset: 0,
            limit: 65536,
        })
        .unwrap();
    let patch = String::from_utf8(B64.decode(patch["data"].as_str().unwrap()).unwrap()).unwrap();
    assert!(patch.contains("-WORKTREE\n+AFTER\n"));
    let first = h
        .query(Query::Compare {
            before: id(&before),
            after: id(&after),
            before_layer: Layer::Worktree,
            after_layer: Layer::Worktree,
            page: Page {
                limit: 1,
                cursor: None,
            },
        })
        .unwrap();
    assert_eq!(first["items"].as_array().unwrap().len(), 1);
    assert_eq!(first["has_more"], true);
    assert!(
        h.query(Query::Compare {
            before: id(&after),
            after: id(&before),
            before_layer: Layer::Worktree,
            after_layer: Layer::Worktree,
            page: Page {
                limit: 1,
                cursor: Some(first["next_cursor"].as_str().unwrap().into())
            }
        })
        .is_err()
    );
}

#[test]
fn unborn_conflicted_lfs_submodules_and_publication_failure_are_explicit() {
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    git(root, &["init", "-q"]);
    fs::write(root.join("file"), b"unborn").unwrap();
    let archive = tempfile::tempdir().unwrap();
    let workspace = Uuid::new_v4().to_string();
    let mut h = History::open(archive.path()).unwrap();
    let unborn = h.checkpoint(root, &workspace).unwrap();
    assert!(unborn["head"].is_null());
    assert_eq!(unborn["status"], "complete");
    git(root, &["add", "file"]);
    git(root, &["commit", "-qm", "base"]);
    let branch = String::from_utf8(git(root, &["branch", "--show-current"])).unwrap();
    git(root, &["checkout", "-qb", "other"]);
    fs::write(root.join("file"), b"other\n").unwrap();
    git(root, &["commit", "-qam", "other"]);
    git(root, &["checkout", "-q", branch.trim()]);
    fs::write(root.join("file"), b"main\n").unwrap();
    git(root, &["commit", "-qam", "main"]);
    let output = git_command(root)
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.com",
            "merge",
            "other",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    fs::write(
        root.join("pointer"),
        b"version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 10\n",
    )
    .unwrap();
    let commit = String::from_utf8(git(root, &["rev-parse", "HEAD"])).unwrap();
    git(
        root,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{},submodule", commit.trim()),
        ],
    );
    let index = fs::read(root.join(".git/index")).unwrap();
    let partial = h.checkpoint(root, &workspace).unwrap();
    assert_eq!(partial["status"], "partial");
    for reason in [
        "conflicted_index",
        "lfs_pointer_only",
        "submodule_not_captured",
    ] {
        assert!(
            partial["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == reason)
        );
    }
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    let file = h
        .query(Query::File {
            checkpoint_id: id(&partial),
            path: B64.encode("file"),
            layer: Layer::Index,
            offset: 0,
            limit: 100,
        })
        .unwrap();
    assert_eq!(file["file"]["stages"].as_object().unwrap().len(), 3);
    let count = rusqlite::Connection::open(archive.path().join("history.sqlite3"))
        .unwrap()
        .query_row("SELECT COUNT(*) FROM checkpoints", [], |r| {
            r.get::<_, u64>(0)
        })
        .unwrap();
    fs::rename(
        archive.path().join("artifacts"),
        archive.path().join("saved-artifacts"),
    )
    .unwrap();
    fs::write(archive.path().join("artifacts"), b"blocked").unwrap();
    assert!(h.checkpoint(root, &workspace).is_err());
    assert_eq!(
        rusqlite::Connection::open(archive.path().join("history.sqlite3"))
            .unwrap()
            .query_row("SELECT COUNT(*) FROM checkpoints", [], |r| r
                .get::<_, u64>(0))
            .unwrap(),
        count
    );
}

#[test]
fn concurrent_writer_marks_the_observation_unstable() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let repo = tempfile::tempdir().unwrap();
    git(repo.path(), &["init", "-q"]);
    fs::write(repo.path().join("changing"), b"initial").unwrap();
    let archive = tempfile::tempdir().unwrap();
    let mut h = History::open(archive.path()).unwrap();
    let stop = AtomicBool::new(false);
    let result = std::thread::scope(|scope| {
        scope.spawn(|| {
            let mut n = 0_u64;
            while !stop.load(Ordering::Relaxed) {
                n += 1;
                fs::write(repo.path().join(".git/changing-temp"), n.to_le_bytes()).unwrap();
                fs::rename(
                    repo.path().join(".git/changing-temp"),
                    repo.path().join("changing"),
                )
                .unwrap();
            }
        });
        let result = h.checkpoint(repo.path(), &Uuid::new_v4().to_string());
        stop.store(true, Ordering::Relaxed);
        result
    })
    .unwrap();
    assert_eq!(result["status"], "unstable");
    assert_eq!(result["coverage"]["atomic"], false);
}
