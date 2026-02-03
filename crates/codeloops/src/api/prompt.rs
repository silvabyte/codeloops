//! # Prompt Session API
//!
//! Handles prompt creation and messaging. All state is persisted in SQLite.
//!
//! ## Architecture
//!
//! - **Source of truth**: SQLite database (`prompts` table)
//! - **No in-memory state**: Each request loads/saves from DB
//! - **Session ID = Prompt ID**: The same ID is used for both concepts
//!
//! ## Endpoints
//!
//! - `POST /api/prompt-session` - Create new prompt (persisted immediately)
//! - `POST /api/prompt-session/{id}/message` - Send message (loads from DB, saves after response)
//! - `POST /api/prompts` - Save/update prompt metadata
//! - `GET /api/prompts` - List all prompts
//! - `GET /api/prompts/{id}` - Get single prompt

use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::Json;
use chrono::Utc;
use codeloops_agent::{create_agent, AgentConfig, AgentType, OutputCallback, OutputType};
use codeloops_db::{Database, PromptFilter, PromptRecord};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use super::prompt_instructions::get_system_instructions;
use super::AppState;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    pub work_type: String,
    pub working_dir: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionResponse {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePromptRequest {
    pub working_dir: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct SavePromptResponse {
    pub path: String,
}

// ============================================================================
// Prompt History Types
// ============================================================================

/// Request to save a prompt session to history.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePromptSessionRequest {
    pub id: String,
    pub title: Option<String>,
    pub work_type: String,
    pub project_path: String,
    pub project_name: String,
    pub content: Option<String>,
    pub session_state: SessionStatePayload,
}

/// Session state payload from frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatePayload {
    pub messages: Vec<MessagePayload>,
    pub prompt_draft: String,
    pub preview_open: bool,
}

/// Message payload from frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    pub id: String,
    pub role: String,
    pub content: String,
}

/// Response from saving a prompt session.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePromptSessionResponse {
    pub id: String,
    pub updated_at: String,
}

/// Query parameters for listing prompts.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsQuery {
    pub project_name: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Response for listing prompts.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsResponse {
    pub prompts: Vec<PromptSummary>,
    pub projects: Vec<String>,
}

/// Summary of a prompt for listing.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptSummary {
    pub id: String,
    pub title: Option<String>,
    pub work_type: String,
    pub project_name: String,
    pub content_preview: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response for getting a single prompt.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptResponse {
    pub id: String,
    pub title: Option<String>,
    pub work_type: String,
    pub project_path: String,
    pub project_name: String,
    pub content: Option<String>,
    pub session_state: SessionStatePayload,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Internal Types
// ============================================================================

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// Create a new prompt session.
///
/// Creates a new prompt record in the database immediately, ensuring the session
/// persists across server restarts. The session ID is also the prompt ID.
pub async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, (StatusCode, String)> {
    let session_id = format!("prompt-{}", uuid::Uuid::new_v4());

    // Extract project name from working dir path
    let project_name = extract_project_name(&req.working_dir);

    // Create initial session state
    let session_state = SessionStatePayload {
        messages: vec![],
        prompt_draft: String::new(),
        preview_open: false,
    };

    let record = PromptRecord {
        id: session_id.clone(),
        title: None,
        work_type: req.work_type,
        project_path: req.working_dir,
        project_name,
        content: None,
        session_state: serde_json::to_string(&session_state).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize session state: {}", e),
            )
        })?,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    state
        .db
        .prompts()
        .save(&record)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(CreateSessionResponse { session_id }))
}

/// Extract project name from a path.
fn extract_project_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Send a message to the prompt session.
///
/// Loads the prompt from the database, builds the agent prompt with conversation
/// history, streams the agent response, and saves messages back to the database.
pub async fn send_message(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)>
{
    // Load prompt from DB
    let record = state
        .db
        .prompts()
        .get(&session_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Prompt not found".to_string()))?;

    // Parse session state
    let session_state: SessionStatePayload = serde_json::from_str(&record.session_state)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse session state: {}", e),
            )
        })?;

    let working_dir = record.project_path.clone();
    let work_type = record.work_type.clone();

    // Convert session messages to ChatMessage format
    let messages: Vec<ChatMessage> = session_state
        .messages
        .iter()
        .map(|m| ChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    // Build agent prompt
    let agent_prompt = if req.content == "__INIT__" {
        build_init_prompt(&work_type, &working_dir)
    } else {
        // Build prompt with existing history plus new user message
        build_agent_prompt_from_messages(&work_type, &working_dir, &messages, &req.content)
    };

    // Stream agent response with DB persistence
    stream_agent_response(
        state.db.clone(),
        session_id,
        req.content,
        agent_prompt,
        working_dir,
    )
    .await
}

