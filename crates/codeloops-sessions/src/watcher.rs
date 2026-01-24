use std::path::PathBuf;

use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tokio::sync::broadcast;

use crate::parser::parse_session_summary;

/// Events emitted when session files change.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum SessionEvent {
    SessionCreated { id: String },
    SessionUpdated { id: String, iteration: usize },
    SessionCompleted { id: String, outcome: String },
}

/// Watches the sessions directory for file changes and emits SessionEvents.
pub struct SessionWatcher {
    tx: broadcast::Sender<SessionEvent>,
    _watcher: RecommendedWatcher,
}

impl SessionWatcher {
    /// Create a new watcher on the default sessions directory.
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .with_context(|| "Could not determine data directory")?;
        let sessions_dir = data_dir.join("codeloops").join("sessions");

        // Ensure the directory exists before watching
        std::fs::create_dir_all(&sessions_dir)?;

        Self::with_dir(sessions_dir)
    }

    /// Create a watcher on a custom directory.
    pub fn with_dir(sessions_dir: PathBuf) -> Result<Self> {
        let (tx, _) = broadcast::channel(256);
        let tx_clone = tx.clone();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                Self::handle_event(&tx_clone, &event);
            }
        })?;

        watcher.watch(&sessions_dir, RecursiveMode::NonRecursive)?;

        Ok(Self {
            tx,
            _watcher: watcher,
        })
    }

    /// Subscribe to session events.
    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.tx.subscribe()
    }

    fn handle_event(tx: &broadcast::Sender<SessionEvent>, event: &Event) {
        let paths: Vec<&PathBuf> = event
            .paths
            .iter()
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("jsonl"))
            .collect();

        for path in paths {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            let session_event = match event.kind {
                EventKind::Create(_) => Some(SessionEvent::SessionCreated { id }),
                EventKind::Modify(_) => {
                    // Re-read summary to determine if completed or just updated
                    match parse_session_summary(path) {
                        Ok(summary) => {
                            if let Some(outcome) = summary.outcome {
                                Some(SessionEvent::SessionCompleted { id, outcome })
                            } else {
                                Some(SessionEvent::SessionUpdated {
                                    id,
                                    iteration: summary.iterations,
                                })
                            }
                        }
                        Err(_) => Some(SessionEvent::SessionUpdated { id, iteration: 0 }),
                    }
                }
                _ => None,
            };

            if let Some(evt) = session_event {
                let _ = tx.send(evt);
            }
        }
    }
}
