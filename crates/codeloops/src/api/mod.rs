mod sessions;
mod sse;
mod stats;

use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;

use codeloops_sessions::{SessionStore, SessionWatcher};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<SessionStore>,
    pub watcher: Arc<SessionWatcher>,
}

pub fn create_router(store: Arc<SessionStore>, watcher: Arc<SessionWatcher>) -> Router {
    let state = AppState { store, watcher };

    Router::new()
        .route("/api/sessions", get(sessions::list_sessions))
        .route("/api/sessions/live", get(sse::session_events))
        .route("/api/sessions/{id}", get(sessions::get_session))
        .route("/api/sessions/{id}/diff", get(sessions::get_session_diff))
        .route("/api/stats", get(stats::get_stats))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
