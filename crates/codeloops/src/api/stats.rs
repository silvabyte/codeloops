use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;

use codeloops_sessions::{SessionFilter, SessionStats};

use super::AppState;

pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<SessionStats>, (StatusCode, String)> {
    let stats = state
        .store
        .stats(&SessionFilter::default())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(stats))
}
