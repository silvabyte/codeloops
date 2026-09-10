use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAX_CAPTURE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PAGE_SIZE: usize = 100;
pub const EXCERPT_CHARS: usize = 4096;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Origin {
    pub device_id: String,
    pub installation_id: String,
    pub source: String,
    pub source_version: String,
}

/// An immutable delivery. Sequence is persistent and monotonic within an installation.
/// It orders observations, not causality between agents or devices.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Capture {
    pub schema_version: u32,
    pub delivery_id: String,
    pub origin: Origin,
    pub sequence: u64,
    pub native_session_id: String,
    pub project_id: String,
    pub workspace_id: String,
    pub observed_at: u64,
    pub occurred_at: Option<u64>,
    pub change: Change,
    pub source_payload: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checkpoints: Vec<CheckpointLink>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Change {
    Tool {
        native_id: String,
        name: String,
        parent_native_id: Option<String>,
        status: String,
        input: Option<Value>,
        output: Option<Value>,
        error: Option<Value>,
    },
    Message {
        native_id: String,
        role: String,
        parent_native_id: Option<String>,
        removed: bool,
    },
    Part {
        message_id: String,
        native_id: String,
        kind: String,
        text: String,
        removed: bool,
    },
    Lifecycle {
        state: Option<String>,
        title: Option<String>,
        parent_native_id: Option<String>,
    },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    pub project_id: Option<String>,
    pub source: Option<String>,
    pub device_id: Option<String>,
    pub session_id: Option<String>,
    pub role: Option<String>,
    pub kind: Option<String>,
    /// Inclusive observation time, milliseconds since Unix epoch.
    pub since: Option<u64>,
    /// Exclusive observation time, milliseconds since Unix epoch.
    pub until: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Page {
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub cursor: Option<String>,
}

fn default_limit() -> usize {
    20
}

impl Default for Page {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            cursor: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Query {
    Export {
        session_id: String,
    },
    Checkpoint {
        checkpoint_id: String,
    },
    Compare {
        before: String,
        after: String,
        #[serde(default)]
        before_layer: Layer,
        #[serde(default)]
        after_layer: Layer,
        #[serde(default)]
        page: Page,
    },
    Changes {
        session_id: Option<String>,
        entry_id: Option<String>,
        workspace_id: String,
        #[serde(default)]
        page: Page,
    },
    File {
        checkpoint_id: String,
        /// Base64-encoded repository-relative path bytes, as returned by compare.
        path: String,
        #[serde(default)]
        layer: Layer,
        #[serde(default)]
        offset: usize,
        #[serde(default = "default_chunk")]
        limit: usize,
    },
    List {
        #[serde(default)]
        filter: Filter,
        #[serde(default)]
        page: Page,
    },
    Search {
        text: String,
        #[serde(default)]
        filter: Filter,
        #[serde(default)]
        page: Page,
    },
    Show {
        session_id: String,
        #[serde(default)]
        page: Page,
    },
    Entry {
        entry_id: String,
    },
    Captures {
        session_id: String,
        #[serde(default)]
        page: Page,
    },
    Artifact {
        hash: String,
        #[serde(default)]
        offset: usize,
        #[serde(default = "default_chunk")]
        limit: usize,
    },
}

fn default_chunk() -> usize {
    65536
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub capture_id: String,
    pub session_id: String,
    pub entry_id: Option<String>,
    pub recorded_at: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Head,
    Index,
    #[default]
    Worktree,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct CheckpointLink {
    pub workspace_id: String,
    pub checkpoint_id: Option<String>,
    pub before_id: Option<String>,
    pub baseline_id: Option<String>,
    pub baseline_status: String,
    /// fresh, reused, late, missing, or failed. This is boundary coverage,
    /// independent of the checkpoint's file-content coverage.
    pub status: String,
    pub reason: Option<String>,
    #[serde(default)]
    pub concurrent_tools: bool,
}