pub async fn save_prompt(
    Json(req): Json<SavePromptRequest>,
) -> Result<Json<SavePromptResponse>, (StatusCode, String)> {
    let path = std::path::Path::new(&req.working_dir).join("prompt.md");

    std::fs::write(&path, &req.content)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SavePromptResponse {
        path: path.to_string_lossy().to_string(),
    }))
}

// ============================================================================
// Prompt History Handlers
// ============================================================================

/// Save or update a prompt session to the database.
pub async fn save_prompt_session(
    State(state): State<AppState>,
    Json(req): Json<SavePromptSessionRequest>,
) -> Result<Json<SavePromptSessionResponse>, (StatusCode, String)> {
    let now = Utc::now();

    let session_state_json = serde_json::to_string(&req.session_state).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid session state: {}", e),
        )
    })?;

    // Check if this is an update or insert
    let existing = state
        .db
        .prompts()
        .get(&req.id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let created_at = existing.map(|e| e.created_at).unwrap_or(now);

    let record = PromptRecord {
        id: req.id.clone(),
        title: req.title,
        work_type: req.work_type,
        project_path: req.project_path,
        project_name: req.project_name,
        content: req.content,
        session_state: session_state_json,
        created_at,
        updated_at: now,
    };

    state
        .db
        .prompts()
        .save(&record)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SavePromptSessionResponse {
        id: req.id,
        updated_at: now.to_rfc3339(),
    }))
}

/// List prompts with optional filtering.
pub async fn list_prompts(
    State(state): State<AppState>,
    Query(query): Query<ListPromptsQuery>,
) -> Result<Json<ListPromptsResponse>, (StatusCode, String)> {
    let filter = PromptFilter {
        project_name: query.project_name,
        search: query.search,
        limit: query.limit,
        offset: query.offset,
    };

    let records = state
        .db
        .prompts()
        .list(&filter)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let projects = state
        .db
        .prompts()
        .list_projects()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let prompts: Vec<PromptSummary> = records
        .into_iter()
        .map(|r| {
            let content_preview = r.content.as_ref().map(|c| {
                let preview: String = c.chars().take(100).collect();
                if c.len() > 100 {
                    format!("{}...", preview)
                } else {
                    preview
                }
            });

            PromptSummary {
                id: r.id,
                title: r.title,
                work_type: r.work_type,
                project_name: r.project_name,
                content_preview,
                created_at: r.created_at.to_rfc3339(),
                updated_at: r.updated_at.to_rfc3339(),
            }
        })
        .collect();

    Ok(Json(ListPromptsResponse { prompts, projects }))
}

/// Get a single prompt by ID.
pub async fn get_prompt(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<GetPromptResponse>, (StatusCode, String)> {
    let record = state
        .db
        .prompts()
        .get(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Prompt not found".to_string()))?;

    let session_state: SessionStatePayload =
        serde_json::from_str(&record.session_state).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Invalid session state: {}", e),
            )
        })?;

    Ok(Json(GetPromptResponse {
        id: record.id,
        title: record.title,
        work_type: record.work_type,
        project_path: record.project_path,
        project_name: record.project_name,
        content: record.content,
        session_state,
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
    }))
}

/// Delete a prompt by ID.
pub async fn delete_prompt(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let deleted = state
        .db
        .prompts()
        .delete(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "Prompt not found".to_string()))
    }
}

// ============================================================================
// Agent Integration
// ============================================================================

/// Build the initial prompt for when conversation starts.
fn build_init_prompt(work_type: &str, working_dir: &str) -> String {
    let system = get_system_instructions(work_type, working_dir);
    format!(
        "{}\n\n---\n\n\
        The user has selected '{}' as the work type and is ready to start.\n\n\
        Introduce yourself briefly (1-2 sentences) and ask your FIRST question \
        to understand what they're working on. Remember: ask ONE question only.",
        system, work_type
    )
}

