use std::fs;
use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use codeloops_sessions::{
    parse_session, parse_session_summary, SessionFilter, SessionStore,
};
use tempfile::TempDir;

/// Helper: create a temp directory with session JSONL files.
fn create_test_sessions_dir() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Session 1: completed successfully with 2 iterations
    let session1 = r#"{"type":"session_start","timestamp":"2026-01-20T10:00:00Z","prompt":"fix the auth bug in login flow","working_dir":"/home/user/project-alpha","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":5}
{"type":"iteration","iteration_number":1,"actor_output":"fixed auth","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":12.5,"git_diff":"diff --git a/auth.rs\n+fixed","git_files_changed":1,"critic_decision":"CONTINUE","feedback":"needs more tests","timestamp":"2026-01-20T10:01:00Z"}
{"type":"iteration","iteration_number":2,"actor_output":"added tests","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":8.3,"git_diff":"diff --git a/auth_test.rs\n+test","git_files_changed":1,"critic_decision":"DONE","feedback":null,"timestamp":"2026-01-20T10:02:00Z"}
{"type":"session_end","outcome":"success","iterations":2,"summary":"Fixed auth bug and added tests","confidence":0.95,"duration_secs":120.0,"timestamp":"2026-01-20T10:02:30Z"}"#;
    fs::write(
        dir.path().join("20260120_100000_abc123.jsonl"),
        session1,
    )
    .unwrap();

    // Session 2: failed with 3 iterations
    let session2 = r#"{"type":"session_start","timestamp":"2026-01-21T14:00:00Z","prompt":"refactor database connection pooling","working_dir":"/home/user/project-beta","actor_agent":"claude","critic_agent":"gpt4","actor_model":"opus","critic_model":null,"max_iterations":5}
{"type":"iteration","iteration_number":1,"actor_output":"started refactor","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":15.0,"git_diff":"diff --git a/db.rs","git_files_changed":2,"critic_decision":"CONTINUE","feedback":"incomplete","timestamp":"2026-01-21T14:01:00Z"}
{"type":"iteration","iteration_number":2,"actor_output":"more changes","actor_stderr":"warning: unused var","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff --git a/pool.rs","git_files_changed":1,"critic_decision":"CONTINUE","feedback":"still issues","timestamp":"2026-01-21T14:02:00Z"}
{"type":"iteration","iteration_number":3,"actor_output":"final attempt","actor_stderr":"error: compile failed","actor_exit_code":1,"actor_duration_secs":5.0,"git_diff":"","git_files_changed":0,"critic_decision":"FAIL","feedback":"compilation errors","timestamp":"2026-01-21T14:03:00Z"}
{"type":"session_end","outcome":"failed","iterations":3,"summary":null,"confidence":0.2,"duration_secs":180.0,"timestamp":"2026-01-21T14:03:30Z"}"#;
    fs::write(
        dir.path().join("20260121_140000_def456.jsonl"),
        session2,
    )
    .unwrap();

    // Session 3: active (no session_end)
    let session3 = r#"{"type":"session_start","timestamp":"2026-01-22T09:00:00Z","prompt":"add user metrics dashboard","working_dir":"/home/user/project-alpha","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":10}
{"type":"iteration","iteration_number":1,"actor_output":"scaffolded dashboard","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":20.0,"git_diff":"diff --git a/dashboard.tsx","git_files_changed":3,"critic_decision":"CONTINUE","feedback":"good start, add charts","timestamp":"2026-01-22T09:01:00Z"}"#;
    fs::write(
        dir.path().join("20260122_090000_ghi789.jsonl"),
        session3,
    )
    .unwrap();

    // Session 4: success, same project as session 1 (project-alpha)
    let session4 = r#"{"type":"session_start","timestamp":"2026-01-23T16:00:00Z","prompt":"add documentation for the API endpoints","working_dir":"/home/user/project-alpha","actor_agent":"claude","critic_agent":"claude","actor_model":"haiku","critic_model":"haiku","max_iterations":3}
{"type":"iteration","iteration_number":1,"actor_output":"wrote docs","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":30.0,"git_diff":"diff --git a/docs/api.md","git_files_changed":1,"critic_decision":"DONE","feedback":null,"timestamp":"2026-01-23T16:01:00Z"}
{"type":"session_end","outcome":"success","iterations":1,"summary":"Added API docs","confidence":0.9,"duration_secs":60.0,"timestamp":"2026-01-23T16:01:30Z"}"#;
    fs::write(
        dir.path().join("20260123_160000_jkl012.jsonl"),
        session4,
    )
    .unwrap();

    dir
}

// ============================================================
// Parser tests
// ============================================================

