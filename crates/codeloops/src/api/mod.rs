pub mod extractors;
mod projects;
mod prompt;
mod prompt_instructions;
mod sessions;
mod stats;

use std::sync::Arc;

use axum::routing::{get, post, put};
use axum::Router;
use tower_http::cors::CorsLayer;

use codeloops_db::Database;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
}

pub fn create_router(db: Arc<Database>) -> Router {
    let state = AppState { db };

    Router::new()
        // Project CRUD (not scoped — top-level resource)
        .route("/api/projects", get(projects::list_projects))
        .route("/api/projects", post(projects::create_project))
        .route(
            "/api/projects/{project_id}",
            get(projects::get_project)
                .put(projects::update_project)
                .delete(projects::delete_project),
        )
        // Project-scoped context
        .route(
            "/api/projects/{project_id}/context",
            get(projects::get_project_context),
        )
        // Project-scoped sessions
        .route(
            "/api/projects/{project_id}/sessions",
            get(sessions::list_sessions),
        )
        .route(
            "/api/projects/{project_id}/sessions/{id}",
            get(sessions::get_session),
        )
        .route(
            "/api/projects/{project_id}/sessions/{id}/diff",
            get(sessions::get_session_diff),
        )
        .route(
            "/api/projects/{project_id}/sessions/{id}/output/{iteration}/{phase}",
            get(sessions::stream_output),
        )
        // Project-scoped stats
        .route(
            "/api/projects/{project_id}/stats",
            get(stats::get_stats),
        )
        .route(
            "/api/projects/{project_id}/metrics",
            get(stats::get_metrics),
        )
        // Project-scoped prompt builder
        .route("/api/skills", get(prompt::list_skills))
        .route(
            "/api/projects/{project_id}/prompt-session",
            post(prompt::create_session),
        )
        .route(
            "/api/projects/{project_id}/prompt-session/{session_id}/message",
            post(prompt::send_message),
        )
        .route(
            "/api/projects/{project_id}/prompt/save",
            post(prompt::save_prompt),
        )
        // Project-scoped prompt history
        .route(
            "/api/projects/{project_id}/prompts",
            get(prompt::list_prompts).post(prompt::save_prompt_session),
        )
        .route(
            "/api/projects/{project_id}/prompts/{id}",
            get(prompt::get_prompt).delete(prompt::delete_prompt),
        )
        // Prompt inheritance
        .route(
            "/api/projects/{project_id}/prompts/{id}/parents",
            put(prompt::update_prompt_parents),
        )
        .route(
            "/api/projects/{project_id}/prompts/{id}/resolved",
            get(prompt::get_resolved_prompt),
        )
        .layer(CorsLayer::permissive())
        .with_state(state)
}
