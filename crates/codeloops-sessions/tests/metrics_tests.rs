use std::fs;

use codeloops_sessions::{SessionFilter, SessionStore};
use tempfile::TempDir;

/// Helper: create a temp directory for testing agentic metrics.
fn create_metrics_test_dir() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Session 1: Single iteration success (first-try success)
    // critic_decision = "approve" counts as approval
    let session1 = r#"{"type":"session_start","timestamp":"2026-01-20T10:00:00Z","prompt":"implement simple feature","working_dir":"/home/user/project-alpha","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":5}
{"type":"iteration","iteration_number":1,"actor_output":"done","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":30.0,"git_diff":"diff --git a/feat.rs","git_files_changed":1,"critic_decision":"approve","feedback":null,"timestamp":"2026-01-20T10:01:00Z"}
{"type":"session_end","outcome":"success","iterations":1,"summary":"Implemented feature","confidence":0.95,"duration_secs":60.0,"timestamp":"2026-01-20T10:01:30Z"}"#;
    fs::write(dir.path().join("20260120_100000_abc123.jsonl"), session1).unwrap();

    // Session 2: Multi-iteration success (2 iterations, second approved)
    // First iteration rejected with feedback, second approved = 1 improvement
    let session2 = r#"{"type":"session_start","timestamp":"2026-01-21T14:00:00Z","prompt":"refactor authentication","working_dir":"/home/user/project-alpha","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":5}
{"type":"iteration","iteration_number":1,"actor_output":"started refactor","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":15.0,"git_diff":"diff --git a/auth.rs","git_files_changed":2,"critic_decision":"continue","feedback":"needs error handling","timestamp":"2026-01-21T14:01:00Z"}
{"type":"iteration","iteration_number":2,"actor_output":"added error handling","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff --git a/auth.rs","git_files_changed":1,"critic_decision":"approved","feedback":null,"timestamp":"2026-01-21T14:02:00Z"}
{"type":"session_end","outcome":"success","iterations":2,"summary":"Refactored auth","confidence":0.9,"duration_secs":120.0,"timestamp":"2026-01-21T14:02:30Z"}"#;
    fs::write(dir.path().join("20260121_140000_def456.jsonl"), session2).unwrap();

    // Session 3: Failed session (waste)
    // All iterations rejected
    let session3 = r#"{"type":"session_start","timestamp":"2026-01-22T09:00:00Z","prompt":"add complex feature","working_dir":"/home/user/project-beta","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":3}
{"type":"iteration","iteration_number":1,"actor_output":"attempt 1","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":20.0,"git_diff":"diff --git a/complex.rs","git_files_changed":1,"critic_decision":"continue","feedback":"wrong approach entirely","timestamp":"2026-01-22T09:01:00Z"}
{"type":"iteration","iteration_number":2,"actor_output":"attempt 2","actor_stderr":"error","actor_exit_code":1,"actor_duration_secs":15.0,"git_diff":"","git_files_changed":0,"critic_decision":"fail","feedback":"still broken","timestamp":"2026-01-22T09:02:00Z"}
{"type":"session_end","outcome":"failed","iterations":2,"summary":null,"confidence":0.1,"duration_secs":180.0,"timestamp":"2026-01-22T09:03:00Z"}"#;
    fs::write(dir.path().join("20260122_090000_ghi789.jsonl"), session3).unwrap();

    // Session 4: Interrupted session (waste)
    let session4 = r#"{"type":"session_start","timestamp":"2026-01-23T10:00:00Z","prompt":"start something","working_dir":"/home/user/project-beta","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":5}
{"type":"iteration","iteration_number":1,"actor_output":"starting","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff --git a/start.rs","git_files_changed":1,"critic_decision":"continue","feedback":"keep going","timestamp":"2026-01-23T10:01:00Z"}
{"type":"session_end","outcome":"interrupted","iterations":1,"summary":null,"confidence":null,"duration_secs":30.0,"timestamp":"2026-01-23T10:01:30Z"}"#;
    fs::write(dir.path().join("20260123_100000_jkl012.jsonl"), session4).unwrap();

    dir
}

// ============================================================
// Agentic metrics tests - Empty state
// ============================================================

#[test]
fn test_agentic_metrics_empty_returns_zeros() {
    let dir = TempDir::new().unwrap();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    assert_eq!(metrics.total_sessions, 0);
    assert_eq!(metrics.successful_sessions, 0);
    assert_eq!(metrics.success_rate, 0.0);
    assert_eq!(metrics.first_try_success_rate, 0.0);
    assert_eq!(metrics.avg_iterations_to_success, 0.0);
    assert_eq!(metrics.avg_cycle_time_secs, 0.0);
    assert_eq!(metrics.waste_rate, 0.0);
    assert_eq!(metrics.total_iterations, 0);
    assert_eq!(metrics.critic_approval_rate, 0.0);
    assert_eq!(metrics.avg_feedback_length, 0.0);
    assert_eq!(metrics.improvement_rate, 0.0);
    assert!(metrics.sessions_over_time.is_empty());
    assert!(metrics.by_project.is_empty());
}

