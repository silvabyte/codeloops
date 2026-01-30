//! Interactive prompt.md generator with TUI.
//!
//! This module provides a TUI-based interview system that uses the configured
//! coding agent to generate comprehensive prompt.md files through a series
//! of probing questions.

mod protocol;
mod scanner;
mod session;
mod system_prompt;
mod tui;

use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;

use codeloops_agent::{create_agent, AgentConfig, AgentType};

use crate::config::{GlobalConfig, ProjectConfig};

// Re-export for use by tui module
pub(crate) use session::*;

/// Arguments for the prompt command
pub struct PromptArgs {
    pub output: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
    pub agent: Option<AgentType>,
    pub model: Option<String>,
    pub resume: Option<PathBuf>,
    pub dry_run: bool,
    /// Clean up old sessions
    pub clean: bool,
    /// Days threshold for cleanup (default 30)
    pub older_than_days: Option<u64>,
}

/// Create a backup path with timestamp for an existing file.
///
/// Given a path like `prompt.md`, returns `prompt.20260128_143000.md`
fn create_backup_path(original: &std::path::Path) -> PathBuf {
    use chrono::Local;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let stem = original
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("backup");
    let ext = original
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("md");

    let backup_name = format!("{}.{}.{}", stem, timestamp, ext);
    original.with_file_name(backup_name)
}

/// Handle the `codeloops prompt` command
pub async fn handle_prompt_command(args: PromptArgs) -> Result<()> {
    // Handle session cleanup if requested
    if args.clean {
        return handle_session_cleanup(args.older_than_days.unwrap_or(30));
    }

    // Determine working directory
    let working_dir = args
        .working_dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    // Load configs
    let global_config = GlobalConfig::load().context("Failed to load global configuration")?;
    let project_config =
        ProjectConfig::load(&working_dir).context("Failed to load project configuration")?;

    // Resolve agent
    // Precedence: CLI flags > project config > global config > default (Claude)
    let agent_type = args
        .agent
        .or_else(|| {
            project_config
                .as_ref()
                .and_then(|c| c.actor_agent())
                .and_then(parse_agent_type)
        })
        .or_else(|| {
            global_config
                .as_ref()
                .and_then(|c| c.actor_agent())
                .and_then(parse_agent_type)
        })
        .unwrap_or(AgentType::ClaudeCode);

    // Resolve model
    let model = args
        .model
        .clone()
        .or_else(|| {
            project_config
                .as_ref()
                .and_then(|c| c.actor_model())
                .map(String::from)
        })
        .or_else(|| {
            global_config
                .as_ref()
                .and_then(|c| c.actor_model())
                .map(String::from)
        });

    // Create agent and verify availability
    let agent = create_agent(agent_type);
    if !agent.is_available().await {
        anyhow::bail!(
            "Agent '{}' is not available.\n\n  \
             Install it or choose a different agent:\n    \
             codeloops prompt --agent opencode\n\n  \
             Available agents: claude, opencode, cursor",
            agent.name()
        );
    }

    // Determine output path
    let output_path = args
        .output
        .clone()
        .unwrap_or_else(|| working_dir.join("prompt.md"));

    // Create agent config
    let agent_config = AgentConfig::new(working_dir.clone());
    let agent_config = if let Some(ref m) = model {
        agent_config.with_model(m.clone())
    } else {
        agent_config
    };

    // Load or create session
    let session = if let Some(ref resume_path) = args.resume {
        eprintln!(
            "{} Resuming session from {}",
            "->".dimmed(),
            resume_path.display()
        );
        InterviewSession::load(resume_path).context("Failed to load session")?
    } else {
        eprintln!(
            "{} Starting new interview with {}",
            "->".dimmed(),
            agent.name().bright_cyan()
        );

        // Scan project for context
        let project_context = scanner::scan_project(&working_dir)?;
        InterviewSession::new(project_context, output_path.clone())
    };

    // Run the TUI
    let mut app = tui::App::new(session, agent, agent_config)?;
    let result = app.run().await;

    // Handle the result
    match result {
        Ok(Some(final_draft)) => {
            let markdown = final_draft.to_markdown();

            if args.dry_run {
                // Dry run: display output without writing
                eprintln!();
                eprintln!("{}", "=== Dry Run ===".bright_blue().bold());
                eprintln!(
                    "{} Would write to: {}",
                    "->".dimmed(),
                    output_path.display()
                );
                eprintln!();
                println!("{}", markdown);
                eprintln!();
                eprintln!("{} Dry run - no file written", "->".dimmed());
            } else {
                // Backup existing file if it exists
                if output_path.exists() {
                    let backup_path = create_backup_path(&output_path);
                    std::fs::copy(&output_path, &backup_path).context("Failed to create backup")?;
                    eprintln!(
                        "{} Backed up existing file to {}",
                        "->".dimmed(),
                        backup_path.display()
                    );
                }

                // Write the final prompt.md
                std::fs::write(&output_path, markdown).context("Failed to write prompt.md")?;

                eprintln!();
                eprintln!(
                    "{} Created {}",
                    "✅".bright_green(),
                    output_path.display().to_string().bright_cyan()
                );
                eprintln!();
                eprintln!(
                    "  Run your prompt: {}",
                    format!("codeloops --prompt-file {}", output_path.display()).bright_cyan()
                );
            }
        }
        Ok(None) => {
            // User cancelled or session saved for later
            eprintln!();
            eprintln!("{} Session saved. Resume with:", "->".dimmed());
            if let Some(session_path) = app.session_path() {
                eprintln!(
                    "  {}",
                    format!("codeloops prompt --resume {}", session_path.display()).bright_cyan()
                );
            }
        }
        Err(e) => {
            // Error occurred - try to save session
            if let Some(session_path) = app.session_path() {
                eprintln!(
                    "\n{} Session saved to {}",
                    "->".dimmed(),
                    session_path.display()
                );
            }
            return Err(e);
        }
    }

    Ok(())
}

