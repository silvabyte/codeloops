//! Sessions store for persistent session storage.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::MutexGuard;
use uuid::Uuid;

/// Data provided when creating a new session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStart {
    pub prompt: String,
    pub working_dir: PathBuf,
    pub actor_agent: String,
    pub critic_agent: String,
    pub actor_model: Option<String>,
    pub critic_model: Option<String>,
    pub max_iterations: Option<usize>,
}

/// Data for a single iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Iteration {
    pub iteration_number: usize,
    pub actor_output: String,
    pub actor_stderr: String,
    pub actor_exit_code: i32,
    pub actor_duration_secs: f64,
    pub git_diff: String,
    pub git_files_changed: usize,
    pub critic_decision: String,
    pub feedback: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Data provided when ending a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEnd {
    pub outcome: String,
    pub iterations: usize,
    pub summary: Option<String>,
    pub confidence: Option<f64>,
    pub duration_secs: f64,
}

/// A fully loaded session record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub prompt: String,
    pub working_dir: PathBuf,
    pub actor_agent: String,
    pub critic_agent: String,
    pub actor_model: Option<String>,
    pub critic_model: Option<String>,
    pub max_iterations: Option<usize>,
    pub outcome: Option<String>,
    pub iteration_count: Option<usize>,
    pub summary: Option<String>,
    pub confidence: Option<f64>,
    pub duration_secs: Option<f64>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub iterations: Vec<Iteration>,
}

/// Summary for list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub prompt_preview: String,
    pub working_dir: PathBuf,
    pub project: String,
    pub outcome: Option<String>,
    pub iterations: usize,
    pub duration_secs: Option<f64>,
    pub confidence: Option<f64>,
    pub actor_agent: String,
    pub critic_agent: String,
}

/// Filter parameters for listing sessions.
#[derive(Debug, Default, Clone)]
pub struct SessionFilter {
    pub outcome: Option<String>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
    pub search: Option<String>,
    pub project: Option<String>,
}

/// Aggregate statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStats {
    pub total_sessions: usize,
    pub success_rate: f64,
    pub avg_iterations: f64,
    pub avg_duration_secs: f64,
    pub sessions_over_time: Vec<DayCount>,
    pub by_project: Vec<ProjectStats>,
}

/// Sessions count for a single day.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayCount {
    pub date: String,
    pub count: usize,
}

/// Per-project statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStats {
    pub project: String,
    pub total: usize,
    pub success_rate: f64,
}

/// Efficacy metrics for agentic software development (DORA-inspired).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgenticMetrics {
    pub total_sessions: usize,
    pub successful_sessions: usize,
    pub success_rate: f64,
    /// % of successful sessions with iterations=1
    pub first_try_success_rate: f64,
    /// Mean iterations for successful sessions
    pub avg_iterations_to_success: f64,
    /// Mean duration for successful sessions
    pub avg_cycle_time_secs: f64,
    /// % failed/interrupted/max_iter
    pub waste_rate: f64,
    pub total_iterations: usize,
    /// % iterations where critic approved
    pub critic_approval_rate: f64,
    /// Mean feedback chars for rejections
    pub avg_feedback_length: f64,
    /// % rejections where next iteration improved
    pub improvement_rate: f64,
    pub sessions_over_time: Vec<DayCount>,
    pub by_project: Vec<ProjectStats>,
}

/// Sessions store with a borrowed connection.
pub struct Sessions<'db> {
    conn: MutexGuard<'db, Connection>,
}

impl<'db> Sessions<'db> {
    /// Create a new Sessions store with a borrowed connection.
    pub(crate) fn new(conn: MutexGuard<'db, Connection>) -> Self {
        Self { conn }
    }