// ============================================================
// Agentic metrics tests - First-try success rate
// ============================================================

#[test]
fn test_first_try_success_rate_single_iteration_success() {
    let dir = TempDir::new().unwrap();

    // Single session with 1 iteration that succeeded
    let session = r#"{"type":"session_start","timestamp":"2026-01-20T10:00:00Z","prompt":"quick fix","working_dir":"/home/user/project","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":5}
{"type":"iteration","iteration_number":1,"actor_output":"fixed","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff","git_files_changed":1,"critic_decision":"approve","feedback":null,"timestamp":"2026-01-20T10:01:00Z"}
{"type":"session_end","outcome":"success","iterations":1,"summary":"Fixed","confidence":0.95,"duration_secs":30.0,"timestamp":"2026-01-20T10:01:30Z"}"#;
    fs::write(dir.path().join("session.jsonl"), session).unwrap();

    let store = SessionStore::with_dir(dir.path().to_path_buf());
    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // 1 successful session, 1 first-try success = 100%
    assert_eq!(metrics.successful_sessions, 1);
    assert_eq!(metrics.first_try_success_rate, 1.0);
}

#[test]
fn test_first_try_success_rate_multi_iteration_success() {
    let dir = TempDir::new().unwrap();

    // Session that took 3 iterations to succeed
    let session = r#"{"type":"session_start","timestamp":"2026-01-20T10:00:00Z","prompt":"complex task","working_dir":"/home/user/project","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":5}
{"type":"iteration","iteration_number":1,"actor_output":"try 1","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff","git_files_changed":1,"critic_decision":"continue","feedback":"needs more","timestamp":"2026-01-20T10:01:00Z"}
{"type":"iteration","iteration_number":2,"actor_output":"try 2","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff","git_files_changed":1,"critic_decision":"continue","feedback":"almost there","timestamp":"2026-01-20T10:02:00Z"}
{"type":"iteration","iteration_number":3,"actor_output":"try 3","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff","git_files_changed":1,"critic_decision":"approve","feedback":null,"timestamp":"2026-01-20T10:03:00Z"}
{"type":"session_end","outcome":"success","iterations":3,"summary":"Done","confidence":0.9,"duration_secs":90.0,"timestamp":"2026-01-20T10:03:30Z"}"#;
    fs::write(dir.path().join("session.jsonl"), session).unwrap();

    let store = SessionStore::with_dir(dir.path().to_path_buf());
    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // 1 successful session, but NOT first-try (3 iterations) = 0%
    assert_eq!(metrics.successful_sessions, 1);
    assert_eq!(metrics.first_try_success_rate, 0.0);
    assert_eq!(metrics.avg_iterations_to_success, 3.0);
}

#[test]
fn test_first_try_success_rate_mixed() {
    let dir = create_metrics_test_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // Total: 4 sessions
    // Successful: 2 (session1: 1 iter, session2: 2 iters)
    // First-try successes: 1 (session1 only)
    // First-try success rate = 1/2 = 50%
    assert_eq!(metrics.total_sessions, 4);
    assert_eq!(metrics.successful_sessions, 2);
    assert_eq!(metrics.first_try_success_rate, 0.5);
}

// ============================================================
// Agentic metrics tests - Waste rate
// ============================================================

#[test]
fn test_waste_rate_all_successful() {
    let dir = TempDir::new().unwrap();

    let session = r#"{"type":"session_start","timestamp":"2026-01-20T10:00:00Z","prompt":"task","working_dir":"/home/user/project","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":5}
{"type":"iteration","iteration_number":1,"actor_output":"done","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff","git_files_changed":1,"critic_decision":"approve","feedback":null,"timestamp":"2026-01-20T10:01:00Z"}
{"type":"session_end","outcome":"success","iterations":1,"summary":"Done","confidence":0.95,"duration_secs":30.0,"timestamp":"2026-01-20T10:01:30Z"}"#;
    fs::write(dir.path().join("session.jsonl"), session).unwrap();

    let store = SessionStore::with_dir(dir.path().to_path_buf());
    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // No waste - all successful
    assert_eq!(metrics.waste_rate, 0.0);
}

#[test]
fn test_waste_rate_mixed_outcomes() {
    let dir = create_metrics_test_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // Total: 4 sessions
    // Waste: session3 (failed) + session4 (interrupted) = 2
    // Waste rate = 2/4 = 50%
    assert_eq!(metrics.total_sessions, 4);
    assert_eq!(metrics.waste_rate, 0.5);
}

