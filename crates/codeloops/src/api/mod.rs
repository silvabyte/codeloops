mod context;
mod prompt;
mod prompt_instructions;
mod sessions;
mod stats;

use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::{delete, get, post, put};
use axum::Router;
use tower_http::cors::CorsLayer;

use codeloops_db::Database;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub working_dir: PathBuf,
}

pub fn create_router(db: Arc<Database>, working_dir: PathBuf) -> Router {
    let state = AppState { db, working_dir };

    Router::new()
        // Session browsing
        .route("/api/sessions", get(sessions::list_sessions))
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
        // Prompt inheritance
        .route("/api/prompts/{id}/parents", put(prompt::update_prompt_parents))
        .route("/api/prompts/{id}/resolved", get(prompt::get_resolved_prompt))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