/// Build the complete prompt for the agent including conversation history.
///
/// Takes conversation history as a slice of ChatMessage and adds the new user message.
fn build_agent_prompt_from_messages(
    work_type: &str,
    working_dir: &str,
    messages: &[ChatMessage],
    new_message: &str,
) -> String {
    let system = get_system_instructions(work_type, working_dir);

    let mut prompt = String::new();

    // System instructions
    prompt.push_str(&system);
    prompt.push_str("\n\n---\n\n");

    // Conversation history
    if !messages.is_empty() || !new_message.is_empty() {
        prompt.push_str("## Conversation so far:\n\n");
        for msg in messages {
            let role = if msg.role == "user" {
                "User"
            } else {
                "Assistant"
            };
            prompt.push_str(&format!("**{}**: {}\n\n", role, msg.content));
        }
        // Add the new user message
        if !new_message.is_empty() {
            prompt.push_str(&format!("**User**: {}\n\n", new_message));
        }
    }

    // Instructions for this turn
    prompt.push_str("---\n\n");
    prompt.push_str(
        "Continue the interview based on the user's latest message. \
        Either:\n\
        1. Ask ONE focused follow-up question to gather more information, OR\n\
        2. If you have enough information (typically after 4-6 exchanges), \
           generate the prompt.md content within <prompt></prompt> tags.\n\n\
        Keep your response concise and conversational.",
    );

    prompt
}

/// Stream response from agent to SSE.
///
/// After the agent responds, messages are persisted to the database.
async fn stream_agent_response(
    db: Arc<Database>,
    session_id: String,
    user_message: String,
    prompt: String,
    working_dir: String,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)>
{
    // Create channel for streaming
    let (tx, rx) = mpsc::channel::<StreamMessage>(1000);

    // Spawn agent execution in background
    let session_id_clone = session_id.clone();
    let db_clone = db.clone();
    let user_message_clone = user_message.clone();
    tokio::spawn(async move {
        let result = execute_agent(prompt, working_dir, tx.clone()).await;

        match result {
            Ok(full_response) => {
                // Extract prompt draft
                let maybe_draft = extract_prompt_draft(&full_response);

                // Save messages to database
                if let Err(e) = save_messages_to_prompt(
                    &db_clone,
                    &session_id_clone,
                    &user_message_clone,
                    &full_response,
                    maybe_draft.as_deref().unwrap_or(""),
                ) {
                    eprintln!("Failed to save messages to prompt: {}", e);
                }

                // Send prompt draft update
                if let Some(draft) = maybe_draft {
                    let _ = tx.send(StreamMessage::PromptDraft(draft)).await;
                }

                let _ = tx.send(StreamMessage::Done).await;
            }
            Err(e) => {
                let _ = tx
                    .send(StreamMessage::Error(format!("Agent error: {}", e)))
                    .await;
            }
        }
    });

    // Convert channel to SSE stream
    let stream = ReceiverStream::new(rx).map(|msg| match msg {
        StreamMessage::Content(content) => {
            let data = serde_json::json!({ "content": content });
            Ok(Event::default().data(serde_json::to_string(&data).unwrap_or_default()))
        }
        StreamMessage::PromptDraft(draft) => {
            let data = serde_json::json!({ "promptDraft": draft });
            Ok(Event::default().data(serde_json::to_string(&data).unwrap_or_default()))
        }
        StreamMessage::Error(err) => {
            let data = serde_json::json!({ "error": err });
            Ok(Event::default().data(serde_json::to_string(&data).unwrap_or_default()))
        }
        StreamMessage::Done => Ok(Event::default().data("[DONE]")),
    });

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive"),
    ))
}

/// Messages sent through the streaming channel.
#[derive(Debug)]
enum StreamMessage {
    Content(String),
    PromptDraft(String),
    Error(String),
    Done,
}

/// Execute the agent and stream output.
async fn execute_agent(
    prompt: String,
    working_dir: String,
    tx: mpsc::Sender<StreamMessage>,
) -> Result<String, String> {
    let agent = create_agent(AgentType::ClaudeCode);
    let config = AgentConfig::new(PathBuf::from(&working_dir));

    // Check if agent is available
    if !agent.is_available().await {
        return Err(
            "Claude Code agent not available. Please ensure 'claude' CLI is installed.".to_string(),
        );
    }

    // Create callback for streaming output
    let tx_clone = tx.clone();
    let accumulated = Arc::new(Mutex::new(String::new()));
    let accumulated_clone = accumulated.clone();

    let callback: OutputCallback = Arc::new(move |line: &str, output_type: OutputType| {
        // Only stream stdout (agent's actual response)
        if output_type == OutputType::Stdout {
            // Accumulate the full response
            if let Ok(mut acc) = accumulated_clone.lock() {
                if !acc.is_empty() {
                    acc.push('\n');
                }
                acc.push_str(line);
            }

            // Stream the line to frontend
            let tx = tx_clone.clone();
            let line = line.to_string();
            // Use try_send to avoid blocking in the callback
            let _ = tx.try_send(StreamMessage::Content(format!("{}\n", line)));
        }
    });

    // Execute agent
    let output = agent
        .execute_with_callback(&prompt, &config, Some(callback))
        .await
        .map_err(|e| format!("Agent execution failed: {}", e))?;

    // Return the full response (prefer accumulated, fall back to output.stdout)
    let full_response = accumulated
        .lock()
        .map(|acc| {
            if acc.is_empty() {
                output.stdout.clone()
            } else {
                acc.clone()
            }
        })
        .unwrap_or(output.stdout);

    Ok(full_response)
}