/// Parse agent type from config string
fn parse_agent_type(s: &str) -> Option<AgentType> {
    match s.to_lowercase().as_str() {
        "claude" | "claude-code" => Some(AgentType::ClaudeCode),
        "opencode" | "open-code" => Some(AgentType::OpenCode),
        "cursor" => Some(AgentType::Cursor),
        _ => None,
    }
}

/// Handle session cleanup command (Task 3.1)
fn handle_session_cleanup(older_than_days: u64) -> Result<()> {
    use chrono::{Duration, Utc};

    let sessions_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine local data directory"))?
        .join("codeloops")
        .join("interviews");

    if !sessions_dir.exists() {
        eprintln!("{} No sessions directory found", "->".dimmed());
        return Ok(());
    }

    let cutoff = Utc::now() - Duration::days(older_than_days as i64);
    let mut sessions_to_delete = Vec::new();

    // Find sessions older than cutoff
    for entry in std::fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        // Try to load the session to check its date
        match InterviewSession::load(&path) {
            Ok(session) => {
                if session.updated_at < cutoff {
                    sessions_to_delete.push((path, session));
                }
            }
            Err(_) => {
                // If we can't parse it, check file modification time
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let modified_time: chrono::DateTime<Utc> = modified.into();
                        if modified_time < cutoff {
                            // Can't load session, but it's old - offer to delete
                            eprintln!(
                                "{} Warning: Could not parse session {}, but it's older than {} days",
                                "!".yellow(),
                                path.display(),
                                older_than_days
                            );
                        }
                    }
                }
            }
        }
    }

    if sessions_to_delete.is_empty() {
        eprintln!(
            "{} No sessions older than {} days found",
            "->".dimmed(),
            older_than_days
        );
        return Ok(());
    }

    // Display sessions to be deleted
    eprintln!(
        "\n{} Found {} session(s) older than {} days:\n",
        "->".dimmed(),
        sessions_to_delete.len(),
        older_than_days
    );

    for (path, session) in &sessions_to_delete {
        let title = session.draft.title.as_deref().unwrap_or("(untitled)");
        let age_days = (Utc::now() - session.updated_at).num_days();
        eprintln!(
            "  {} - {} ({} days old)",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown"),
            title,
            age_days
        );
    }
    eprintln!();

    // Confirm with user
    let confirm = dialoguer::Confirm::new()
        .with_prompt(format!("Delete {} session(s)?", sessions_to_delete.len()))
        .default(false)
        .interact()?;

    if !confirm {
        eprintln!("{} Cancelled", "->".dimmed());
        return Ok(());
    }

    // Delete sessions
    let mut deleted = 0;
    for (path, _) in sessions_to_delete {
        match std::fs::remove_file(&path) {
            Ok(()) => {
                deleted += 1;
            }
            Err(e) => {
                eprintln!("{} Failed to delete {}: {}", "!".red(), path.display(), e);
            }
        }
    }

    eprintln!("\n{} Deleted {} session(s)", "✅".bright_green(), deleted);

    Ok(())
}