    /// Create a new session, returning the generated UUID.
    pub fn create(&self, start: &SessionStart) -> Result<String, rusqlite::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        self.conn.execute(
            r#"
            INSERT INTO sessions (
                id, prompt, working_dir, actor_agent, critic_agent,
                actor_model, critic_model, max_iterations, started_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                id,
                start.prompt,
                start.working_dir.to_string_lossy().to_string(),
                start.actor_agent,
                start.critic_agent,
                start.actor_model,
                start.critic_model,
                start.max_iterations.map(|n| n as i64),
                now.to_rfc3339(),
            ],
        )?;

        Ok(id)
    }

    /// Add an iteration to a session.
    pub fn add_iteration(&self, session_id: &str, iter: &Iteration) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            r#"
            INSERT INTO iterations (
                session_id, iteration_number, actor_output, actor_stderr,
                actor_exit_code, actor_duration_secs, git_diff, git_files_changed,
                critic_decision, feedback, timestamp
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                session_id,
                iter.iteration_number as i64,
                iter.actor_output,
                iter.actor_stderr,
                iter.actor_exit_code,
                iter.actor_duration_secs,
                iter.git_diff,
                iter.git_files_changed as i64,
                iter.critic_decision,
                iter.feedback,
                iter.timestamp.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// End a session with final outcome data.
    pub fn end(&self, session_id: &str, end: &SessionEnd) -> Result<(), rusqlite::Error> {
        let now = Utc::now();

        self.conn.execute(
            r#"
            UPDATE sessions SET
                outcome = ?1,
                iteration_count = ?2,
                summary = ?3,
                confidence = ?4,
                duration_secs = ?5,
                ended_at = ?6
            WHERE id = ?7
            "#,
            params![
                end.outcome,
                end.iterations as i64,
                end.summary,
                end.confidence,
                end.duration_secs,
                now.to_rfc3339(),
                session_id,
            ],
        )?;

        Ok(())
    }

    /// Get a full session by ID, including all iterations.
    pub fn get(&self, id: &str) -> Result<Option<Session>, rusqlite::Error> {
        let session = self
            .conn
            .query_row(
                r#"
                SELECT id, prompt, working_dir, actor_agent, critic_agent,
                       actor_model, critic_model, max_iterations, outcome,
                       iteration_count, summary, confidence, duration_secs,
                       started_at, ended_at
                FROM sessions WHERE id = ?1
                "#,
                params![id],
                Self::row_to_session,
            )
            .optional()?;

        match session {
            Some(mut s) => {
                s.iterations = self.get_iterations(id)?;
                Ok(Some(s))
            }
            None => Ok(None),
        }
    }

    /// List sessions matching the given filter.
    pub fn list(&self, filter: &SessionFilter) -> Result<Vec<SessionSummary>, rusqlite::Error> {
        let mut sql = String::from(
            r#"
            SELECT id, prompt, working_dir, actor_agent, critic_agent,
                   outcome, iteration_count, duration_secs, confidence, started_at
            FROM sessions WHERE 1=1
            "#,
        );
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref outcome) = filter.outcome {
            sql.push_str(" AND outcome = ?");
            param_values.push(Box::new(outcome.clone()));
        }

        if let Some(after) = filter.after {
            sql.push_str(" AND started_at >= ?");
            param_values.push(Box::new(after.to_rfc3339()));
        }

        if let Some(before) = filter.before {
            sql.push_str(" AND started_at <= ?");
            param_values.push(Box::new(before.to_rfc3339()));
        }

        if let Some(ref search) = filter.search {
            sql.push_str(" AND prompt LIKE ?");
            param_values.push(Box::new(format!("%{}%", search)));
        }

        if let Some(ref project) = filter.project {
            // Project is derived from working_dir, we filter on path
            sql.push_str(" AND working_dir LIKE ?");
            param_values.push(Box::new(format!("%/{}", project)));
        }

        sql.push_str(" ORDER BY started_at DESC");

        let params: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params.as_slice(), Self::row_to_summary)?;

        let mut summaries = Vec::new();
        for row in rows {
            summaries.push(row?);
        }

        Ok(summaries)
    }

    /// Get concatenated diffs from all iterations of a session.
    pub fn get_diff(&self, id: &str) -> Result<Option<String>, rusqlite::Error> {
        let iterations = self.get_iterations(id)?;
        if iterations.is_empty() {
            return Ok(None);
        }

        let diffs: Vec<&str> = iterations
            .iter()
            .filter(|i| !i.git_diff.is_empty())
            .map(|i| i.git_diff.as_str())
            .collect();

        Ok(Some(diffs.join("\n")))
    }

    /// Compute aggregate statistics over sessions matching the filter.
    pub fn stats(&self, filter: &SessionFilter) -> Result<SessionStats, rusqlite::Error> {
        let summaries = self.list(filter)?;
        let total_sessions = summaries.len();

        if total_sessions == 0 {
            return Ok(SessionStats {
                total_sessions: 0,
                success_rate: 0.0,
                avg_iterations: 0.0,
                avg_duration_secs: 0.0,
                sessions_over_time: Vec::new(),
                by_project: Vec::new(),
            });
        }

        let success_count = summaries
            .iter()
            .filter(|s| s.outcome.as_deref() == Some("success"))
            .count();
        let success_rate = success_count as f64 / total_sessions as f64;

        let avg_iterations =
            summaries.iter().map(|s| s.iterations as f64).sum::<f64>() / total_sessions as f64;

        let durations: Vec<f64> = summaries.iter().filter_map(|s| s.duration_secs).collect();
        let avg_duration_secs = if durations.is_empty() {
            0.0
        } else {
            durations.iter().sum::<f64>() / durations.len() as f64
        };

        let sessions_over_time = Self::compute_sessions_over_time(&summaries);
        let by_project = Self::compute_by_project(&summaries);

        Ok(SessionStats {
            total_sessions,
            success_rate,
            avg_iterations,
            avg_duration_secs,
            sessions_over_time,
            by_project,
        })
    }

    /// Compute agentic efficacy metrics (DORA-inspired).
    pub fn agentic_metrics(&self, filter: &SessionFilter) -> Result<AgenticMetrics, rusqlite::Error> {
        let summaries = self.list(filter)?;
        let total_sessions = summaries.len();

        if total_sessions == 0 {
            return Ok(AgenticMetrics {
                total_sessions: 0,
                successful_sessions: 0,
                success_rate: 0.0,
                first_try_success_rate: 0.0,
                avg_iterations_to_success: 0.0,
                avg_cycle_time_secs: 0.0,
                waste_rate: 0.0,
                total_iterations: 0,
                critic_approval_rate: 0.0,
                avg_feedback_length: 0.0,
                improvement_rate: 0.0,
                sessions_over_time: Vec::new(),
                by_project: Vec::new(),
            });
        }

        let successful_summaries: Vec<_> = summaries
            .iter()
            .filter(|s| s.outcome.as_deref() == Some("success"))
            .collect();
        let successful_sessions = successful_summaries.len();
        let success_rate = successful_sessions as f64 / total_sessions as f64;

        let first_try_successes = successful_summaries
            .iter()
            .filter(|s| s.iterations == 1)
            .count();
        let first_try_success_rate = if successful_sessions > 0 {
            first_try_successes as f64 / successful_sessions as f64
        } else {
            0.0
        };

        let avg_iterations_to_success = if successful_sessions > 0 {
            successful_summaries
                .iter()
                .map(|s| s.iterations as f64)
                .sum::<f64>()
                / successful_sessions as f64
        } else {
            0.0
        };

        let successful_durations: Vec<f64> = successful_summaries
            .iter()
            .filter_map(|s| s.duration_secs)
            .collect();
        let avg_cycle_time_secs = if successful_durations.is_empty() {
            0.0
        } else {
            successful_durations.iter().sum::<f64>() / successful_durations.len() as f64
        };

        let waste_outcomes = ["failed", "interrupted", "max_iterations_reached"];
        let waste_count = summaries
            .iter()
            .filter(|s| {
                s.outcome
                    .as_deref()
                    .map(|o| waste_outcomes.contains(&o))
                    .unwrap_or(false)
            })
            .count();
        let waste_rate = waste_count as f64 / total_sessions as f64;

        // Critic metrics: load iterations for all sessions
        let (total_iterations, critic_approval_rate, avg_feedback_length, improvement_rate) =
            self.compute_critic_metrics(&summaries)?;

        let sessions_over_time = Self::compute_sessions_over_time(&summaries);
        let by_project = Self::compute_by_project(&summaries);

        Ok(AgenticMetrics {
            total_sessions,
            successful_sessions,
            success_rate,
            first_try_success_rate,
            avg_iterations_to_success,
            avg_cycle_time_secs,
            waste_rate,
            total_iterations,
            critic_approval_rate,
            avg_feedback_length,
            improvement_rate,
            sessions_over_time,
            by_project,
        })
    }

    /// Return IDs of active sessions (those without an end).
    pub fn active_sessions(&self) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM sessions WHERE ended_at IS NULL ORDER BY started_at DESC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }

        Ok(ids)
    }

    /// Delete a session by ID (cascades to iterations).
    pub fn delete(&self, id: &str) -> Result<bool, rusqlite::Error> {
        let rows_affected = self
            .conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(rows_affected > 0)
    }

    /// Get all unique project names.
    pub fn list_projects(&self) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT DISTINCT
                CASE
                    WHEN working_dir LIKE '%/%' THEN
                        SUBSTR(working_dir, LENGTH(working_dir) - LENGTH(REPLACE(working_dir, '/', '')) + 1)
                    ELSE working_dir
                END as project
            FROM sessions
            ORDER BY project
            "#,
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

        let mut projects = Vec::new();
        for row in rows {
            let project = row?;
            // Remove leading slash if present
            let project = project.trim_start_matches('/').to_string();
            if !project.is_empty() && !projects.contains(&project) {
                projects.push(project);
            }
        }

        Ok(projects)
    }

    // Helper methods

    fn get_iterations(&self, session_id: &str) -> Result<Vec<Iteration>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT iteration_number, actor_output, actor_stderr, actor_exit_code,
                   actor_duration_secs, git_diff, git_files_changed, critic_decision,
                   feedback, timestamp
            FROM iterations
            WHERE session_id = ?1
            ORDER BY iteration_number
            "#,
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            let timestamp_str: String = row.get(9)?;
            Ok(Iteration {
                iteration_number: row.get::<_, i64>(0)? as usize,
                actor_output: row.get(1)?,
                actor_stderr: row.get(2)?,
                actor_exit_code: row.get(3)?,
                actor_duration_secs: row.get(4)?,
                git_diff: row.get(5)?,
                git_files_changed: row.get::<_, i64>(6)? as usize,
                critic_decision: row.get(7)?,
                feedback: row.get(8)?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        let mut iterations = Vec::new();
        for row in rows {
            iterations.push(row?);
        }

        Ok(iterations)
    }

    fn row_to_session(row: &rusqlite::Row) -> Result<Session, rusqlite::Error> {
        let started_at_str: String = row.get(13)?;
        let ended_at_str: Option<String> = row.get(14)?;

        Ok(Session {
            id: row.get(0)?,
            prompt: row.get(1)?,
            working_dir: PathBuf::from(row.get::<_, String>(2)?),
            actor_agent: row.get(3)?,
            critic_agent: row.get(4)?,
            actor_model: row.get(5)?,
            critic_model: row.get(6)?,
            max_iterations: row.get::<_, Option<i64>>(7)?.map(|n| n as usize),
            outcome: row.get(8)?,
            iteration_count: row.get::<_, Option<i64>>(9)?.map(|n| n as usize),
            summary: row.get(10)?,
            confidence: row.get(11)?,
            duration_secs: row.get(12)?,
            started_at: DateTime::parse_from_rfc3339(&started_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            ended_at: ended_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            }),
            iterations: Vec::new(), // Populated by get()
        })
    }

    fn row_to_summary(row: &rusqlite::Row) -> Result<SessionSummary, rusqlite::Error> {
        let prompt: String = row.get(1)?;
        let working_dir_str: String = row.get(2)?;
        let working_dir = PathBuf::from(&working_dir_str);
        let started_at_str: String = row.get(9)?;

        // Extract project name from working_dir
        let project = working_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Create prompt preview (first 100 chars)
        let prompt_preview = if prompt.len() > 100 {
            format!("{}...", &prompt[..100])
        } else {
            prompt
        };

        Ok(SessionSummary {
            id: row.get(0)?,
            timestamp: DateTime::parse_from_rfc3339(&started_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            prompt_preview,
            working_dir,
            project,
            outcome: row.get(5)?,
            iterations: row.get::<_, Option<i64>>(6)?.unwrap_or(0) as usize,
            duration_secs: row.get(7)?,
            confidence: row.get(8)?,
            actor_agent: row.get(3)?,
            critic_agent: row.get(4)?,
        })
    }

    fn compute_sessions_over_time(summaries: &[SessionSummary]) -> Vec<DayCount> {
        let mut day_counts: BTreeMap<String, usize> = BTreeMap::new();
        for s in summaries {
            let date = s.timestamp.format("%Y-%m-%d").to_string();
            *day_counts.entry(date).or_insert(0) += 1;
        }
        day_counts
            .into_iter()
            .map(|(date, count)| DayCount { date, count })
            .collect()
    }

    fn compute_by_project(summaries: &[SessionSummary]) -> Vec<ProjectStats> {
        let mut project_map: HashMap<String, (usize, usize)> = HashMap::new();
        for s in summaries {
            let entry = project_map.entry(s.project.clone()).or_insert((0, 0));
            entry.0 += 1;
            if s.outcome.as_deref() == Some("success") {
                entry.1 += 1;
            }
        }
        let mut by_project: Vec<ProjectStats> = project_map
            .into_iter()
            .map(|(project, (total, successes))| ProjectStats {
                project,
                total,
                success_rate: successes as f64 / total as f64,
            })
            .collect();
        by_project.sort_by(|a, b| b.total.cmp(&a.total));
        by_project
    }

    fn compute_critic_metrics(
        &self,
        summaries: &[SessionSummary],
    ) -> Result<(usize, f64, f64, f64), rusqlite::Error> {
        let mut total_iterations = 0usize;
        let mut approvals = 0usize;
        let mut rejections = 0usize;
        let mut total_feedback_length = 0usize;
        let mut improvement_count = 0usize;

        for summary in summaries {
            let iterations = self.get_iterations(&summary.id)?;
            total_iterations += iterations.len();

            for (i, iteration) in iterations.iter().enumerate() {
                let decision_lower = iteration.critic_decision.to_lowercase();
                let is_approved = decision_lower == "approve"
                    || decision_lower == "approved"
                    || decision_lower == "done";

                if is_approved {
                    approvals += 1;

                    if i > 0 {
                        let prev = &iterations[i - 1];
                        let prev_decision = prev.critic_decision.to_lowercase();
                        let prev_rejected = prev_decision != "approve"
                            && prev_decision != "approved"
                            && prev_decision != "done";
                        if prev_rejected {
                            improvement_count += 1;
                        }
                    }
                } else {
                    rejections += 1;
                    if let Some(feedback) = &iteration.feedback {
                        total_feedback_length += feedback.len();
                    }
                }
            }
        }

        let critic_approval_rate = if total_iterations > 0 {
            approvals as f64 / total_iterations as f64
        } else {
            0.0
        };

        let avg_feedback_length = if rejections > 0 {
            total_feedback_length as f64 / rejections as f64
        } else {
            0.0
        };

        let improvement_rate = if rejections > 0 {
            improvement_count as f64 / rejections as f64
        } else {
            0.0
        };

        Ok((
            total_iterations,
            critic_approval_rate,
            avg_feedback_length,
            improvement_rate,
        ))
    }
}
