use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Mutex;

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;

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
// Session State
// ============================================================================

#[derive(Debug, Clone)]
pub struct PromptSession {
    pub id: String,
    pub work_type: String,
    pub working_dir: String,
    pub messages: Vec<ChatMessage>,
    pub prompt_draft: String,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

// Global session store (simple in-memory for now)
lazy_static::lazy_static! {
    static ref PROMPT_SESSIONS: Mutex<HashMap<String, PromptSession>> = Mutex::new(HashMap::new());
}

// ============================================================================
// Interview Flow Definitions
// ============================================================================

fn get_initial_prompt(work_type: &str) -> &'static str {
    match work_type {
        "feature" => {
            "Let's design this feature together. I'll ask you some questions to understand what you're building.\n\n\
            First, **who is this feature for** and **what problem does it solve**? \
            What should they be able to do that they can't do now?"
        }
        "defect" => {
            "Let's figure out what's going wrong. I'll help you capture the issue clearly.\n\n\
            **What's happening** that shouldn't be? And **what should be happening** instead?\n\n\
            If you have specific reproduction steps, share those too."
        }
        "risk" => {
            "Let's identify and document this risk properly.\n\n\
            **What risk have you identified?** This could be a security vulnerability, \
            performance issue, or technical concern.\n\n\
            How did you discover it, and what's the potential impact?"
        }
        "debt" => {
            "Let's tackle this technical debt. I'll help you plan the cleanup.\n\n\
            **What's the current state** that needs improvement? Why is it problematic now, \
            and what pain is it causing?"
        }
        _ => {
            "Tell me about what you're working on.\n\n\
            **What's the goal?** What do you want to accomplish?"
        }
    }
}

fn get_follow_up_prompt(work_type: &str, message_count: usize) -> Option<&'static str> {
    match work_type {
        "feature" => match message_count {
            2 => Some(
                "Good context. Now let's think about the **technical approach**.\n\n\
                What components or files will likely need to change? \
                Are there existing patterns in the codebase we should follow?"
            ),
            4 => Some(
                "Let's get more specific about the **implementation**.\n\n\
                For each component you mentioned, what's the rough shape of the change? \
                Are there any edge cases or error conditions to handle?"
            ),
            6 => Some(
                "Almost there. Let's define **done**.\n\n\
                What are the acceptance criteria? How will you verify it works correctly?"
            ),
            _ => None,
        },
        "defect" => match message_count {
            2 => Some(
                "Let's dig into the **root cause**.\n\n\
                Where in the code do you think this is happening? \
                Any theories on why?"
            ),
            4 => Some(
                "What's your **fix strategy**?\n\n\
                How will you address the root cause? Which files will you modify?"
            ),
            6 => Some(
                "How will you **verify the fix**?\n\n\
                What tests should pass? How will you prevent regression?"
            ),
            _ => None,
        },
        "risk" => match message_count {
            2 => Some(
                "What's the **current state** of the risk?\n\n\
                Where does it exist in the code? Are there any mitigations in place?"
            ),
            4 => Some(
                "What's your **remediation plan**?\n\n\
                How will you address this risk? What files need to change?"
            ),
            6 => Some(
                "How will you **validate** the fix?\n\n\
                What checks will confirm the risk is mitigated?"
            ),
            _ => None,
        },
        "debt" => match message_count {
            2 => Some(
                "What's the **target state**?\n\n\
                What should this look like after the cleanup? \
                What patterns should it follow?"
            ),
            4 => Some(
                "What's your **refactoring plan**?\n\n\
                Which files will you touch? What's the safe order of operations?"
            ),
            6 => Some(
                "How will you **verify** the refactoring?\n\n\
                What tests must pass? How will you confirm behavior is preserved?"
            ),
            _ => None,
        },
        _ => None,
    }
}

// ============================================================================
// Handlers
// ============================================================================