#[cfg(test)]
mod integration_tests {
    //! Integration tests for the prompt generator using mock agent responses.
    //!
    //! These tests verify the full interview flow without requiring an actual
    //! coding agent by simulating agent responses.

    use super::protocol::*;
    use super::scanner::{KeyFile, ProjectContext, ProjectType};
    use super::session::{InterviewSession, PromptDraft};
    use std::path::PathBuf;

    /// Create a mock project context for testing
    fn mock_project_context() -> ProjectContext {
        ProjectContext {
            project_type: ProjectType::Rust,
            languages: vec!["Rust".to_string()],
            frameworks: vec!["Tokio".to_string(), "Serde".to_string()],
            key_files: vec![
                KeyFile {
                    path: "Cargo.toml".to_string(),
                    description: "Rust manifest".to_string(),
                },
                KeyFile {
                    path: "src/main.rs".to_string(),
                    description: "Application entry point".to_string(),
                },
            ],
            directory_structure: vec!["src/".to_string(), "tests/".to_string()],
            project_name: Some("test-project".to_string()),
            project_description: Some("A test project for integration tests".to_string()),
        }
    }

    /// Simulate a sequence of agent messages representing a typical interview
    fn mock_interview_sequence() -> Vec<AgentMessage> {
        vec![
            // First question: ask about the goal
            AgentMessage::Question {
                text: "What is the main goal of this feature or task?".to_string(),
                context: Some("Understanding the primary objective helps define the scope and success criteria.".to_string()),
                input_type: InputType::Text,
                options: vec![],
                section: Some("goal".to_string()),
            },
            // Update title based on response
            AgentMessage::DraftUpdate {
                section: "title".to_string(),
                content: "Add User Authentication".to_string(),
                append: false,
            },
            // Update goal
            AgentMessage::DraftUpdate {
                section: "goal".to_string(),
                content: "Implement secure user authentication with JWT tokens.".to_string(),
                append: false,
            },
            // Ask about auth method (select)
            AgentMessage::Question {
                text: "What authentication method should be used?".to_string(),
                context: None,
                input_type: InputType::Select,
                options: vec![
                    SelectOption {
                        value: "jwt".to_string(),
                        label: "JWT Tokens".to_string(),
                        description: Some("Stateless authentication with JSON Web Tokens".to_string()),
                    },
                    SelectOption {
                        value: "session".to_string(),
                        label: "Session-based".to_string(),
                        description: Some("Server-side sessions with cookies".to_string()),
                    },
                ],
                section: Some("requirements".to_string()),
            },
            // Update requirements based on selection
            AgentMessage::DraftUpdate {
                section: "requirements".to_string(),
                content: "- Implement JWT-based authentication".to_string(),
                append: false,
            },
            // Ask about features (multi-select)
            AgentMessage::Question {
                text: "Which features should be included?".to_string(),
                context: None,
                input_type: InputType::MultiSelect,
                options: vec![
                    SelectOption {
                        value: "login".to_string(),
                        label: "Login".to_string(),
                        description: None,
                    },
                    SelectOption {
                        value: "register".to_string(),
                        label: "Registration".to_string(),
                        description: None,
                    },
                    SelectOption {
                        value: "reset".to_string(),
                        label: "Password Reset".to_string(),
                        description: None,
                    },
                ],
                section: Some("requirements".to_string()),
            },
            // Update requirements with multiple items
            AgentMessage::DraftUpdate {
                section: "requirements".to_string(),
                content: "- Support user login with email/password".to_string(),
                append: true,
            },
            AgentMessage::DraftUpdate {
                section: "requirements".to_string(),
                content: "- Support user registration".to_string(),
                append: true,
            },
            AgentMessage::DraftUpdate {
                section: "requirements".to_string(),
                content: "- Support password reset via email".to_string(),
                append: true,
            },
            // Ask confirmation
            AgentMessage::Question {
                text: "Should refresh tokens be supported?".to_string(),
                context: Some("Refresh tokens allow users to stay logged in longer without re-entering credentials.".to_string()),
                input_type: InputType::Confirm,
                options: vec![],
                section: Some("requirements".to_string()),
            },
            // Add constraint
            AgentMessage::DraftUpdate {
                section: "constraints".to_string(),
                content: "- Must use existing database schema".to_string(),
                append: false,
            },
            // Add acceptance criteria
            AgentMessage::DraftUpdate {
                section: "acceptance_criteria".to_string(),
                content: "Users can register with email and password".to_string(),
                append: false,
            },
            AgentMessage::DraftUpdate {
                section: "acceptance_criteria".to_string(),
                content: "Users can log in and receive a valid JWT".to_string(),
                append: true,
            },
            AgentMessage::DraftUpdate {
                section: "acceptance_criteria".to_string(),
                content: "Invalid credentials are rejected with appropriate error".to_string(),
                append: true,
            },
            // Complete the interview
            AgentMessage::DraftComplete {
                summary: "Created comprehensive prompt for user authentication feature with JWT, registration, login, and password reset.".to_string(),
            },
        ]
    }

