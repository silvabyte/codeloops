//! Interview session state management.
//!
//! Handles the state of an interview session, including conversation history,
//! the current draft, and session persistence for resumption.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::protocol::{AgentMessage, UserResponse};
use super::scanner::ProjectContext;

/// Current session format version (for migration support)
pub const SESSION_VERSION: u32 = 2;

/// An interview session that can be saved and resumed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterviewSession {
    /// Unique session identifier
    pub id: String,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
    /// Project context gathered from scanning
    pub project_context: ProjectContext,
    /// Conversation history
    pub history: Vec<ConversationTurn>,
    /// Current state of the draft
    pub draft: PromptDraft,
    /// Path where the final prompt.md will be written
    pub output_path: PathBuf,
    /// Whether the interview is complete
    pub is_complete: bool,
    /// Session format version (for migrations)
    #[serde(default = "default_version")]
    pub version: u32,
}

/// Default version for old sessions without version field
fn default_version() -> u32 {
    1
}

/// A single turn in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Who sent this message
    pub role: Role,
    /// The content of the turn
    pub content: TurnContent,
    /// When this turn occurred
    pub timestamp: DateTime<Utc>,
}

/// The role of a conversation participant
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System messages (prompts, context)
    System,
    /// The AI agent conducting the interview
    Agent,
    /// The user being interviewed
    User,
}

/// Content of a conversation turn
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TurnContent {
    /// Agent message (question, draft update, etc.)
    AgentMessage(AgentMessage),
    /// User response
    UserResponse(UserResponse),
    /// Raw text (for system messages)
    Text(String),
}

/// The draft prompt being built during the interview
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptDraft {
    /// Title of the task/feature
    pub title: Option<String>,
    /// Main goal or objective
    pub goal: Option<String>,
    /// Background context and motivation
    pub context: Option<String>,
    /// Detailed requirements
    pub requirements: Vec<String>,
    /// Constraints and limitations
    pub constraints: Vec<String>,
    /// Files that will be modified
    pub files_to_modify: Vec<String>,
    /// How to verify the task is complete
    pub acceptance_criteria: Vec<String>,
    /// Any additional notes
    pub notes: Option<String>,
    /// Edge cases to consider
    #[serde(default)]
    pub edge_cases: Vec<String>,
    /// Error handling approach
    #[serde(default)]
    pub error_handling: Option<String>,
    /// Testing strategy
    #[serde(default)]
    pub testing_strategy: Option<String>,
    /// User flow description
    #[serde(default)]
    pub user_flow: Option<String>,
}

impl PromptDraft {
    /// Create a new empty draft
    pub fn new() -> Self {
        Self::default()
    }

    /// Update a section of the draft
    pub fn update_section(&mut self, section: &str, content: &str, append: bool) {
        match section.to_lowercase().as_str() {
            "title" => self.title = Some(content.to_string()),
            "goal" => self.goal = Some(content.to_string()),
            "context" => {
                if append {
                    if let Some(ref mut existing) = self.context {
                        existing.push_str("\n\n");
                        existing.push_str(content);
                    } else {
                        self.context = Some(content.to_string());
                    }
                } else {
                    self.context = Some(content.to_string());
                }
            }
            "requirements" => {
                if append {
                    self.requirements.push(content.to_string());
                } else {
                    self.requirements = vec![content.to_string()];
                }
            }
            "constraints" => {
                if append {
                    self.constraints.push(content.to_string());
                } else {
                    self.constraints = vec![content.to_string()];
                }
            }
            "files" | "files_to_modify" => {
                if append {
                    self.files_to_modify.push(content.to_string());
                } else {
                    self.files_to_modify = vec![content.to_string()];
                }
            }
            "acceptance_criteria" | "acceptance" | "criteria" => {
                if append {
                    self.acceptance_criteria.push(content.to_string());
                } else {
                    self.acceptance_criteria = vec![content.to_string()];
                }
            }
            "notes" => {
                if append {
                    if let Some(ref mut existing) = self.notes {
                        existing.push_str("\n\n");
                        existing.push_str(content);
                    } else {
                        self.notes = Some(content.to_string());
                    }
                } else {
                    self.notes = Some(content.to_string());
                }
            }
            // New sections (Task 3.2)
            "edge_cases" | "edge cases" => {
                if append {
                    self.edge_cases.push(content.to_string());
                } else {
                    self.edge_cases = vec![content.to_string()];
                }
            }
            "error_handling" | "error handling" | "errors" => {
                if append {
                    if let Some(ref mut existing) = self.error_handling {
                        existing.push_str("\n\n");
                        existing.push_str(content);
                    } else {
                        self.error_handling = Some(content.to_string());
                    }
                } else {
                    self.error_handling = Some(content.to_string());
                }
            }
            "testing_strategy" | "testing strategy" | "testing" | "tests" => {
                if append {
                    if let Some(ref mut existing) = self.testing_strategy {
                        existing.push_str("\n\n");
                        existing.push_str(content);
                    } else {
                        self.testing_strategy = Some(content.to_string());
                    }
                } else {
                    self.testing_strategy = Some(content.to_string());
                }
            }
            "user_flow" | "user flow" | "flow" => {
                if append {
                    if let Some(ref mut existing) = self.user_flow {
                        existing.push_str("\n\n");
                        existing.push_str(content);
                    } else {
                        self.user_flow = Some(content.to_string());
                    }
                } else {
                    self.user_flow = Some(content.to_string());
                }
            }
            _ => {
                // Unknown section, add to notes
                if append {
                    if let Some(ref mut existing) = self.notes {
                        existing.push_str(&format!("\n\n## {}\n{}", section, content));
                    } else {
                        self.notes = Some(format!("## {}\n{}", section, content));
                    }
                } else {
                    self.notes = Some(format!("## {}\n{}", section, content));
                }
            }
        }
    }

