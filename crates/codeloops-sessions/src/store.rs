use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::parser::{parse_session, parse_session_summary};
use crate::types::{DayCount, ProjectStats, Session, SessionFilter, SessionStats, SessionSummary};

/// Provides access to session files on disk.
pub struct SessionStore {
    sessions_dir: PathBuf,
}

impl SessionStore {
    /// Create a new SessionStore using the default sessions directory.
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_dir().with_context(|| "Could not determine data directory")?;
        let sessions_dir = data_dir.join("codeloops").join("sessions");
        Ok(Self { sessions_dir })
    }

    /// Create a SessionStore with a custom directory (useful for testing).
    pub fn with_dir(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }

    /// Return the sessions directory path.
    pub fn sessions_dir(&self) -> &PathBuf {
        &self.sessions_dir
    }

    /// List sessions matching the given filter, sorted by timestamp descending.
    pub fn list(&self, filter: &SessionFilter) -> Result<Vec<SessionSummary>> {
        if !self.sessions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut summaries: Vec<SessionSummary> = Vec::new();

        let entries = std::fs::read_dir(&self.sessions_dir)
            .with_context(|| format!("Failed to read sessions dir: {:?}", self.sessions_dir))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                continue;
            }

            match parse_session_summary(&path) {
                Ok(summary) => {
                    if self.matches_filter(&summary, filter) {
                        summaries.push(summary);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse session {:?}: {}", path, e);
                }
            }
        }

        // Sort by timestamp descending (newest first)
        summaries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(summaries)
    }

    /// Get a fully parsed session by ID.
    pub fn get(&self, id: &str) -> Result<Session> {
        let path = self.sessions_dir.join(format!("{}.jsonl", id));
        parse_session(&path)
    }

    /// Get concatenated diffs from all iterations of a session.
    pub fn get_diff(&self, id: &str) -> Result<String> {
        let session = self.get(id)?;
        let diffs: Vec<&str> = session
            .iterations
            .iter()
            .filter(|i| !i.git_diff.is_empty())
            .map(|i| i.git_diff.as_str())
            .collect();

        Ok(diffs.join("\n"))
    }

    /// Compute aggregate statistics over sessions matching the filter.
    pub fn stats(&self, filter: &SessionFilter) -> Result<SessionStats> {
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

        // Sessions over time (group by date)
        let mut day_counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for s in &summaries {
            let date = s.timestamp.format("%Y-%m-%d").to_string();
            *day_counts.entry(date).or_insert(0) += 1;
        }
        let sessions_over_time: Vec<DayCount> = day_counts
            .into_iter()
            .map(|(date, count)| DayCount { date, count })
            .collect();

        // By project
        let mut project_map: std::collections::HashMap<String, (usize, usize)> =
            std::collections::HashMap::new();
        for s in &summaries {
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

        Ok(SessionStats {
            total_sessions,
            success_rate,
            avg_iterations,
            avg_duration_secs,
            sessions_over_time,
            by_project,
        })
    }

    /// Return IDs of active sessions (those without a session_end line).
    pub fn active_sessions(&self) -> Result<Vec<String>> {
        let summaries = self.list(&SessionFilter::default())?;
        Ok(summaries
            .into_iter()
            .filter(|s| s.outcome.is_none())
            .map(|s| s.id)
            .collect())
    }

    fn matches_filter(&self, summary: &SessionSummary, filter: &SessionFilter) -> bool {
        if let Some(ref outcome) = filter.outcome {
            match summary.outcome.as_deref() {
                Some(o) if o == outcome => {}
                _ => return false,
            }
        }

        if let Some(after) = filter.after {
            if summary.timestamp < after {
                return false;
            }
        }

        if let Some(before) = filter.before {
            if summary.timestamp > before {
                return false;
            }
        }

        if let Some(ref search) = filter.search {
            let search_lower = search.to_lowercase();
            if !summary
                .prompt_preview
                .to_lowercase()
                .contains(&search_lower)
            {
                return false;
            }
        }

        if let Some(ref project) = filter.project {
            if summary.project != *project {
                return false;
            }
        }

        true
    }
}