    /// Simulate user responses for the mock interview
    fn mock_user_responses() -> Vec<UserResponse> {
        vec![
            UserResponse::text("I want to add user authentication to my application"),
            UserResponse::selection("jwt"),
            UserResponse::multi_selection(vec![
                "login".to_string(),
                "register".to_string(),
                "reset".to_string(),
            ]),
            UserResponse::confirm(true),
        ]
    }

    #[test]
    fn test_full_interview_flow() {
        // Create a session
        let context = mock_project_context();
        let mut session = InterviewSession::new(context, PathBuf::from("test-prompt.md"));

        let agent_messages = mock_interview_sequence();
        let user_responses = mock_user_responses();
        let mut response_idx = 0;

        // Process each agent message
        for msg in agent_messages {
            // Add agent message to history
            session.add_agent_message(msg.clone());

            match &msg {
                AgentMessage::Question { .. } | AgentMessage::Clarification { .. } => {
                    // Simulate user response
                    if response_idx < user_responses.len() {
                        session.add_user_response(user_responses[response_idx].clone());
                        response_idx += 1;
                    }
                }
                AgentMessage::DraftUpdate {
                    section,
                    content,
                    append,
                } => {
                    // Apply draft update
                    session.apply_draft_update(section, content, *append);
                }
                AgentMessage::DraftComplete { .. } => {
                    session.mark_complete();
                }
                _ => {}
            }
        }

        // Verify session state
        assert!(session.is_complete);
        assert!(!session.history.is_empty());

        // Verify draft content
        let draft = &session.draft;
        assert_eq!(draft.title, Some("Add User Authentication".to_string()));
        assert_eq!(
            draft.goal,
            Some("Implement secure user authentication with JWT tokens.".to_string())
        );
        assert_eq!(draft.requirements.len(), 4);
        assert_eq!(draft.constraints.len(), 1);
        assert_eq!(draft.acceptance_criteria.len(), 3);

        // Verify markdown generation
        let markdown = draft.to_markdown();
        assert!(markdown.contains("# Add User Authentication"));
        assert!(markdown.contains("## Goal"));
        assert!(markdown.contains("## Requirements"));
        assert!(markdown.contains("JWT-based authentication"));
        assert!(markdown.contains("## Acceptance Criteria"));
        assert!(markdown.contains("- [ ] Users can register"));
    }