    /// Convert the draft to markdown format
    pub fn to_markdown(&self) -> String {
        let mut parts = Vec::new();

        // Title
        if let Some(ref title) = self.title {
            parts.push(format!("# {}\n", title));
        }

        // Goal
        if let Some(ref goal) = self.goal {
            parts.push(format!("## Goal\n\n{}\n", goal));
        }

        // Context
        if let Some(ref context) = self.context {
            parts.push(format!("## Context\n\n{}\n", context));
        }

        // Requirements
        if !self.requirements.is_empty() {
            parts.push("## Requirements\n".to_string());
            for req in &self.requirements {
                // Check if it's already formatted as a list item
                if req.starts_with('-') || req.starts_with('*') || req.starts_with("1.") {
                    parts.push(format!("{}\n", req));
                } else {
                    parts.push(format!("- {}\n", req));
                }
            }
            parts.push("\n".to_string());
        }

        // Constraints
        if !self.constraints.is_empty() {
            parts.push("## Constraints\n".to_string());
            for constraint in &self.constraints {
                if constraint.starts_with('-') || constraint.starts_with('*') {
                    parts.push(format!("{}\n", constraint));
                } else {
                    parts.push(format!("- {}\n", constraint));
                }
            }
            parts.push("\n".to_string());
        }

        // Files to modify
        if !self.files_to_modify.is_empty() {
            parts.push("## Files to Modify\n".to_string());
            for file in &self.files_to_modify {
                if file.starts_with('-') || file.starts_with('*') {
                    parts.push(format!("{}\n", file));
                } else {
                    parts.push(format!("- `{}`\n", file));
                }
            }
            parts.push("\n".to_string());
        }

        // Acceptance criteria
        if !self.acceptance_criteria.is_empty() {
            parts.push("## Acceptance Criteria\n".to_string());
            for criteria in &self.acceptance_criteria {
                if criteria.starts_with('-')
                    || criteria.starts_with('*')
                    || criteria.starts_with('[')
                {
                    parts.push(format!("{}\n", criteria));
                } else {
                    parts.push(format!("- [ ] {}\n", criteria));
                }
            }
            parts.push("\n".to_string());
        }

        // Edge Cases (new section)
        if !self.edge_cases.is_empty() {
            parts.push("## Edge Cases\n".to_string());
            for edge_case in &self.edge_cases {
                if edge_case.starts_with('-') || edge_case.starts_with('*') {
                    parts.push(format!("{}\n", edge_case));
                } else {
                    parts.push(format!("- {}\n", edge_case));
                }
            }
            parts.push("\n".to_string());
        }

        // Error Handling (new section)
        if let Some(ref error_handling) = self.error_handling {
            parts.push(format!("## Error Handling\n\n{}\n", error_handling));
        }

        // Testing Strategy (new section)
        if let Some(ref testing_strategy) = self.testing_strategy {
            parts.push(format!("## Testing Strategy\n\n{}\n", testing_strategy));
        }

        // User Flow (new section)
        if let Some(ref user_flow) = self.user_flow {
            parts.push(format!("## User Flow\n\n{}\n", user_flow));
        }

        // Notes
        if let Some(ref notes) = self.notes {
            parts.push(format!("## Notes\n\n{}\n", notes));
        }

        parts.join("")
    }

