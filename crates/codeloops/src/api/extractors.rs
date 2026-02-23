//! Shared Axum extractors for project-scoped routes.

use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::Json;
use serde::Serialize;
use std::collections::HashMap;

use codeloops_db::ProjectRecord;

use super::AppState;

/// Extracts and validates a project from the `:project_id` URL parameter.
///
/// Handlers receive a validated `ProjectRecord` — no raw string ID handling.
#[allow(dead_code)]
pub struct ProjectExtractor(pub ProjectRecord);

#[derive(Serialize)]
#[allow(dead_code)]
pub struct ErrorResponse {
    pub error: String,
}

impl FromRequestParts<AppState> for ProjectExtractor {
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract path params to get project_id
        let Path(params): Path<HashMap<String, String>> =
            Path::from_request_parts(parts, state)
                .await
                .map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: "Missing project_id parameter".to_string(),
                        }),
                    )
                })?;

        let project_id = params.get("project_id").ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Missing project_id parameter".to_string(),
                }),
            )
        })?;

        let project = state
            .db
            .projects()
            .get(project_id)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Database error: {}", e),
                    }),
                )
            })?
            .ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "Project not found".to_string(),
                    }),
                )
            })?;

        Ok(ProjectExtractor(project))
    }
}
