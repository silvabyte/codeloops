mod context;
mod prompt;
mod prompt_instructions;
mod sessions;
mod sse;
mod stats;

use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::cors::CorsLayer;

use codeloops_db::Database;
use codeloops_sessions::{SessionStore, SessionWatcher};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<SessionStore>,
    pub watcher: Arc<SessionWatcher>,
    pub working_dir: PathBuf,
    pub db: Arc<Database>,
}

pub fn create_router(
    store: Arc<SessionStore>,
    watcher: Arc<SessionWatcher>,
    working_dir: PathBuf,
    db: Arc<Database>,
) -> Router {
    let state = AppState {
        store,
        watcher,
        working_dir,
        db,
    };

    Router::new()
        // Session browsing
        .route("/api/sessions", get(sessions::list_sessions))
        .route("/api/sessions/live", get(sse::session_events))
        .route("/api/sessions/{id}", get(sessions::get_session))
        .route("/api/sessions/{id}/diff", get(sessions::get_session_diff))
        .route("/api/stats", get(stats::get_stats))
        .route("/api/metrics", get(stats::get_metrics))
        // Prompt builder
        .route("/api/context", get(context::get_context))
        .route("/api/prompt-session", post(prompt::create_session))
        .route(
            "/api/prompt-session/{session_id}/message",
            post(prompt::send_message),
        )
        .route("/api/prompt/save", post(prompt::save_prompt))
        // Prompt history
        .route("/api/prompts", get(prompt::list_prompts))
        .route("/api/prompts", post(prompt::save_prompt_session))
        .route("/api/prompts/{id}", get(prompt::get_prompt))
        .route("/api/prompts/{id}", delete(prompt::delete_prompt))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