    /// Check if the draft has minimal content
    #[allow(dead_code)]
    pub fn has_content(&self) -> bool {
        self.title.is_some()
            || self.goal.is_some()
            || self.context.is_some()
            || !self.requirements.is_empty()
    }

    /// Estimate completion percentage
    pub fn completion_percentage(&self) -> u8 {
        let mut score = 0u8;
        let mut max_score = 0u8;

        // Title (required)
        max_score += 15;
        if self.title.is_some() {
            score += 15;
        }

        // Goal (required)
        max_score += 20;
        if self.goal.is_some() {
            score += 20;
        }

        // Context (important)
        max_score += 15;
        if self.context.is_some() {
            score += 15;
        }

        // Requirements (required, more = better)
        max_score += 25;
        if !self.requirements.is_empty() {
            score += (15 + self.requirements.len().min(10) as u8).min(25);
        }

        // Constraints (optional but good)
        max_score += 10;
        if !self.constraints.is_empty() {
            score += 10;
        }

        // Acceptance criteria (important)
        max_score += 15;
        if !self.acceptance_criteria.is_empty() {
            score += 15;
        }

        ((score as f32 / max_score as f32) * 100.0) as u8
    }
}

impl InterviewSession {
    /// Create a new interview session
    pub fn new(project_context: ProjectContext, output_path: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: generate_session_id(),
            created_at: now,
            updated_at: now,
            project_context,
            history: Vec::new(),
            draft: PromptDraft::new(),
            output_path,
            is_complete: false,
            version: SESSION_VERSION,
        }
    }

    /// Load a session from a file.
    ///
    /// Automatically migrates older session formats to the current version.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read session file: {}", path.display()))?;
        let mut session: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse session file: {}", path.display()))?;

        // Perform migrations if needed
        if session.version < SESSION_VERSION {
            session.migrate()?;
        }

        Ok(session)
    }

    /// Migrate session from older version to current version.
    fn migrate(&mut self) -> Result<()> {
        // v1 -> v2: Add new draft sections (handled by serde defaults)
        // The new fields (edge_cases, error_handling, testing_strategy, user_flow)
        // are already initialized to defaults by #[serde(default)]

        // Update version
        self.version = SESSION_VERSION;
        self.updated_at = Utc::now();

        Ok(())
    }

    /// Save the session to a file atomically.
    ///
    /// Uses write-to-temp-then-rename pattern to avoid corruption on crash.
    pub fn save(&self) -> Result<PathBuf> {
        let sessions_dir = get_sessions_dir()?;
        let path = sessions_dir.join(format!("{}.json", self.id));
        let temp_path = sessions_dir.join(format!(".{}.json.tmp", self.id));

        let content = serde_json::to_string_pretty(self).context("Failed to serialize session")?;

        // Write to temp file first
        std::fs::write(&temp_path, &content)
            .with_context(|| format!("Failed to write temp file: {}", temp_path.display()))?;

        // Atomic rename (on same filesystem)
        std::fs::rename(&temp_path, &path).with_context(|| {
            // Try to clean up temp file on error
            let _ = std::fs::remove_file(&temp_path);
            format!(
                "Failed to rename {} to {}",
                temp_path.display(),
                path.display()
            )
        })?;

        Ok(path)
    }

    /// Get the path where this session would be saved
    pub fn session_path(&self) -> Result<PathBuf> {
        let sessions_dir = get_sessions_dir()?;
        Ok(sessions_dir.join(format!("{}.json", self.id)))
    }

    /// Add an agent message to the history
    pub fn add_agent_message(&mut self, message: AgentMessage) {
        self.history.push(ConversationTurn {
            role: Role::Agent,
            content: TurnContent::AgentMessage(message),
            timestamp: Utc::now(),
        });
        self.updated_at = Utc::now();
    }

    /// Add a user response to the history
    pub fn add_user_response(&mut self, response: UserResponse) {
        self.history.push(ConversationTurn {
            role: Role::User,
            content: TurnContent::UserResponse(response),
            timestamp: Utc::now(),
        });
        self.updated_at = Utc::now();
    }

    /// Add a system message to the history
    #[allow(dead_code)]
    pub fn add_system_message(&mut self, message: impl Into<String>) {
        self.history.push(ConversationTurn {
            role: Role::System,
            content: TurnContent::Text(message.into()),
            timestamp: Utc::now(),
        });
        self.updated_at = Utc::now();
    }

    /// Apply a draft update from the agent
    pub fn apply_draft_update(&mut self, section: &str, content: &str, append: bool) {
        self.draft.update_section(section, content, append);
        self.updated_at = Utc::now();
    }

    /// Mark the session as complete
    pub fn mark_complete(&mut self) {
        self.is_complete = true;
        self.updated_at = Utc::now();
    }

    /// Build the conversation history as a string for the agent prompt
    pub fn history_for_prompt(&self) -> String {
        let mut parts = Vec::new();

        for turn in &self.history {
            match &turn.content {
                TurnContent::AgentMessage(msg) => match msg {
                    AgentMessage::Question { text, .. } => {
                        parts.push(format!("Assistant: {}", text));
                    }
                    AgentMessage::Clarification { text, .. } => {
                        parts.push(format!("Assistant (clarification): {}", text));
                    }
                    AgentMessage::Thinking { message } => {
                        parts.push(format!("Assistant (thinking): {}", message));
                    }
                    AgentMessage::DraftUpdate {
                        section, content, ..
                    } => {
                        parts.push(format!(
                            "Assistant (updating draft section '{}'): {}",
                            section,
                            content.chars().take(100).collect::<String>()
                        ));
                    }
                    AgentMessage::DraftComplete { summary } => {
                        parts.push(format!("Assistant: Draft complete. {}", summary));
                    }
                    AgentMessage::Error { message } => {
                        parts.push(format!("Assistant (error): {}", message));
                    }
                },
                TurnContent::UserResponse(resp) => {
                    parts.push(format!("User: {}", resp.answer.to_prompt_string()));
                    if let Some(ref feedback) = resp.feedback {
                        parts.push(format!("User (additional): {}", feedback));
                    }
                }
                TurnContent::Text(text) => {
                    parts.push(format!("System: {}", text));
                }
            }
        }

        parts.join("\n\n")
    }
}

