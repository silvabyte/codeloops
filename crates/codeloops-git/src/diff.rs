use git2::{DiffOptions, Repository, StatusOptions};
use std::path::Path;
use thiserror::Error;
use tracing::debug;

use crate::GitStatus;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Not a git repository: {0}")]
    NotARepo(String),

    #[error("Git operation failed: {0}")]
    GitOperationFailed(#[from] git2::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("No commits in repository")]
    NoCommits,
}

/// Summary of diff statistics
#[derive(Debug, Clone, Default)]
pub struct DiffSummary {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

/// Utility for capturing git diffs
pub struct DiffCapture {
    /// Whether to include untracked files
    include_untracked: bool,
}

impl Default for DiffCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffCapture {
    pub fn new() -> Self {
        Self {
            include_untracked: true,
        }
    }

    pub fn with_untracked(mut self, include: bool) -> Self {
        self.include_untracked = include;
        self
    }

    /// Capture current working directory status
    pub fn capture_status(&self, working_dir: &Path) -> Result<GitStatus, GitError> {
        let repo = Repository::discover(working_dir)?;

        let mut opts = StatusOptions::new();
        opts.include_untracked(self.include_untracked)
            .recurse_untracked_dirs(true);

        let statuses = repo.statuses(Some(&mut opts))?;

        let mut status = GitStatus::default();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let st = entry.status();

            if st.is_wt_modified() || st.is_index_modified() {
                status.modified.push(path.clone());
            }
            if st.is_wt_new() {
                status.untracked.push(path.clone());
            }
            if st.is_index_new() {
                status.added.push(path.clone());
            }
            if st.is_wt_deleted() || st.is_index_deleted() {
                status.deleted.push(path);
            }
        }

        debug!(
            modified = status.modified.len(),
            added = status.added.len(),
            deleted = status.deleted.len(),
            untracked = status.untracked.len(),
            "Captured git status"
        );

        Ok(status)
    }

    /// Capture the full diff of working directory changes
    pub fn capture_diff(&self, working_dir: &Path) -> Result<String, GitError> {
        let repo = Repository::discover(working_dir)?;

        // Try to get HEAD tree, handle empty repo case
        let head_tree = match repo.head() {
            Ok(head) => Some(head.peel_to_tree()?),
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => None,
            Err(e) => return Err(GitError::GitOperationFailed(e)),
        };

        let mut opts = DiffOptions::new();
        opts.include_untracked(self.include_untracked)
            .recurse_untracked_dirs(true);

        let diff = repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut opts))?;

        let mut diff_text = String::new();

        diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
            // Include file headers
            if let Some(path) = delta.new_file().path() {
                if diff_text.is_empty() || !diff_text.ends_with(&format!("{}\n", path.display())) {
                    // File header is already included by git2
                }
            }

            let prefix = match line.origin() {
                '+' => "+",
                '-' => "-",
                ' ' => " ",
                'H' => "", // Hunk header
                'F' => "", // File header
                _ => "",
            };

            if !prefix.is_empty() {
                diff_text.push_str(prefix);
            }

            if let Ok(content) = std::str::from_utf8(line.content()) {
                diff_text.push_str(content);
            }

            true
        })?;

        debug!(diff_len = diff_text.len(), "Captured git diff");

        Ok(diff_text)
    }

    /// Get a summary of changes (for logging)
    pub fn capture_summary(&self, working_dir: &Path) -> Result<DiffSummary, GitError> {
        let repo = Repository::discover(working_dir)?;

        let head_tree = match repo.head() {
            Ok(head) => Some(head.peel_to_tree()?),
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => None,
            Err(e) => return Err(GitError::GitOperationFailed(e)),
        };

        let mut opts = DiffOptions::new();
        let diff = repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut opts))?;

        let stats = diff.stats()?;

        Ok(DiffSummary {
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
        })
    }
}
