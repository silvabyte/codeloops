use std::convert::Infallible;

use axum::extract::State;
use axum::response::sse::{Event, Sse};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use super::AppState;

pub async fn session_events(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.watcher.subscribe();
    let stream = BroadcastStream::new(rx).map(|result| {
        let event = match result {
            Ok(evt) => {
                let event_type = match &evt {
                    codeloops_sessions::SessionEvent::SessionCreated { .. } => "session_created",
                    codeloops_sessions::SessionEvent::SessionUpdated { .. } => "session_updated",
                    codeloops_sessions::SessionEvent::SessionCompleted { .. } => {
                        "session_completed"
                    }
                };
                Event::default()
                    .event(event_type)
                    .data(serde_json::to_string(&evt).unwrap_or_default())
            }
            Err(_) => Event::default().comment("missed event"),
        };
        Ok(event)
    });

    Sse::new(stream)
}