#[test]
fn test_waste_rate_includes_max_iterations_reached() {
    let dir = TempDir::new().unwrap();

    let session = r#"{"type":"session_start","timestamp":"2026-01-20T10:00:00Z","prompt":"task","working_dir":"/home/user/project","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":2}
{"type":"iteration","iteration_number":1,"actor_output":"try 1","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff","git_files_changed":1,"critic_decision":"continue","feedback":"more","timestamp":"2026-01-20T10:01:00Z"}
{"type":"iteration","iteration_number":2,"actor_output":"try 2","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff","git_files_changed":1,"critic_decision":"continue","feedback":"still more","timestamp":"2026-01-20T10:02:00Z"}
{"type":"session_end","outcome":"max_iterations_reached","iterations":2,"summary":null,"confidence":0.3,"duration_secs":60.0,"timestamp":"2026-01-20T10:02:30Z"}"#;
    fs::write(dir.path().join("session.jsonl"), session).unwrap();

    let store = SessionStore::with_dir(dir.path().to_path_buf());
    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // max_iterations_reached is counted as waste
    assert_eq!(metrics.waste_rate, 1.0);
}

// ============================================================
// Agentic metrics tests - Critic metrics
// ============================================================

#[test]
fn test_critic_approval_rate() {
    let dir = create_metrics_test_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // Iterations:
    // session1: 1 iteration (approve)     -> 1 approval
    // session2: 2 iterations (continue, approved) -> 1 approval
    // session3: 2 iterations (continue, fail)   -> 0 approvals
    // session4: 1 iteration (continue)    -> 0 approvals
    // Total: 6 iterations, 2 approvals
    // Approval rate = 2/6 = 0.333...
    assert_eq!(metrics.total_iterations, 6);
    assert!((metrics.critic_approval_rate - 2.0 / 6.0).abs() < 0.01);
}

#[test]
fn test_avg_feedback_length() {
    let dir = create_metrics_test_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // Feedback from rejections:
    // session2 iter1: "needs error handling" = 20 chars
    // session3 iter1: "wrong approach entirely" = 23 chars
    // session3 iter2: "still broken" = 12 chars
    // session4 iter1: "keep going" = 10 chars
    // Total feedback: 20 + 23 + 12 + 10 = 65 chars
    // Rejections: 4
    // Avg = 65/4 = 16.25
    assert!((metrics.avg_feedback_length - 16.25).abs() < 0.01);
}

#[test]
fn test_improvement_rate() {
    let dir = create_metrics_test_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // Improvement: when a rejection is followed by an approval
    // session1: only 1 iteration, no prior rejection -> 0 improvements
    // session2: iter1=rejected, iter2=approved -> 1 improvement
    // session3: iter1=rejected, iter2=rejected -> 0 improvements
    // session4: iter1=rejected, no iter2 -> 0 improvements
    // Total improvements: 1
    // Total rejections: 4
    // Improvement rate = 1/4 = 0.25
    assert!((metrics.improvement_rate - 0.25).abs() < 0.01);
}

#[test]
fn test_improvement_rate_all_approved() {
    let dir = TempDir::new().unwrap();

    // Session where all iterations are approved (unusual but possible)
    let session = r#"{"type":"session_start","timestamp":"2026-01-20T10:00:00Z","prompt":"task","working_dir":"/home/user/project","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":5}
{"type":"iteration","iteration_number":1,"actor_output":"done","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff","git_files_changed":1,"critic_decision":"approve","feedback":null,"timestamp":"2026-01-20T10:01:00Z"}
{"type":"session_end","outcome":"success","iterations":1,"summary":"Done","confidence":0.95,"duration_secs":30.0,"timestamp":"2026-01-20T10:01:30Z"}"#;
    fs::write(dir.path().join("session.jsonl"), session).unwrap();

    let store = SessionStore::with_dir(dir.path().to_path_buf());
    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // No rejections means improvement_rate is 0 (no division by zero)
    assert_eq!(metrics.critic_approval_rate, 1.0);
    assert_eq!(metrics.improvement_rate, 0.0);
    assert_eq!(metrics.avg_feedback_length, 0.0);
}

// ============================================================
// Agentic metrics tests - Average metrics
// ============================================================

#[test]
fn test_avg_iterations_to_success() {
    let dir = create_metrics_test_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // Successful sessions: session1 (1 iter), session2 (2 iters)
    // Avg = (1 + 2) / 2 = 1.5
    assert_eq!(metrics.avg_iterations_to_success, 1.5);
}

#[test]
fn test_avg_cycle_time_secs() {
    let dir = create_metrics_test_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // Successful sessions: session1 (60s), session2 (120s)
    // Avg = (60 + 120) / 2 = 90
    assert_eq!(metrics.avg_cycle_time_secs, 90.0);
}

