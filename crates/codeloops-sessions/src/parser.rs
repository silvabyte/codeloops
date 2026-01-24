use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{Context, Result};

use crate::types::{Iteration, Session, SessionEnd, SessionLine, SessionStart, SessionSummary};

/// Parse a single JSONL file into a fully-loaded Session.
pub fn parse_session(path: &Path) -> Result<Session> {
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let file = File::open(path).with_context(|| format!("Failed to open session file: {:?}", path))?;
    let reader = BufReader::new(file);

    let mut start: Option<SessionStart> = None;
    let mut iterations: Vec<Iteration> = Vec::new();
    let mut end: Option<SessionEnd> = None;

    for line in reader.lines() {
        let line = line.with_context(|| "Failed to read line from session file")?;
        if line.trim().is_empty() {
            continue;
        }

        let session_line: SessionLine = serde_json::from_str(&line)
            .with_context(|| format!("Failed to parse session line: {}", &line[..line.len().min(100)]))?;

        match session_line {
            SessionLine::SessionStart(s) => start = Some(s),
            SessionLine::Iteration(i) => iterations.push(i),
            SessionLine::SessionEnd(e) => end = Some(e),
        }
    }

    let start = start.with_context(|| "Session file missing session_start line")?;

    Ok(Session {
        id,
        start,
        iterations,
        end,
    })
}

/// Parse just enough of the file to produce a SessionSummary (fast listing).
/// Reads the first line for start info, then seeks to the end for completion status.
pub fn parse_session_summary(path: &Path) -> Result<SessionSummary> {
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut file = File::open(path).with_context(|| format!("Failed to open session file: {:?}", path))?;

    // Read first line for session_start
    let mut first_line = String::new();
    let mut reader = BufReader::new(&mut file);
    reader.read_line(&mut first_line)?;

    let start_line: SessionLine = serde_json::from_str(first_line.trim())
        .with_context(|| "Failed to parse first line as session_start")?;

    let start = match start_line {
        SessionLine::SessionStart(s) => s,
        _ => anyhow::bail!("First line of session file is not session_start"),
    };

    // Read the last line to check for session_end
    drop(reader);
    let last_line = read_last_line(&mut file)?;

    let (outcome, iteration_count, duration, confidence) = if let Some(ref last) = last_line {
        match serde_json::from_str::<SessionLine>(last.trim()) {
            Ok(SessionLine::SessionEnd(e)) => (
                Some(e.outcome),
                e.iterations,
                Some(e.duration_secs),
                e.confidence,
            ),
            Ok(SessionLine::Iteration(i)) => {
                // Session still active — count iterations by counting lines minus 1 (the start line)
                let line_count = count_lines(path)?;
                (None, line_count.saturating_sub(1), Some(i.actor_duration_secs), None)
            }
            _ => (None, 0, None, None),
        }
    } else {
        (None, 0, None, None)
    };

    let prompt_preview = if start.prompt.len() > 100 {
        format!("{}...", &start.prompt[..100])
    } else {
        start.prompt.clone()
    };

    let project = start
        .working_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(SessionSummary {
        id,
        timestamp: start.timestamp,
        prompt_preview,
        working_dir: start.working_dir,
        project,
        outcome,
        iterations: iteration_count,
        duration_secs: duration,
        confidence,
        actor_agent: start.actor_agent,
        critic_agent: start.critic_agent,
    })
}

/// Read the last non-empty line of a file by seeking from the end.
fn read_last_line(file: &mut File) -> Result<Option<String>> {
    let file_len = file.seek(SeekFrom::End(0))?;
    if file_len == 0 {
        return Ok(None);
    }

    // Read up to the last 64KB to find the last line
    let read_size = file_len.min(65536);
    file.seek(SeekFrom::End(-(read_size as i64)))?;

    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    let last = buf
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .map(|s| s.to_string());

    Ok(last)
}

/// Count lines in a file (for estimating iteration count of active sessions).
fn count_lines(path: &Path) -> Result<usize> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(reader.lines().count())
}
