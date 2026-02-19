use std::convert::Infallible;
use std::path::PathBuf;
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::Json;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use codeloops_db::{Session, SessionFilter, SessionSummary};

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
        .db
        .sessions()
        .list(&filter)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(summaries))
}

pub async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Session>, (StatusCode, String)> {
    let session = state
        .db
        .sessions()
        .get(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Session not found: {}", id)))?;

    Ok(Json(session))
}

pub async fn get_session_diff(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<String, (StatusCode, String)> {
    let diff = state
        .db
        .sessions()
        .get_diff(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .unwrap_or_default();

    Ok(diff)
}

/// SSE endpoint for streaming live agent output from temp files.
///
/// `GET /api/sessions/{id}/output/{iteration}/{phase}`
///
/// - `phase` is one of: `actor`, `critic`
/// - For in-progress phases: tails the temp file and streams new lines
/// - For completed phases: returns full output from DB as a single event + [DONE]
pub async fn stream_output(
    State(state): State<AppState>,
    Path((id, iteration, phase)): Path<(String, usize, String)>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)>
{
    if phase != "actor" && phase != "critic" {
        return Err((
            StatusCode::BAD_REQUEST,
            "Phase must be 'actor' or 'critic'".to_string(),
        ));
    }

    let (tx, rx) = mpsc::channel::<OutputEvent>(1000);
    let db = state.db.clone();

    tokio::spawn(async move {
        let output_dir = output_dir_for_session(&id);
        let stdout_path = output_dir.join(format!("iter_{}_{}.stdout", iteration, phase));
        let stderr_path = output_dir.join(format!("iter_{}_{}.stderr", iteration, phase));

        let mut stdout_pos: u64 = 0;
        let mut stderr_pos: u64 = 0;
        let mut wait_count: u32 = 0;
        let max_waits: u32 = 300; // 30 seconds at 100ms intervals

        loop {
            // Read new stdout data
            if let Some(data) = read_from_position(&stdout_path, &mut stdout_pos).await {
                for line in data.lines() {
                    if !line.is_empty() {
                        let event = OutputEvent::Line {
                            line: line.to_string(),
                            stream: "stdout".to_string(),
                        };
                        if tx.send(event).await.is_err() {
                            return; // Client disconnected
                        }
                    }
                }
            }

            // Read new stderr data
            if let Some(data) = read_from_position(&stderr_path, &mut stderr_pos).await {
                for line in data.lines() {
                    if !line.is_empty() {
                        let event = OutputEvent::Line {
                            line: line.to_string(),
                            stream: "stderr".to_string(),
                        };
                        if tx.send(event).await.is_err() {
                            return;
                        }
                    }
                }
            }

            // Check if the phase is complete in the DB
            if is_phase_complete(&db, &id, iteration, &phase) {
                // Flush any remaining data
                tokio::time::sleep(Duration::from_millis(50)).await;
                for path in [&stdout_path, &stderr_path] {
                    let stream_name = if path == &stdout_path {
                        "stdout"
                    } else {
                        "stderr"
                    };
                    let pos = if path == &stdout_path {
                        &mut stdout_pos
                    } else {
                        &mut stderr_pos
                    };
                    if let Some(data) = read_from_position(path, pos).await {
                        for line in data.lines() {
                            if !line.is_empty() {
                                let _ = tx
                                    .send(OutputEvent::Line {
                                        line: line.to_string(),
                                        stream: stream_name.to_string(),
                                    })
                                    .await;
                            }
                        }
                    }
                }
                let _ = tx.send(OutputEvent::Done).await;
                return;
            }

            // If files don't exist yet, wait a bit then check again
            if !stdout_path.exists() && !stderr_path.exists() {
                wait_count += 1;
                if wait_count >= max_waits {
                    let _ = tx.send(OutputEvent::Done).await;
                    return;
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let stream = ReceiverStream::new(rx).map(|event| match event {
        OutputEvent::Line { line, stream } => {
            let data = serde_json::json!({ "line": line, "stream": stream });
            Ok(Event::default().data(serde_json::to_string(&data).unwrap_or_default()))
        }
        OutputEvent::Done => Ok(Event::default().data("[DONE]")),
    });

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive"),
    ))
}

enum OutputEvent {
    Line { line: String, stream: String },
    Done,
}

fn output_dir_for_session(session_id: &str) -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("codeloops")
        .join("output")
        .join(session_id)
}

/// Read new data from a file starting at the given byte position.
/// Updates `pos` to the new file position after reading.
async fn read_from_position(path: &PathBuf, pos: &mut u64) -> Option<String> {
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    let mut file = tokio::fs::File::open(path).await.ok()?;
    let metadata = file.metadata().await.ok()?;
    let file_len = metadata.len();

    if file_len <= *pos {
        return None;
    }

    file.seek(std::io::SeekFrom::Start(*pos)).await.ok()?;
    let mut buffer = vec![0u8; (file_len - *pos) as usize];
    let bytes_read = file.read(&mut buffer).await.ok()?;
    *pos += bytes_read as u64;

    if bytes_read == 0 {
        return None;
    }

    buffer.truncate(bytes_read);
    Some(String::from_utf8_lossy(&buffer).to_string())
}

/// Check if a phase is complete by looking at the iteration's phase in the DB.
fn is_phase_complete(
    db: &std::sync::Arc<codeloops_db::Database>,
    session_id: &str,
    iteration: usize,
    phase: &str,
) -> bool {
    let target_phase = match phase {
        "actor" => "actor_completed",
        "critic" => "critic_completed",
        _ => return false,
    };

    // The phase is "complete" if the DB phase is at or beyond the target
    let phase_order = |p: &str| -> u8 {
        match p {
            "actor_started" => 0,
            "actor_completed" => 1,
            "diff_captured" => 2,
            "critic_started" => 3,
            "critic_completed" => 4,
            _ => 0,
        }
    };

    if let Ok(Some(session)) = db.sessions().get(session_id) {
        for iter in &session.iterations {
            if iter.iteration_number == iteration {
                return phase_order(&iter.phase) >= phase_order(target_phase);
            }
        }
        // Also check if session is complete (ended_at is set)
        return session.ended_at.is_some();
    }

    false
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
