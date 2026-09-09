//! Cursor command-hook translation. Native payloads are retained without reading
//! optional transcript paths or returning control fields to the client.
use crate::{
    AppResult,
    config::Config,
    outbox::{self, identity},
};
use rusqlite::{OptionalExtension, Transaction, params};
use serde_json::Value;
use session_history::{
    model::{Capture, Change, Origin},
    now_ms,
};
use uuid::Uuid;

fn required<'a>(event: &'a Value, key: &str) -> AppResult<&'a str> {
    optional(event, key).ok_or_else(|| format!("missing {key} in Cursor hook").into())
}

fn optional<'a>(event: &'a Value, key: &str) -> Option<&'a str> {
    event[key].as_str().filter(|s| !s.is_empty())
}

fn lifecycle(state: Option<&str>, parent: Option<&str>) -> Change {
    Change::Lifecycle {
        state: state.map(String::from),
        title: None,
        parent_native_id: parent.map(String::from),
    }
}

fn changes(tx: &Transaction<'_>, event: &Value, session: &str) -> AppResult<Vec<Change>> {
    let hook = required(event, "hook_event_name")?;
    let first = tx.execute("INSERT OR IGNORE INTO cursor_sessions VALUES(?)", [session])? == 1;
    let parent = optional(event, "parent_conversation_id").filter(|id| *id != session);
    let state = match hook {
        // Lifecycle hooks can arrive after the first completed turn. A delayed
        // sessionStart must not reopen an idle/ended conversation.
        "sessionStart" => first.then_some("active"),
        "beforeSubmitPrompt" => Some("active"),
        "stop" => Some(match optional(event, "status") {
            Some("completed") => "idle",
            Some("aborted" | "error") => "interrupted",
            _ => "unknown",
        }),
        "sessionEnd" => Some(match optional(event, "reason") {
            Some("completed" | "window_close" | "user_close") => "ended",
            Some("aborted" | "error") => "interrupted",
            _ => "unknown",
        }),
        // These are observations in the containing conversation, not proof of a
        // distinct child conversation or of the parent's lifecycle transition.
        "afterAgentResponse" | "subagentStart" | "subagentStop" => None,
        _ => return Err(format!("unsupported Cursor hook: {hook}").into()),
    };
    let mut changes = vec![lifecycle(state, parent)];
    if hook == "beforeSubmitPrompt" || hook == "afterAgentResponse" {
        let is_user = hook == "beforeSubmitPrompt";
        let field = if is_user { "prompt" } else { "text" };
        let text = event[field]
            .as_str()
            .ok_or_else(|| format!("missing {field} in Cursor hook"))?;
        // Neither text nor generation identity uniquely identifies a message:
        // several assistant messages may finish in one generation.
        let message = format!("hook-message:{}", Uuid::new_v4());
        let generation = optional(event, "generation_id");
        let mut parent_message = None;
        if let Some(generation) = generation {
            if is_user {
                tx.execute("INSERT INTO cursor_prompts VALUES(?,?,?) ON CONFLICT(session,generation) DO UPDATE SET message=excluded.message", params![session,generation,message])?;
            } else {
                parent_message = tx
                    .query_row(
                        "SELECT message FROM cursor_prompts WHERE session=? AND generation=?",
                        params![session, generation],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()?;
            }
        }
        changes.push(Change::Message {
            native_id: message.clone(),
            role: if is_user { "user" } else { "assistant" }.into(),
            parent_native_id: parent_message,
            removed: false,
        });
        changes.push(Change::Part {
            message_id: message,
            native_id: "text".into(),
            kind: "text".into(),
            text: text.into(),
            removed: false,
        });
    }
    Ok(changes)
}

pub fn enqueue(config: &Config, event: Value) -> AppResult<()> {
    let observed_at = now_ms();
    let session = required(&event, "conversation_id")?;
    let version = optional(&event, "cursor_version").unwrap_or("unknown");
    if session.len() > 1024
        || version.len() > 1024
        || optional(&event, "parent_conversation_id").is_some_and(|s| s.len() > 1024)
    {
        return Err("Cursor source identity exceeds 1024 bytes".into());
    }
    let mut roots = Vec::new();
    if let Some(value) = event.get("workspace_roots") {
        let values = value.as_array().ok_or("workspace_roots must be an array")?;
        if values.len() > 32 {
            return Err("Cursor hook exceeds 32 workspace roots".into());
        }
        for root in values {
            let root = root
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("workspace root must be a nonempty string")?;
            roots.push(root.to_string());
        }
    }
    roots.sort();
    roots.dedup();
    // A missing workspace is session-specific; do not group unrelated rootless chats.
    let project_key = if roots.is_empty() {
        format!("cursor-project:rootless:{session}")
    } else {
        format!("cursor-project:roots:{}", serde_json::to_string(&roots)?)
    };
    let primary_key = roots
        .first()
        .map(|r| format!("workspace:{r}"))
        .unwrap_or_else(|| format!("cursor-rootless:{session}"));
    let mut db = outbox::open(config)?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let changes = changes(&tx, &event, session)?;
    let origin = Origin {
        device_id: identity(&tx, "device")?,
        installation_id: identity(&tx, "cursor")?,
        source: "cursor".into(),
        source_version: version.into(),
    };
    let mut capture = Capture {
        schema_version: 1,
        delivery_id: String::new(),
        origin,
        sequence: 0,
        native_session_id: session.into(),
        project_id: identity(&tx, &project_key)?,
        workspace_id: identity(&tx, &primary_key)?,
        observed_at,
        occurred_at: None,
        change: lifecycle(None, None),
        source_payload: event.clone(),
    };
    for change in changes {
        capture.change = change;
        queue(&tx, &mut capture)?;
    }
    // Preserve every observed root as an association, without duplicating messages.
    for root in roots.iter().skip(1) {
        capture.workspace_id = identity(&tx, &format!("workspace:{root}"))?;
        capture.change = lifecycle(None, None);
        queue(&tx, &mut capture)?;
    }
    tx.commit()?;
    Ok(())
}

fn queue(tx: &Transaction<'_>, capture: &mut Capture) -> AppResult<()> {
    capture.delivery_id = Uuid::new_v4().to_string();
    capture.sequence = outbox::sequence(tx)?;
    outbox::queue(tx, capture)
}