#[test]
fn test_parse_session_complete() {
    let dir = create_test_sessions_dir();
    let path = dir.path().join("20260120_100000_abc123.jsonl");

    let session = parse_session(&path).unwrap();

    assert_eq!(session.id, "20260120_100000_abc123");
    assert_eq!(session.start.prompt, "fix the auth bug in login flow");
    assert_eq!(
        session.start.working_dir,
        PathBuf::from("/home/user/project-alpha")
    );
    assert_eq!(session.start.actor_agent, "claude");
    assert_eq!(session.start.critic_agent, "claude");
    assert_eq!(session.start.actor_model, Some("sonnet".to_string()));
    assert_eq!(session.start.max_iterations, Some(5));
    assert_eq!(session.iterations.len(), 2);
    assert!(session.end.is_some());

    let end = session.end.unwrap();
    assert_eq!(end.outcome, "success");
    assert_eq!(end.iterations, 2);
    assert_eq!(end.confidence, Some(0.95));
    assert_eq!(end.duration_secs, 120.0);
}

#[test]
fn test_parse_session_iterations() {
    let dir = create_test_sessions_dir();
    let path = dir.path().join("20260121_140000_def456.jsonl");

    let session = parse_session(&path).unwrap();

    assert_eq!(session.iterations.len(), 3);

    // First iteration
    assert_eq!(session.iterations[0].iteration_number, 1);
    assert_eq!(session.iterations[0].actor_exit_code, 0);
    assert_eq!(session.iterations[0].critic_decision, "CONTINUE");
    assert_eq!(
        session.iterations[0].feedback,
        Some("incomplete".to_string())
    );

    // Last iteration (failed)
    assert_eq!(session.iterations[2].iteration_number, 3);
    assert_eq!(session.iterations[2].actor_exit_code, 1);
    assert_eq!(session.iterations[2].critic_decision, "FAIL");
    assert_eq!(session.iterations[2].git_files_changed, 0);
}

#[test]
fn test_parse_session_active_no_end() {
    let dir = create_test_sessions_dir();
    let path = dir.path().join("20260122_090000_ghi789.jsonl");

    let session = parse_session(&path).unwrap();

    assert_eq!(session.id, "20260122_090000_ghi789");
    assert_eq!(session.start.prompt, "add user metrics dashboard");
    assert_eq!(session.iterations.len(), 1);
    assert!(session.end.is_none());
}

#[test]
fn test_parse_session_summary_complete() {
    let dir = create_test_sessions_dir();
    let path = dir.path().join("20260120_100000_abc123.jsonl");

    let summary = parse_session_summary(&path).unwrap();

    assert_eq!(summary.id, "20260120_100000_abc123");
    assert_eq!(summary.prompt_preview, "fix the auth bug in login flow");
    assert_eq!(summary.project, "project-alpha");
    assert_eq!(summary.outcome, Some("success".to_string()));
    assert_eq!(summary.iterations, 2);
    assert_eq!(summary.duration_secs, Some(120.0));
    assert_eq!(summary.confidence, Some(0.95));
    assert_eq!(summary.actor_agent, "claude");
    assert_eq!(summary.critic_agent, "claude");
}

#[test]
fn test_parse_session_summary_active() {
    let dir = create_test_sessions_dir();
    let path = dir.path().join("20260122_090000_ghi789.jsonl");

    let summary = parse_session_summary(&path).unwrap();

    assert_eq!(summary.id, "20260122_090000_ghi789");
    assert_eq!(summary.outcome, None);
    assert_eq!(summary.project, "project-alpha");
}

#[test]
fn test_parse_session_summary_truncates_long_prompt() {
    let dir = TempDir::new().unwrap();
    let long_prompt = "x".repeat(200);
    let session = format!(
        r#"{{"type":"session_start","timestamp":"2026-01-20T10:00:00Z","prompt":"{}","working_dir":"/home/user/myproject","actor_agent":"claude","critic_agent":"claude","actor_model":null,"critic_model":null,"max_iterations":5}}"#,
        long_prompt
    );
    fs::write(dir.path().join("long_prompt.jsonl"), session).unwrap();

    let summary = parse_session_summary(&dir.path().join("long_prompt.jsonl")).unwrap();

    assert_eq!(summary.prompt_preview.len(), 103); // 100 chars + "..."
    assert!(summary.prompt_preview.ends_with("..."));
}

// ============================================================
// Store tests
// ============================================================

