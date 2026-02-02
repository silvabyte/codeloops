use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use serde::Serialize;

use super::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextResponse {
    pub working_dir: String,
    pub project_name: String,
}

pub async fn get_context(
    State(state): State<AppState>,
) -> Result<Json<ContextResponse>, (StatusCode, String)> {
    let working_dir = state.working_dir.to_string_lossy().to_string();
    let project_name = state
        .working_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(Json(ContextResponse {
        working_dir,
        project_name,
    }))
}
