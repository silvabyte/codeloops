use serde_json::json;
use session_history::{
    Error, History,
    model::{Capture, Change, Filter, Origin, Page, Query},
};
use uuid::Uuid;

fn capture() -> Capture {
    Capture {
        schema_version: 1,
        delivery_id: Uuid::new_v4().to_string(),
        origin: Origin {
            device_id: Uuid::new_v4().to_string(),
            installation_id: Uuid::new_v4().to_string(),
            source: "opencode".into(),
            source_version: "fixture".into(),
        },
        sequence: 1,
        native_session_id: "ses_native".into(),
        project_id: Uuid::new_v4().to_string(),
        workspace_id: Uuid::new_v4().to_string(),
        observed_at: 100,
        occurred_at: Some(90),
        change: Change::Message {
            native_id: "msg_a".into(),
            role: "user".into(),
            parent_native_id: None,
            removed: false,
        },
        source_payload: json!({"unknown_future_field":{"retain":true}}),
        checkpoints: vec![],
    }
}

fn part(base: &Capture, sequence: u64, message: &str, text: &str) -> Capture {
    Capture {
        delivery_id: Uuid::new_v4().to_string(),
        sequence,
        change: Change::Part {
            message_id: message.into(),
            native_id: "part_a".into(),
            kind: "text".into(),
            text: text.into(),
            removed: false,
        },
        ..base.clone()
    }
}

#[test]
fn retry_revision_repeat_and_restart_preserve_identity_and_provenance() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = History::open(dir.path()).unwrap();
    let c = capture();
    let receipt = h.ingest(c.clone()).unwrap();
    let repeated = h.ingest(c.clone()).unwrap();
    assert_eq!(receipt.capture_id, repeated.capture_id);
    assert_eq!(receipt.recorded_at, repeated.recorded_at);
    let mut conflicting = c.clone();
    conflicting.source_payload = json!({"changed":true});
    assert!(matches!(h.ingest(conflicting), Err(Error::Conflict(_))));
    h.ingest(part(&c, 3, "msg_a", "needle final")).unwrap();
    h.ingest(part(&c, 2, "msg_a", "needle stale")).unwrap();
    h.ingest(part(&c, 4, "msg_b", "needle final")).unwrap();
    drop(h);
    let h = History::open(dir.path()).unwrap();
    let result = h
        .query(Query::Search {
            text: "needle final".into(),
            filter: Filter::default(),
            page: Page::default(),
        })
        .unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 2);
    assert_eq!(result["items"][0]["id"], receipt.entry_id.unwrap());
    assert_eq!(result["items"][0]["role"], "user");
    assert_ne!(result["items"][0]["id"], result["items"][1]["id"]);
    let captures = h
        .query(Query::Captures {
            session_id: receipt.session_id,
            page: Page::default(),
        })
        .unwrap();
    assert_eq!(captures["items"].as_array().unwrap().len(), 4);
    let hash = captures["items"][0]["envelope_hash"].as_str().unwrap();
    let artifact = h
        .query(Query::Artifact {
            hash: hash.into(),
            offset: 0,
            limit: 65536,
        })
        .unwrap();
    use base64::Engine;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(artifact["data"].as_str().unwrap())
        .unwrap();
    let envelope: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    let payload_hash = envelope["source_payload_hash"].as_str().unwrap();
    assert_eq!(captures["items"][1]["source_payload_hash"], payload_hash);
    let payload = h
        .query(Query::Artifact {
            hash: payload_hash.into(),
            offset: 0,
            limit: 65536,
        })
        .unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload["data"].as_str().unwrap())
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
        c.source_payload
    );
}

#[test]
fn pages_are_bounded_and_scope_checked_and_sources_do_not_collide() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = History::open(dir.path()).unwrap();
    let c = capture();
    let first = h.ingest(c.clone()).unwrap();
    h.ingest(part(&c, 2, "msg_b", "second")).unwrap();
    let page = h
        .query(Query::Show {
            session_id: first.session_id.clone(),
            page: Page {
                limit: 1,
                cursor: None,
            },
        })
        .unwrap();
    h.ingest(part(&c, 3, "msg_c", "new after page start"))
        .unwrap();
    let cursor = page["next_cursor"].as_str().unwrap().to_string();
    let next = h
        .query(Query::Show {
            session_id: first.session_id.clone(),
            page: Page {
                limit: 1,
                cursor: Some(cursor.clone()),
            },
        })
        .unwrap();
    assert_eq!(next["items"].as_array().unwrap().len(), 1);
    assert_eq!(next["items"][0]["native_id"], "msg_b");
    assert_eq!(next["has_more"], false);
    assert!(
        h.query(Query::List {
            filter: Filter::default(),
            page: Page {
                limit: 1,
                cursor: Some(cursor)
            }
        })
        .is_err()
    );
    let other = capture();
    let other_receipt = h.ingest(other).unwrap();
    assert_ne!(first.session_id, other_receipt.session_id);
    assert!(matches!(
        h.query(Query::Show {
            session_id: "missing".into(),
            page: Page::default()
        }),
        Err(Error::Unknown(_))
    ));
    assert!(
        h.query(Query::Show {
            session_id: first.session_id,
            page: Page {
                limit: 101,
                cursor: None
            }
        })
        .is_err()
    );
}