#[test]
fn test_store_list_all() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let summaries = store.list(&SessionFilter::default()).unwrap();

    assert_eq!(summaries.len(), 4);
    // Should be sorted by timestamp descending
    assert_eq!(summaries[0].id, "20260123_160000_jkl012");
    assert_eq!(summaries[1].id, "20260122_090000_ghi789");
    assert_eq!(summaries[2].id, "20260121_140000_def456");
    assert_eq!(summaries[3].id, "20260120_100000_abc123");
}

#[test]
fn test_store_filter_by_outcome() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let filter = SessionFilter {
        outcome: Some("success".to_string()),
        ..Default::default()
    };
    let summaries = store.list(&filter).unwrap();

    assert_eq!(summaries.len(), 2);
    assert!(summaries.iter().all(|s| s.outcome.as_deref() == Some("success")));

    let filter = SessionFilter {
        outcome: Some("failed".to_string()),
        ..Default::default()
    };
    let summaries = store.list(&filter).unwrap();

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].id, "20260121_140000_def456");
}

#[test]
fn test_store_filter_by_date_range() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let filter = SessionFilter {
        after: Some(Utc.with_ymd_and_hms(2026, 1, 21, 0, 0, 0).unwrap()),
        before: Some(Utc.with_ymd_and_hms(2026, 1, 22, 23, 59, 59).unwrap()),
        ..Default::default()
    };
    let summaries = store.list(&filter).unwrap();

    assert_eq!(summaries.len(), 2);
    // Sessions from Jan 21 and Jan 22 only
    assert!(summaries.iter().any(|s| s.id == "20260121_140000_def456"));
    assert!(summaries.iter().any(|s| s.id == "20260122_090000_ghi789"));
}

#[test]
fn test_store_filter_by_search() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let filter = SessionFilter {
        search: Some("auth".to_string()),
        ..Default::default()
    };
    let summaries = store.list(&filter).unwrap();

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].id, "20260120_100000_abc123");
}

#[test]
fn test_store_filter_by_search_case_insensitive() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let filter = SessionFilter {
        search: Some("DATABASE".to_string()),
        ..Default::default()
    };
    let summaries = store.list(&filter).unwrap();

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].id, "20260121_140000_def456");
}

#[test]
fn test_store_filter_by_project() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let filter = SessionFilter {
        project: Some("project-alpha".to_string()),
        ..Default::default()
    };
    let summaries = store.list(&filter).unwrap();

    assert_eq!(summaries.len(), 3); // sessions 1, 3, 4
    assert!(summaries.iter().all(|s| s.project == "project-alpha"));
}

#[test]
fn test_store_filter_combined() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let filter = SessionFilter {
        outcome: Some("success".to_string()),
        project: Some("project-alpha".to_string()),
        ..Default::default()
    };
    let summaries = store.list(&filter).unwrap();

    assert_eq!(summaries.len(), 2); // sessions 1 and 4
}

#[test]
fn test_store_get_session() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let session = store.get("20260120_100000_abc123").unwrap();

    assert_eq!(session.id, "20260120_100000_abc123");
    assert_eq!(session.iterations.len(), 2);
    assert!(session.end.is_some());
}

#[test]
fn test_store_get_diff() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let diff = store.get_diff("20260120_100000_abc123").unwrap();

    assert!(diff.contains("diff --git a/auth.rs"));
    assert!(diff.contains("diff --git a/auth_test.rs"));
}

#[test]
fn test_store_get_diff_excludes_empty() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    // Session 2 has an empty diff on iteration 3
    let diff = store.get_diff("20260121_140000_def456").unwrap();

    assert!(diff.contains("diff --git a/db.rs"));
    assert!(diff.contains("diff --git a/pool.rs"));
    // Should only have 2 diff sections, not 3
    assert_eq!(diff.matches("diff --git").count(), 2);
}

#[test]
fn test_store_active_sessions() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let active = store.active_sessions().unwrap();

    assert_eq!(active.len(), 1);
    assert_eq!(active[0], "20260122_090000_ghi789");
}

#[test]
fn test_store_list_empty_dir() {
    let dir = TempDir::new().unwrap();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let summaries = store.list(&SessionFilter::default()).unwrap();
    assert!(summaries.is_empty());
}

#[test]
fn test_store_list_nonexistent_dir() {
    let store = SessionStore::with_dir(PathBuf::from("/nonexistent/path/sessions"));

    let summaries = store.list(&SessionFilter::default()).unwrap();
    assert!(summaries.is_empty());
}

// ============================================================
// Stats tests
// ============================================================

#[test]
fn test_stats_totals() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let stats = store.stats(&SessionFilter::default()).unwrap();

    assert_eq!(stats.total_sessions, 4);
    assert_eq!(stats.success_rate, 0.5); // 2 out of 4 (active counts as non-success)
}