/// Get the directory for storing interview sessions
fn get_sessions_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine local data directory"))?;
    let sessions_dir = data_dir.join("codeloops").join("interviews");

    if !sessions_dir.exists() {
        std::fs::create_dir_all(&sessions_dir).with_context(|| {
            format!(
                "Failed to create sessions directory: {}",
                sessions_dir.display()
            )
        })?;
    }

    Ok(sessions_dir)
}

/// Generate a unique session ID
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let random: u32 = rand_simple();

    format!("interview-{}-{:08x}", timestamp, random)
}

/// Simple random number generator (no external dependency)
fn rand_simple() -> u32 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    hasher.write_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64,
    );
    hasher.finish() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draft_to_markdown() {
        let mut draft = PromptDraft::new();
        draft.title = Some("Add User Authentication".to_string());
        draft.goal = Some("Implement secure user authentication".to_string());
        draft.context = Some("The app currently has no auth".to_string());
        draft
            .requirements
            .push("Support email/password login".to_string());
        draft
            .requirements
            .push("Add password reset flow".to_string());
        draft
            .constraints
            .push("Must use existing database".to_string());
        draft
            .acceptance_criteria
            .push("Users can log in".to_string());
        draft
            .acceptance_criteria
            .push("Invalid passwords are rejected".to_string());

        let markdown = draft.to_markdown();
        assert!(markdown.contains("# Add User Authentication"));
        assert!(markdown.contains("## Goal"));
        assert!(markdown.contains("## Requirements"));
        assert!(markdown.contains("- Support email/password login"));
        assert!(markdown.contains("- [ ] Users can log in"));
    }

    #[test]
    fn test_draft_update_section() {
        let mut draft = PromptDraft::new();

        draft.update_section("title", "My Feature", false);
        assert_eq!(draft.title, Some("My Feature".to_string()));

        draft.update_section("requirements", "First req", false);
        assert_eq!(draft.requirements.len(), 1);

        draft.update_section("requirements", "Second req", true);
        assert_eq!(draft.requirements.len(), 2);
    }

    #[test]
    fn test_completion_percentage() {
        let mut draft = PromptDraft::new();
        assert_eq!(draft.completion_percentage(), 0);

        draft.title = Some("Test".to_string());
        let pct1 = draft.completion_percentage();
        assert!(pct1 > 0);

        draft.goal = Some("Do something".to_string());
        let pct2 = draft.completion_percentage();
        assert!(pct2 > pct1);

        draft.requirements.push("Req 1".to_string());
        let pct3 = draft.completion_percentage();
        assert!(pct3 > pct2);
    }

    #[test]
    fn test_session_serialization() {
        let context = super::super::scanner::ProjectContext {
            project_type: super::super::scanner::ProjectType::Rust,
            languages: vec!["Rust".to_string()],
            frameworks: vec![],
            key_files: vec![],
            directory_structure: vec![],
            project_name: Some("test".to_string()),
            project_description: None,
        };

        let session = InterviewSession::new(context, PathBuf::from("prompt.md"));
        let json = serde_json::to_string(&session).unwrap();
        let parsed: InterviewSession = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, session.id);
        assert_eq!(
            parsed.project_context.project_name,
            Some("test".to_string())
        );
    }

    #[test]
    fn test_new_sections_update() {
        let mut draft = PromptDraft::new();

        // Test edge_cases
        draft.update_section("edge_cases", "Empty input", false);
        assert_eq!(draft.edge_cases.len(), 1);
        draft.update_section("edge_cases", "Null values", true);
        assert_eq!(draft.edge_cases.len(), 2);

        // Test error_handling
        draft.update_section("error_handling", "Retry on network error", false);
        assert_eq!(
            draft.error_handling,
            Some("Retry on network error".to_string())
        );

        // Test testing_strategy
        draft.update_section("testing_strategy", "Unit tests for all modules", false);
        assert_eq!(
            draft.testing_strategy,
            Some("Unit tests for all modules".to_string())
        );

        // Test user_flow
        draft.update_section("user_flow", "User logs in then views dashboard", false);
        assert_eq!(
            draft.user_flow,
            Some("User logs in then views dashboard".to_string())
        );
    }

    #[test]
    fn test_new_sections_markdown_rendering() {
        let mut draft = PromptDraft::new();
        draft.title = Some("Test Feature".to_string());
        draft.edge_cases.push("Empty input".to_string());
        draft.edge_cases.push("Very long input".to_string());
        draft.error_handling = Some("Log all errors".to_string());
        draft.testing_strategy = Some("Unit tests required".to_string());
        draft.user_flow = Some("Step 1: User clicks button".to_string());

        let markdown = draft.to_markdown();
        assert!(markdown.contains("## Edge Cases"));
        assert!(markdown.contains("- Empty input"));
        assert!(markdown.contains("- Very long input"));
        assert!(markdown.contains("## Error Handling"));
        assert!(markdown.contains("Log all errors"));
        assert!(markdown.contains("## Testing Strategy"));
        assert!(markdown.contains("Unit tests required"));
        assert!(markdown.contains("## User Flow"));
        assert!(markdown.contains("Step 1: User clicks button"));
    }

    #[test]
    fn test_session_version() {
        let context = super::super::scanner::ProjectContext {
            project_type: super::super::scanner::ProjectType::Rust,
            languages: vec!["Rust".to_string()],
            frameworks: vec![],
            key_files: vec![],
            directory_structure: vec![],
            project_name: Some("test".to_string()),
            project_description: None,
        };

        let session = InterviewSession::new(context, PathBuf::from("prompt.md"));
        assert_eq!(session.version, SESSION_VERSION);
    }

    #[test]
    fn test_v1_session_loads_with_defaults() {
        // Simulate a v1 session JSON without version field and new draft sections
        let v1_json = r#"{
            "id": "interview-test-12345678",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "project_context": {
                "project_type": "rust",
                "languages": ["Rust"],
                "frameworks": [],
                "key_files": [],
                "directory_structure": [],
                "project_name": "test",
                "project_description": null
            },
            "history": [],
            "draft": {
                "title": "Old Session",
                "goal": "Test migration",
                "context": null,
                "requirements": [],
                "constraints": [],
                "files_to_modify": [],
                "acceptance_criteria": [],
                "notes": null
            },
            "output_path": "prompt.md",
            "is_complete": false
        }"#;

        let session: InterviewSession = serde_json::from_str(v1_json).unwrap();

        // Version should default to 1
        assert_eq!(session.version, 1);

        // New draft fields should have defaults
        assert!(session.draft.edge_cases.is_empty());
        assert!(session.draft.error_handling.is_none());
        assert!(session.draft.testing_strategy.is_none());
        assert!(session.draft.user_flow.is_none());

        // Old fields should be preserved
        assert_eq!(session.draft.title, Some("Old Session".to_string()));
        assert_eq!(session.draft.goal, Some("Test migration".to_string()));
    }
}