#[test]
fn artifact_failure_cannot_publish_a_capture_and_missing_bytes_are_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = History::open(dir.path()).unwrap();
    let c = capture();
    std::fs::remove_dir(dir.path().join("artifacts")).unwrap();
    std::fs::write(dir.path().join("artifacts"), "blocked").unwrap();
    assert!(h.ingest(c.clone()).is_err());
    assert_eq!(
        h.query(Query::List {
            filter: Filter::default(),
            page: Page::default()
        })
        .unwrap()["items"],
        json!([])
    );
    std::fs::remove_file(dir.path().join("artifacts")).unwrap();
    std::fs::create_dir(dir.path().join("artifacts")).unwrap();
    let receipt = h.ingest(c).unwrap();
    let captures = h
        .query(Query::Captures {
            session_id: receipt.session_id,
            page: Page::default(),
        })
        .unwrap();
    let hash = captures["items"][0]["envelope_hash"].as_str().unwrap();
    std::fs::write(dir.path().join("artifacts").join(hash), b"broken zstd").unwrap();
    assert!(matches!(
        h.query(Query::Artifact {
            hash: hash.into(),
            offset: 0,
            limit: 100
        }),
        Err(Error::UnavailableArtifact(_))
    ));
}

#[test]
fn removal_retains_history_and_long_content_has_explicit_continuation() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = History::open(dir.path()).unwrap();
    let c = capture();
    let receipt = h.ingest(c.clone()).unwrap();
    h.ingest(part(&c, 2, "msg_a", &"界".repeat(5000))).unwrap();
    let entry_id = receipt.entry_id.unwrap();
    let entry = h
        .query(Query::Entry {
            entry_id: entry_id.clone(),
        })
        .unwrap();
    assert_eq!(entry["truncated"], true);
    assert_eq!(entry["text_bytes"], 15000);
    let first = h
        .query(Query::Artifact {
            hash: entry["content_hash"].as_str().unwrap().into(),
            offset: 0,
            limit: 10,
        })
        .unwrap();
    assert_eq!(first["next_offset"], 10);
    let mut removed = c.clone();
    removed.delivery_id = Uuid::new_v4().to_string();
    removed.sequence = 3;
    removed.change = Change::Message {
        native_id: "msg_a".into(),
        role: "user".into(),
        parent_native_id: None,
        removed: true,
    };
    h.ingest(removed).unwrap();
    assert_eq!(h.query(Query::Entry { entry_id }).unwrap()["removed"], true);
    assert_eq!(
        h.query(Query::Search {
            text: "界".repeat(4),
            filter: Filter::default(),
            page: Page::default()
        })
        .unwrap()["items"],
        json!([])
    );
}

#[test]
fn lifecycle_metadata_does_not_erase_state_or_workspace_associations() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = History::open(dir.path()).unwrap();
    let c = capture();
    h.ingest(c.clone()).unwrap();
    let mut metadata = c.clone();
    metadata.delivery_id = Uuid::new_v4().to_string();
    metadata.workspace_id = Uuid::new_v4().to_string();
    metadata.sequence = 10;
    metadata.change = Change::Lifecycle {
        state: None,
        title: Some("Renamed".into()),
        parent_native_id: None,
    };
    h.ingest(metadata).unwrap();
    let mut state = c;
    state.delivery_id = Uuid::new_v4().to_string();
    state.sequence = 9;
    state.change = Change::Lifecycle {
        state: Some("idle".into()),
        title: None,
        parent_native_id: None,
    };
    h.ingest(state).unwrap();
    let sessions = h
        .query(Query::List {
            filter: Filter::default(),
            page: Page::default(),
        })
        .unwrap();
    assert_eq!(sessions["items"][0]["state"], "idle");
    assert_eq!(sessions["items"][0]["title"], "Renamed");
    assert_eq!(
        sessions["items"][0]["workspace_ids"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
