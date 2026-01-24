use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;

use codeloops_sessions::{Session, SessionFilter, SessionSummary};

use super::AppState;

#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub outcome: Option<String>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub search: Option<String>,
    pub project: Option<String>,
}

pub async fn list_sessions(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<SessionSummary>>, (StatusCode, String)> {
    let filter = build_filter(params).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let summaries = state
        .store
        .list(&filter)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(summaries))
}

pub async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Session>, (StatusCode, String)> {
    let session = state
        .store
        .get(&id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(session))
}

pub async fn get_session_diff(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<String, (StatusCode, String)> {
    let diff = state
        .store
        .get_diff(&id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(diff)
}

fn build_filter(params: ListParams) -> anyhow::Result<SessionFilter> {
    use chrono::{NaiveDate, TimeZone, Utc};

    let after = params
        .after
        .map(|s| {
            NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map(|d| Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).unwrap()))
                .map_err(|e| anyhow::anyhow!("Invalid after date: {}", e))
        })
        .transpose()?;

    let before = params
        .before
        .map(|s| {
            NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map(|d| Utc.from_utc_datetime(&d.and_hms_opt(23, 59, 59).unwrap()))
                .map_err(|e| anyhow::anyhow!("Invalid before date: {}", e))
        })
        .transpose()?;

    Ok(SessionFilter {
        outcome: params.outcome,
        after,
        before,
        search: params.search,
        project: params.project,
    })
}