#[test]
fn test_stats_averages() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let stats = store.stats(&SessionFilter::default()).unwrap();

    // Avg iterations: (2 + 3 + 1 + 1) / 4 = 1.75
    assert!((stats.avg_iterations - 1.75).abs() < 0.01);

    // Avg duration: only sessions with duration_secs are counted
    // Sessions 1: 120, 2: 180, 3: 20.0 (from iteration), 4: 60
    assert!(stats.avg_duration_secs > 0.0);
}

#[test]
fn test_stats_by_project() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let stats = store.stats(&SessionFilter::default()).unwrap();

    assert_eq!(stats.by_project.len(), 2); // project-alpha and project-beta

    let alpha = stats
        .by_project
        .iter()
        .find(|p| p.project == "project-alpha")
        .unwrap();
    assert_eq!(alpha.total, 3); // sessions 1, 3, 4
    // 2 successes out of 3 (session 3 is active)
    assert!((alpha.success_rate - 2.0 / 3.0).abs() < 0.01);

    let beta = stats
        .by_project
        .iter()
        .find(|p| p.project == "project-beta")
        .unwrap();
    assert_eq!(beta.total, 1);
    assert_eq!(beta.success_rate, 0.0);
}

#[test]
fn test_stats_sessions_over_time() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let stats = store.stats(&SessionFilter::default()).unwrap();

    assert_eq!(stats.sessions_over_time.len(), 4); // 4 different days
    // DayCounts are sorted by date (BTreeMap)
    assert_eq!(stats.sessions_over_time[0].date, "2026-01-20");
    assert_eq!(stats.sessions_over_time[0].count, 1);
}

#[test]
fn test_stats_empty() {
    let dir = TempDir::new().unwrap();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let stats = store.stats(&SessionFilter::default()).unwrap();

    assert_eq!(stats.total_sessions, 0);
    assert_eq!(stats.success_rate, 0.0);
    assert_eq!(stats.avg_iterations, 0.0);
    assert_eq!(stats.avg_duration_secs, 0.0);
    assert!(stats.sessions_over_time.is_empty());
    assert!(stats.by_project.is_empty());
}

#[test]
fn test_stats_with_filter() {
    let dir = create_test_sessions_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let filter = SessionFilter {
        outcome: Some("success".to_string()),
        ..Default::default()
    };
    let stats = store.stats(&filter).unwrap();

    assert_eq!(stats.total_sessions, 2);
    assert_eq!(stats.success_rate, 1.0); // all filtered are successes
}

// ============================================================
// Edge cases
// ============================================================

#[test]
fn test_parse_session_skips_empty_lines() {
    let dir = TempDir::new().unwrap();
    let content = r#"{"type":"session_start","timestamp":"2026-01-20T10:00:00Z","prompt":"test","working_dir":"/tmp","actor_agent":"a","critic_agent":"c","actor_model":null,"critic_model":null,"max_iterations":5}

{"type":"iteration","iteration_number":1,"actor_output":"out","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":1.0,"git_diff":"","git_files_changed":0,"critic_decision":"DONE","feedback":null,"timestamp":"2026-01-20T10:01:00Z"}

{"type":"session_end","outcome":"success","iterations":1,"summary":null,"confidence":null,"duration_secs":10.0,"timestamp":"2026-01-20T10:01:10Z"}"#;
    fs::write(dir.path().join("empty_lines.jsonl"), content).unwrap();

    let session = parse_session(&dir.path().join("empty_lines.jsonl")).unwrap();

    assert_eq!(session.iterations.len(), 1);
    assert!(session.end.is_some());
}

#[test]
fn test_parse_session_missing_start_line() {
    let dir = TempDir::new().unwrap();
    let content = r#"{"type":"iteration","iteration_number":1,"actor_output":"out","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":1.0,"git_diff":"","git_files_changed":0,"critic_decision":"DONE","feedback":null,"timestamp":"2026-01-20T10:01:00Z"}"#;
    fs::write(dir.path().join("no_start.jsonl"), content).unwrap();

    let result = parse_session(&dir.path().join("no_start.jsonl"));
    assert!(result.is_err());
}

#[test]
fn test_store_ignores_non_jsonl_files() {
    let dir = create_test_sessions_dir();
    // Write a non-jsonl file
    fs::write(dir.path().join("notes.txt"), "not a session").unwrap();
    fs::write(dir.path().join("data.json"), "{}").unwrap();

    let store = SessionStore::with_dir(dir.path().to_path_buf());
    let summaries = store.list(&SessionFilter::default()).unwrap();

    // Should still be 4, ignoring the txt and json files
    assert_eq!(summaries.len(), 4);
}