pub async fn create_session(
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, (StatusCode, String)> {
    let session_id = format!("prompt-{}", uuid::Uuid::new_v4());

    let session = PromptSession {
        id: session_id.clone(),
        work_type: req.work_type,
        working_dir: req.working_dir,
        messages: Vec::new(),
        prompt_draft: String::new(),
    };

    PROMPT_SESSIONS
        .lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Lock error".to_string()))?
        .insert(session_id.clone(), session);

    Ok(Json(CreateSessionResponse { session_id }))
}

pub async fn send_message(
    Path(session_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)>
{
    // Get session
    let mut sessions = PROMPT_SESSIONS
        .lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Lock error".to_string()))?;

    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    // Handle initial message or user message
    let response_content = if req.content == "__INIT__" {
        get_initial_prompt(&session.work_type).to_string()
    } else {
        // Add user message to history
        session.messages.push(ChatMessage {
            role: "user".to_string(),
            content: req.content.clone(),
        });

        // Get AI response based on interview flow
        let message_count = session.messages.len();
        if let Some(follow_up) = get_follow_up_prompt(&session.work_type, message_count) {
            follow_up.to_string()
        } else {
            // Default response - summarize and build prompt
            build_summary_response(&session.work_type, &session.messages)
        }
    };

    // Add assistant response to history
    session.messages.push(ChatMessage {
        role: "assistant".to_string(),
        content: response_content.clone(),
    });

    // Build prompt draft
    let prompt_draft = build_prompt_draft(&session.work_type, &session.messages);
    session.prompt_draft = prompt_draft.clone();

    // Create SSE stream (simulated streaming for now)
    let chunks: Vec<String> = response_content
        .split(' ')
        .map(|s| s.to_string())
        .collect();

    let stream = tokio_stream::iter(chunks.into_iter().enumerate())
        .map(move |(i, chunk)| {
            let data = if i == 0 {
                serde_json::json!({ "content": chunk })
            } else {
                serde_json::json!({ "content": format!(" {}", chunk) })
            };
            Ok(Event::default().data(serde_json::to_string(&data).unwrap_or_default()))
        })
        .chain(tokio_stream::once(Ok(Event::default().data(
            serde_json::to_string(&serde_json::json!({ "promptDraft": prompt_draft }))
                .unwrap_or_default(),
        ))))
        .chain(tokio_stream::once(Ok(Event::default().data("[DONE]"))));

    Ok(Sse::new(stream))
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
// Prompt Building
// ============================================================================

fn build_summary_response(work_type: &str, messages: &[ChatMessage]) -> String {
    let user_messages: Vec<_> = messages.iter().filter(|m| m.role == "user").collect();

    if user_messages.is_empty() {
        return "Thanks for the details. The prompt is taking shape in the preview panel. \
                Keep adding context or edit the preview directly when you're ready."
            .to_string();
    }

    format!(
        "I've captured the key points in the preview panel. You can:\n\n\
         - **Keep chatting** to add more detail\n\
         - **Edit the preview** directly to refine it\n\
         - **Save** when you're happy with it\n\n\
         Is there anything else important about this {} I should know?",
        work_type
    )
}

fn build_prompt_draft(work_type: &str, messages: &[ChatMessage]) -> String {
    let user_messages: Vec<_> = messages
        .iter()
        .filter(|m| m.role == "user")
        .map(|m| m.content.as_str())
        .collect();

    if user_messages.is_empty() {
        return String::new();
    }

    let header = match work_type {
        "feature" => "# Feature: ",
        "defect" => "# Bug Fix: ",
        "risk" => "# Risk Mitigation: ",
        "debt" => "# Technical Debt: ",
        _ => "# Task: ",
    };

    // Extract first line of first message as title
    let title = user_messages
        .first()
        .unwrap_or(&"")
        .lines()
        .next()
        .unwrap_or("Untitled");

    let mut draft = format!("{}{}\n\n", header, title);

    // Add sections based on work type
    match work_type {
        "feature" => {
            draft.push_str("## Problem\n");
            if let Some(first) = user_messages.first() {
                draft.push_str(first);
                draft.push_str("\n\n");
            }

            if user_messages.len() > 1 {
                draft.push_str("## Technical Approach\n");
                draft.push_str(user_messages.get(1).unwrap_or(&""));
                draft.push_str("\n\n");
            }

            if user_messages.len() > 2 {
                draft.push_str("## Implementation Details\n");
                draft.push_str(user_messages.get(2).unwrap_or(&""));
                draft.push_str("\n\n");
            }

            if user_messages.len() > 3 {
                draft.push_str("## Acceptance Criteria\n");
                draft.push_str(user_messages.get(3).unwrap_or(&""));
                draft.push_str("\n\n");
            }
        }
        "defect" => {
            draft.push_str("## Symptom\n");
            if let Some(first) = user_messages.first() {
                draft.push_str(first);
                draft.push_str("\n\n");
            }

            if user_messages.len() > 1 {
                draft.push_str("## Root Cause\n");
                draft.push_str(user_messages.get(1).unwrap_or(&""));
                draft.push_str("\n\n");
            }

            if user_messages.len() > 2 {
                draft.push_str("## Fix Strategy\n");
                draft.push_str(user_messages.get(2).unwrap_or(&""));
                draft.push_str("\n\n");
            }

            if user_messages.len() > 3 {
                draft.push_str("## Verification\n");
                draft.push_str(user_messages.get(3).unwrap_or(&""));
                draft.push_str("\n\n");
            }
        }
        "risk" => {
            draft.push_str("## Risk Identified\n");
            if let Some(first) = user_messages.first() {
                draft.push_str(first);
                draft.push_str("\n\n");
            }

            if user_messages.len() > 1 {
                draft.push_str("## Current State\n");
                draft.push_str(user_messages.get(1).unwrap_or(&""));
                draft.push_str("\n\n");
            }

            if user_messages.len() > 2 {
                draft.push_str("## Remediation Plan\n");
                draft.push_str(user_messages.get(2).unwrap_or(&""));
                draft.push_str("\n\n");
            }

            if user_messages.len() > 3 {
                draft.push_str("## Validation\n");
                draft.push_str(user_messages.get(3).unwrap_or(&""));
                draft.push_str("\n\n");
            }
        }
        "debt" => {
            draft.push_str("## Current State\n");
            if let Some(first) = user_messages.first() {
                draft.push_str(first);
                draft.push_str("\n\n");
            }

            if user_messages.len() > 1 {
                draft.push_str("## Target State\n");
                draft.push_str(user_messages.get(1).unwrap_or(&""));
                draft.push_str("\n\n");
            }

            if user_messages.len() > 2 {
                draft.push_str("## Refactoring Plan\n");
                draft.push_str(user_messages.get(2).unwrap_or(&""));
                draft.push_str("\n\n");
            }

            if user_messages.len() > 3 {
                draft.push_str("## Verification\n");
                draft.push_str(user_messages.get(3).unwrap_or(&""));
                draft.push_str("\n\n");
            }
        }
        _ => {
            draft.push_str("## Description\n");
            for msg in user_messages {
                draft.push_str(msg);
                draft.push_str("\n\n");
            }
        }
    }

    draft.trim().to_string()
}
