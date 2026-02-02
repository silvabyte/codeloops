mod context;
mod prompt;
mod prompt_instructions;
mod sessions;
mod sse;
mod stats;

use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;

use codeloops_sessions::{SessionStore, SessionWatcher};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<SessionStore>,
    pub watcher: Arc<SessionWatcher>,
    pub working_dir: PathBuf,
}

pub fn create_router(
    store: Arc<SessionStore>,
    watcher: Arc<SessionWatcher>,
    working_dir: PathBuf,
) -> Router {
    let state = AppState {
        store,
        watcher,
        working_dir,
    };

    Router::new()
        // Session browsing
        .route("/api/sessions", get(sessions::list_sessions))
        .route("/api/sessions/live", get(sse::session_events))
        .route("/api/sessions/{id}", get(sessions::get_session))
        .route("/api/sessions/{id}/diff", get(sessions::get_session_diff))
        .route("/api/stats", get(stats::get_stats))
        // Prompt builder
        .route("/api/context", get(context::get_context))
        .route("/api/prompt-session", post(prompt::create_session))
        .route(
            "/api/prompt-session/{session_id}/message",
            post(prompt::send_message),
        )
        .route("/api/prompt/save", post(prompt::save_prompt))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