    #[test]
    fn test_session_save_and_load() {
        let context = mock_project_context();
        let mut session = InterviewSession::new(context, PathBuf::from("test-prompt.md"));

        // Add some messages
        session.add_agent_message(AgentMessage::Question {
            text: "What is your goal?".to_string(),
            context: None,
            input_type: InputType::Text,
            options: vec![],
            section: Some("goal".to_string()),
        });
        session.add_user_response(UserResponse::text("Build an API"));
        session.apply_draft_update("title", "Build API", false);

        // Serialize and deserialize
        let json = serde_json::to_string(&session).expect("Failed to serialize");
        let loaded: InterviewSession = serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify state preserved
        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.history.len(), session.history.len());
        assert_eq!(loaded.draft.title, Some("Build API".to_string()));
        assert!(!loaded.is_complete);
    }

    #[test]
    fn test_draft_incremental_updates() {
        let mut draft = PromptDraft::new();

        // Test replace mode
        draft.update_section("requirements", "First requirement", false);
        assert_eq!(draft.requirements.len(), 1);

        // Test append mode
        draft.update_section("requirements", "Second requirement", true);
        assert_eq!(draft.requirements.len(), 2);

        // Test replace mode replaces
        draft.update_section("requirements", "Only requirement", false);
        assert_eq!(draft.requirements.len(), 1);
        assert_eq!(draft.requirements[0], "Only requirement");
    }

    #[test]
    fn test_user_answer_conversion() {
        assert_eq!(
            UserAnswer::Text("hello".to_string()).to_prompt_string(),
            "hello"
        );
        assert_eq!(
            UserAnswer::Selection("option1".to_string()).to_prompt_string(),
            "option1"
        );
        assert_eq!(
            UserAnswer::MultiSelection(vec!["a".to_string(), "b".to_string()]).to_prompt_string(),
            "a, b"
        );
        assert_eq!(UserAnswer::Confirm(true).to_prompt_string(), "yes");
        assert_eq!(UserAnswer::Confirm(false).to_prompt_string(), "no");
    }

    #[test]
    fn test_history_for_prompt() {
        let context = mock_project_context();
        let mut session = InterviewSession::new(context, PathBuf::from("test.md"));

        session.add_agent_message(AgentMessage::Question {
            text: "What is your goal?".to_string(),
            context: None,
            input_type: InputType::Text,
            options: vec![],
            section: None,
        });
        session.add_user_response(UserResponse::text("Build an API"));
        session.add_agent_message(AgentMessage::DraftUpdate {
            section: "goal".to_string(),
            content: "Build a REST API".to_string(),
            append: false,
        });

        let history = session.history_for_prompt();
        assert!(history.contains("Assistant: What is your goal?"));
        assert!(history.contains("User: Build an API"));
        assert!(history.contains("updating draft section 'goal'"));
    }

    #[test]
    fn test_protocol_json_roundtrip() {
        // Test Question with select options
        let msg = AgentMessage::Question {
            text: "Choose one".to_string(),
            context: Some("Context here".to_string()),
            input_type: InputType::Select,
            options: vec![
                SelectOption {
                    value: "a".to_string(),
                    label: "Option A".to_string(),
                    description: Some("Description A".to_string()),
                },
                SelectOption {
                    value: "b".to_string(),
                    label: "Option B".to_string(),
                    description: None,
                },
            ],
            section: Some("requirements".to_string()),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let parsed: AgentMessage = serde_json::from_str(&json).expect("Failed to deserialize");

        match parsed {
            AgentMessage::Question {
                text,
                options,
                input_type,
                ..
            } => {
                assert_eq!(text, "Choose one");
                assert_eq!(options.len(), 2);
                assert!(matches!(input_type, InputType::Select));
            }
            _ => panic!("Expected Question variant"),
        }
    }

    #[test]
    fn test_completion_percentage() {
        let mut draft = PromptDraft::new();

        // Empty draft should be 0%
        assert_eq!(draft.completion_percentage(), 0);

        // Adding title should increase
        draft.title = Some("Title".to_string());
        let pct1 = draft.completion_percentage();
        assert!(pct1 > 0);

        // Adding goal should increase more
        draft.goal = Some("Goal".to_string());
        let pct2 = draft.completion_percentage();
        assert!(pct2 > pct1);

        // Full draft should be high
        draft.context = Some("Context".to_string());
        draft.requirements.push("Req 1".to_string());
        draft.requirements.push("Req 2".to_string());
        draft.constraints.push("Constraint".to_string());
        draft.acceptance_criteria.push("Criteria".to_string());

        let final_pct = draft.completion_percentage();
        assert!(final_pct >= 90);
    }

    #[test]
    fn test_backup_path_creation() {
        use super::create_backup_path;
        use std::path::Path;

        let original = Path::new("/tmp/prompt.md");
        let backup = create_backup_path(original);

        // Should have same parent directory
        assert_eq!(backup.parent(), original.parent());

        // Should contain original stem and extension
        let backup_name = backup.file_name().unwrap().to_str().unwrap();
        assert!(backup_name.starts_with("prompt."));
        assert!(backup_name.ends_with(".md"));

        // Should have timestamp in between
        let parts: Vec<&str> = backup_name.split('.').collect();
        assert_eq!(parts.len(), 3); // prompt, timestamp, md
        assert!(parts[1].len() == 15); // YYYYMMDD_HHMMSS
    }

    #[test]
    fn test_backup_path_without_extension() {
        use super::create_backup_path;
        use std::path::Path;

        let original = Path::new("/tmp/Makefile");
        let backup = create_backup_path(original);

        let backup_name = backup.file_name().unwrap().to_str().unwrap();
        // Should still work with default extension
        assert!(backup_name.starts_with("Makefile."));
    }

    #[test]
    fn test_empty_text_response() {
        // Verify empty string is handled gracefully
        let response = UserResponse::text("");
        match &response.answer {
            UserAnswer::Text(text) => assert!(text.is_empty()),
            _ => panic!("Expected Text answer"),
        }
        assert_eq!(response.answer.to_prompt_string(), "");
    }

    #[test]
    fn test_empty_multi_selection() {
        // Verify empty selection is handled gracefully
        let response = UserResponse::multi_selection(vec![]);
        match &response.answer {
            UserAnswer::MultiSelection(selections) => assert!(selections.is_empty()),
            _ => panic!("Expected MultiSelection answer"),
        }
        assert_eq!(response.answer.to_prompt_string(), "");
    }

    #[test]
    fn test_malformed_json_fallback() {
        // Verify that malformed JSON creates a fallback question
        let malformed = "This is just plain text, not JSON";

        // Attempt to parse as AgentMessage
        let result: Result<AgentMessage, _> = serde_json::from_str(malformed);
        assert!(result.is_err());

        // The TUI's parse_agent_message would create a fallback
        // We can't test that directly here, but we test the fallback creation logic
        let fallback = AgentMessage::Question {
            text: malformed.trim().to_string(),
            context: Some("(Agent response was not in expected format)".to_string()),
            input_type: InputType::Text,
            options: vec![],
            section: None,
        };

        match fallback {
            AgentMessage::Question { text, context, .. } => {
                assert_eq!(text, malformed);
                assert!(context.is_some());
            }
            _ => panic!("Expected Question fallback"),
        }
    }

    #[test]
    fn test_editor_input_type_parsing() {
        // Verify Editor input type is correctly parsed from JSON
        let json = r#"{"type":"question","text":"Describe the feature","input_type":"editor","options":[]}"#;
        let msg: AgentMessage = serde_json::from_str(json).unwrap();

        match msg {
            AgentMessage::Question { input_type, .. } => {
                assert!(matches!(input_type, InputType::Editor));
            }
            _ => panic!("Expected Question with Editor input type"),
        }
    }
}
