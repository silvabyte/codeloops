//! Coordinates observations while the collector holds the shared outbox lock.
//! Queue replay consumes saved links and never invokes this module.
use crate::{AppResult, config::Config};
use rusqlite::{OptionalExtension, Transaction, params};
use session_history::{History, model::CheckpointLink};
use std::path::Path;

pub enum Boundary<'a> {
    Reuse,
    Prompt,
    Before(&'a str),
    After { call: &'a str, late: bool },
    Turn,
}

pub fn observe(
    config: &Config,
    tx: &Transaction<'_>,
    source: &str,
    session: &str,
    workspace: &str,
    directory: Option<&str>,
    boundary: Boundary<'_>,
) -> AppResult<CheckpointLink> {
    let key = serde_json::to_string(&(source, session, workspace))?;
    let previous: Option<String> = tx
        .query_row(
            "SELECT link FROM snapshot_state WHERE scope=?",
            [&key],
            |r| r.get(0),
        )
        .optional()?;
    let mut link = previous
        .as_deref()
        .map(serde_json::from_str::<CheckpointLink>)
        .transpose()?
        .unwrap_or(CheckpointLink {
            workspace_id: workspace.into(),
            checkpoint_id: None,
            before_id: None,
            baseline_id: None,
            baseline_status: "missing".into(),
            status: "missing".into(),
            reason: None,
            concurrent_tools: false,
        });
    if matches!(boundary, Boundary::Reuse) {
        link.status = if link.checkpoint_id.is_some() {
            "reused"
        } else {
            "missing"
        }
        .into();
        link.before_id = None;
        return Ok(link);
    }
    if matches!(boundary, Boundary::Prompt) && link.checkpoint_id.is_some() {
        link.status = "reused".into();
        link.before_id = None;
        return Ok(link);
    }
    let pre = matches!(boundary, Boundary::Prompt | Boundary::Before(_));
    link.concurrent_tools = false;
    match &boundary {
        Boundary::Before(call) => {
            let active: bool = tx.query_row(
                "SELECT EXISTS(
                     SELECT 1 FROM tool_activity
                     WHERE workspace = ? AND NOT(scope = ? AND call = ?)
                 )",
                params![workspace, key, call],
                |r| r.get(0),
            )?;
            if active {
                tx.execute(
                    "UPDATE tool_activity SET overlap=1 WHERE workspace=?",
                    [workspace],
                )?;
            }
            tx.execute(
                "INSERT INTO tool_activity VALUES(?, ?, ?, ?)
                 ON CONFLICT(scope, call) DO UPDATE
                 SET overlap = MAX(overlap, excluded.overlap)",
                params![key, call, workspace, active],
            )?;
            link.concurrent_tools = active;
        }
        Boundary::After { call, .. } => {
            link.concurrent_tools = tx
                .query_row(
                    "SELECT overlap FROM tool_activity WHERE scope=? AND call=?",
                    params![key, call],
                    |r| r.get::<_, bool>(0),
                )
                .optional()?
                .unwrap_or(false);
            tx.execute(
                "DELETE FROM tool_activity WHERE scope=? AND call=?",
                params![key, call],
            )?;
        }
        Boundary::Turn => {
            link.concurrent_tools = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM tool_activity WHERE workspace=?)",
                [workspace],
                |r| r.get(0),
            )?;
        }
        _ => {}
    }
    let late = matches!(
        boundary,
        Boundary::After { late: true, .. } | Boundary::Turn
    );
    link.before_id = match &boundary {
        Boundary::After { call, .. } => tx
            .query_row(
                "SELECT checkpoint FROM tool_boundaries WHERE scope=? AND call=?",
                params![key, call],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten(),
        Boundary::Turn => link.checkpoint_id.clone(),
        _ => None,
    };
    let captured = (|| -> AppResult<serde_json::Value> {
        let directory = directory.ok_or("source did not expose a workspace root")?;
        Ok(History::open(config.archive())?.checkpoint(Path::new(directory), workspace)?)
    })();
    match captured {
        Ok(checkpoint) => {
            link.checkpoint_id = checkpoint["id"].as_str().map(String::from);
            link.status = if late { "late" } else { "fresh" }.into();
            link.reason = if late {
                Some("source callback is not an awaited completion boundary".into())
            } else {
                None
            };
            if link.baseline_id.is_none() {
                link.baseline_id = link.checkpoint_id.clone();
                link.baseline_status = if pre && previous.is_none() {
                    "pre_action"
                } else {
                    "late"
                }
                .into();
            }
            if matches!(boundary, Boundary::After { .. }) && link.before_id.is_none() {
                link.reason = Some(
                    "tool pre-action checkpoint missing; event-local comparison unavailable".into(),
                );
            }
        }
        Err(error) => {
            link.checkpoint_id = None;
            link.status = "failed".into();
            link.reason = Some(error.to_string());
        }
    }
    if let Boundary::Before(call) = boundary {
        tx.execute(
            "INSERT INTO tool_boundaries VALUES(?, ?, ?)
             ON CONFLICT(scope, call) DO UPDATE SET checkpoint = excluded.checkpoint",
            params![key, call, link.checkpoint_id],
        )?;
    }
    tx.execute(
        "INSERT INTO snapshot_state VALUES(?, ?)
         ON CONFLICT(scope) DO UPDATE SET link = excluded.link",
        params![key, serde_json::to_string(&link)?],
    )?;
    Ok(link)
}
