use serde::{Deserialize, Serialize};

/// Status of the git working directory
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitStatus {
    pub modified: Vec<String>,
    pub added: Vec<String>,
    pub deleted: Vec<String>,
    pub untracked: Vec<String>,
}

impl GitStatus {
    pub fn is_clean(&self) -> bool {
        self.modified.is_empty()
            && self.added.is_empty()
            && self.deleted.is_empty()
            && self.untracked.is_empty()
    }

    pub fn total_changes(&self) -> usize {
        self.modified.len() + self.added.len() + self.deleted.len() + self.untracked.len()
    }
}