// ============================================================
// Agentic metrics tests - Breakdowns
// ============================================================

#[test]
fn test_sessions_over_time() {
    let dir = create_metrics_test_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // 4 sessions on 4 different days
    assert_eq!(metrics.sessions_over_time.len(), 4);

    // BTreeMap sorts by date
    assert_eq!(metrics.sessions_over_time[0].date, "2026-01-20");
    assert_eq!(metrics.sessions_over_time[0].count, 1);
}

#[test]
fn test_by_project() {
    let dir = create_metrics_test_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // 2 projects: project-alpha (2 sessions), project-beta (2 sessions)
    assert_eq!(metrics.by_project.len(), 2);

    // Sorted by total descending (or alphabetically if equal)
    let alpha = metrics
        .by_project
        .iter()
        .find(|p| p.project == "project-alpha")
        .unwrap();
    assert_eq!(alpha.total, 2);
    assert_eq!(alpha.success_rate, 1.0); // both successful

    let beta = metrics
        .by_project
        .iter()
        .find(|p| p.project == "project-beta")
        .unwrap();
    assert_eq!(beta.total, 2);
    assert_eq!(beta.success_rate, 0.0); // both failed/interrupted
}

// ============================================================
// Agentic metrics tests - Edge cases
// ============================================================

#[test]
fn test_metrics_with_active_session() {
    let dir = TempDir::new().unwrap();

    // Active session (no session_end)
    let session = r#"{"type":"session_start","timestamp":"2026-01-20T10:00:00Z","prompt":"in progress","working_dir":"/home/user/project","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":5}
{"type":"iteration","iteration_number":1,"actor_output":"working","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff","git_files_changed":1,"critic_decision":"continue","feedback":"keep going","timestamp":"2026-01-20T10:01:00Z"}"#;
    fs::write(dir.path().join("session.jsonl"), session).unwrap();

    let store = SessionStore::with_dir(dir.path().to_path_buf());
    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // Active session counted in total but not successful
    assert_eq!(metrics.total_sessions, 1);
    assert_eq!(metrics.successful_sessions, 0);
    assert_eq!(metrics.success_rate, 0.0);
    // Not counted as waste either (outcome is None, not failed/interrupted/max_iter)
    assert_eq!(metrics.waste_rate, 0.0);
}

#[test]
fn test_metrics_with_filter() {
    let dir = create_metrics_test_dir();
    let store = SessionStore::with_dir(dir.path().to_path_buf());

    let filter = SessionFilter {
        outcome: Some("success".to_string()),
        ..Default::default()
    };
    let metrics = store.agentic_metrics(&filter).unwrap();

    // Only successful sessions
    assert_eq!(metrics.total_sessions, 2);
    assert_eq!(metrics.successful_sessions, 2);
    assert_eq!(metrics.success_rate, 1.0);
    assert_eq!(metrics.waste_rate, 0.0);
}

#[test]
fn test_metrics_case_insensitive_approval() {
    let dir = TempDir::new().unwrap();

    // Test different case variations of "approve"/"approved"
    let session = r#"{"type":"session_start","timestamp":"2026-01-20T10:00:00Z","prompt":"task","working_dir":"/home/user/project","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":5}
{"type":"iteration","iteration_number":1,"actor_output":"try 1","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff","git_files_changed":1,"critic_decision":"APPROVE","feedback":null,"timestamp":"2026-01-20T10:01:00Z"}
{"type":"session_end","outcome":"success","iterations":1,"summary":"Done","confidence":0.95,"duration_secs":30.0,"timestamp":"2026-01-20T10:01:30Z"}"#;
    fs::write(dir.path().join("session1.jsonl"), session).unwrap();

    let session2 = r#"{"type":"session_start","timestamp":"2026-01-21T10:00:00Z","prompt":"task2","working_dir":"/home/user/project","actor_agent":"claude","critic_agent":"claude","actor_model":"sonnet","critic_model":"sonnet","max_iterations":5}
{"type":"iteration","iteration_number":1,"actor_output":"try 1","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":10.0,"git_diff":"diff","git_files_changed":1,"critic_decision":"Approved","feedback":null,"timestamp":"2026-01-21T10:01:00Z"}
{"type":"session_end","outcome":"success","iterations":1,"summary":"Done","confidence":0.95,"duration_secs":30.0,"timestamp":"2026-01-21T10:01:30Z"}"#;
    fs::write(dir.path().join("session2.jsonl"), session2).unwrap();

    let store = SessionStore::with_dir(dir.path().to_path_buf());
    let metrics = store.agentic_metrics(&SessionFilter::default()).unwrap();

    // Both should be counted as approvals (case-insensitive)
    assert_eq!(metrics.total_iterations, 2);
    assert_eq!(metrics.critic_approval_rate, 1.0);
}
