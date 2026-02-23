//! Project CRUD API handlers.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::{Deserialize, Serialize};

use codeloops_db::{NewProject, ProjectConfigOverrides, ProjectRecord, ProjectUpdate};

use super::AppState;

// ============================================================================
// Request / Response types
// ============================================================================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub path: String,
    pub name: Option<String>,
    pub config_overrides: Option<ProjectConfigOverrides>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub config_overrides: Option<ProjectConfigOverrides>,
    pub is_default: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectRecord>,
}

// ============================================================================
// Handlers
// ============================================================================

/// List all projects, sorted by last_accessed_at descending.
pub async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<ProjectListResponse>, (StatusCode, String)> {
    let projects = state
        .db
        .projects()
        .list()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ProjectListResponse { projects }))
}

/// Create a new project.
pub async fn create_project(
    State(state): State<AppState>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectRecord>), (StatusCode, String)> {
    // Canonicalize path
    let canonical = std::fs::canonicalize(&req.path).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            format!("Directory does not exist: {}", req.path),
        )
    })?;

    let path_str = canonical.to_string_lossy().to_string();

    // Check for duplicate
    if state
        .db
        .projects()
        .get_by_path(&path_str)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_some()
    {
        return Err((
            StatusCode::CONFLICT,
            "A project already exists for this directory".to_string(),
        ));
    }

    // Default name to directory basename
    let name = req.name.unwrap_or_else(|| {
        canonical
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    });

    let project = state
        .db
        .projects()
        .add(&NewProject {
            path: path_str,
            name,
            config_overrides: req.config_overrides,
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(project)))
}

/// Get a single project by ID.
pub async fn get_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectRecord>, (StatusCode, String)> {
    let project = state
        .db
        .projects()
        .get(&project_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Project not found".to_string()))?;

    Ok(Json(project))
}

/// Update a project.
pub async fn update_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectRecord>, (StatusCode, String)> {
    let update = ProjectUpdate {
        name: req.name,
        config_overrides: req.config_overrides.map(Some),
        is_default: req.is_default,
    };

    let project = state
        .db
        .projects()
        .update(&project_id, &update)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Project not found".to_string()))?;

    Ok(Json(project))
}

/// Delete a project.
pub async fn delete_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let deleted = state
        .db
        .projects()
        .remove(&project_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "Project not found".to_string()))
    }
}

/// Get project context (path, name, config). Project-scoped replacement for /api/context.
pub async fn get_project_context(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectRecord>, (StatusCode, String)> {
    let project = state
        .db
        .projects()
        .get(&project_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Project not found".to_string()))?;

    Ok(Json(project))
}