// ============================================================================
// Database Persistence
// ============================================================================

/// Save messages to the prompt record in the database.
///
/// Updates the prompt record with the new user and assistant messages.
/// Auto-generates a title from the first user message if not already set.
fn save_messages_to_prompt(
    db: &Database,
    prompt_id: &str,
    user_message: &str,
    assistant_message: &str,
    prompt_draft: &str,
) -> Result<(), String> {
    let mut record = db
        .prompts()
        .get(prompt_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Prompt not found".to_string())?;

    let mut session_state: SessionStatePayload =
        serde_json::from_str(&record.session_state).map_err(|e| e.to_string())?;

    // Add user message (if not the init message)
    if user_message != "__INIT__" {
        session_state.messages.push(MessagePayload {
            id: format!("msg-{}", uuid::Uuid::new_v4()),
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        // Auto-generate title from first user message (if no title yet)
        if record.title.is_none() {
            let title = user_message
                .lines()
                .next()
                .unwrap_or(user_message)
                .chars()
                .take(50)
                .collect::<String>();
            record.title = Some(title);
        }
    }

    // Add assistant message
    session_state.messages.push(MessagePayload {
        id: format!("msg-{}", uuid::Uuid::new_v4()),
        role: "assistant".to_string(),
        content: assistant_message.to_string(),
    });

    // Update prompt draft if provided
    if !prompt_draft.is_empty() {
        session_state.prompt_draft = prompt_draft.to_string();
        record.content = Some(prompt_draft.to_string());
    }

    // Save back
    record.session_state = serde_json::to_string(&session_state).map_err(|e| e.to_string())?;
    record.updated_at = Utc::now();

    db.prompts().save(&record).map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Prompt Draft Extraction
// ============================================================================

/// Extract prompt.md content from agent response if present.
///
/// The agent is instructed to wrap the final prompt in `<prompt>` and `</prompt>` tags
/// when it has gathered enough information to generate the prompt.md content.
fn extract_prompt_draft(response: &str) -> Option<String> {
    let start_tag = "<prompt>";
    let end_tag = "</prompt>";

    if let Some(start) = response.find(start_tag) {
        if let Some(end) = response.find(end_tag) {
            if end > start {
                let content = &response[start + start_tag.len()..end];
                return Some(content.trim().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_prompt_draft_found() {
        let response = "Here's your prompt:\n\n<prompt>\n# Feature: Add login\n\n## Problem\nUsers can't sign in.\n</prompt>\n\nLet me know if you want changes!";
        let draft = extract_prompt_draft(response);
        assert!(draft.is_some());
        let draft = draft.unwrap();
        assert!(draft.contains("# Feature: Add login"));
        assert!(draft.contains("Users can't sign in"));
    }

    #[test]
    fn test_extract_prompt_draft_not_found() {
        let response = "What component will this affect?";
        let draft = extract_prompt_draft(response);
        assert!(draft.is_none());
    }

    #[test]
    fn test_extract_prompt_draft_malformed() {
        let response = "<prompt>content without closing tag";
        let draft = extract_prompt_draft(response);
        assert!(draft.is_none());
    }

    #[test]
    fn test_build_init_prompt() {
        let prompt = build_init_prompt("feature", "/path/to/project");
        assert!(prompt.contains("feature"));
        assert!(prompt.contains("/path/to/project"));
        assert!(prompt.contains("FIRST question"));
    }

    #[test]
    fn test_build_agent_prompt_with_history() {
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: "I want to add a login button".to_string(),
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: "Where should the button appear?".to_string(),
            },
        ];

        let prompt =
            build_agent_prompt_from_messages("feature", "/project", &messages, "On the header");
        assert!(prompt.contains("Conversation so far"));
        assert!(prompt.contains("I want to add a login button"));
        assert!(prompt.contains("Where should the button appear"));
        assert!(prompt.contains("On the header"));
        assert!(prompt.contains("feature"));
    }

    #[test]
    fn test_extract_project_name() {
        assert_eq!(extract_project_name("/home/user/projects/myapp"), "myapp");
        assert_eq!(extract_project_name("/tmp/test"), "test");
        assert_eq!(extract_project_name(""), "unknown");
    }
}
