//! Render smoke tests using `ratatui::backend::TestBackend` so we exercise
//! `render::draw` without touching a real terminal.

use std::path::PathBuf;

use codeloops_logging::FileChangeType;
use codeloops_tui::{render, AppState, FileEvent, RenderEvent};
use ratatui::{backend::TestBackend, Terminal};

fn buf_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buf = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

fn populated_state() -> AppState {
    let mut s = AppState::new();
    s.apply(RenderEvent::SetMaxIterations(Some(5)));
    s.apply(RenderEvent::SetAgentNames {
        actor: "claude".into(),
        critic: "claude".into(),
    });
    s.apply(RenderEvent::Header {
        prompt: "refactor the auth module to support OAuth".into(),
        working_dir: PathBuf::from("/tmp/proj"),
    });
    s.apply(RenderEvent::IterationStart { iteration: 1 });
    s.apply(RenderEvent::ActorStart);
    s.apply(RenderEvent::FileChange(FileEvent {
        path: "src/auth/login.rs".into(),
        change_type: FileChangeType::Modified,
    }));
    s.apply(RenderEvent::FileChange(FileEvent {
        path: "src/auth/oauth.rs".into(),
        change_type: FileChangeType::Created,
    }));
    s.apply(RenderEvent::FileChange(FileEvent {
        path: "src/auth/legacy.rs".into(),
        change_type: FileChangeType::Deleted,
    }));
    s
}

#[test]
fn renders_at_80x24() {
    let backend = TestBackend::new(80, 24);
    let mut term = Terminal::new(backend).unwrap();
    let state = populated_state();
    term.draw(|f| render::draw(&state, f)).unwrap();

    let s = buf_to_string(&term);
    assert!(s.contains("codeloops"), "expected title:\n{}", s);
    assert!(
        s.contains("Iteration 1 of 5"),
        "expected iteration label:\n{}",
        s
    );
    assert!(s.contains("login.rs"), "expected file path:\n{}", s);
    assert!(s.contains("oauth.rs"));
    assert!(s.contains("legacy.rs"));
    // Sigils
    assert!(s.contains('+'));
    assert!(s.contains('~'));
    assert!(s.contains('-'));
}

#[test]
fn renders_at_12_columns_without_panic() {
    let backend = TestBackend::new(12, 24);
    let mut term = Terminal::new(backend).unwrap();
    let state = populated_state();
    // Regression check: old fit_path_to_width path panicked / wrapped here.
    term.draw(|f| render::draw(&state, f)).unwrap();

    let buf = term.backend().buffer();
    assert_eq!(buf.area.width, 12);
}

#[test]
fn fill_region_grows_with_terminal_height() {
    // Push enough events that the file list would overflow the small viewport
    // but fit comfortably in the large one.
    let mut state = populated_state();
    for i in 0..30 {
        state.apply(RenderEvent::FileChange(FileEvent {
            path: format!("src/file_{:02}.rs", i),
            change_type: FileChangeType::Modified,
        }));
    }

    let mut small = Terminal::new(TestBackend::new(80, 16)).unwrap();
    small.draw(|f| render::draw(&state, f)).unwrap();
    let small_str = buf_to_string(&small);
    assert!(
        small_str.contains("showing"),
        "small terminal should show overflow indicator:\n{}",
        small_str
    );

    let mut large = Terminal::new(TestBackend::new(80, 50)).unwrap();
    large.draw(|f| render::draw(&state, f)).unwrap();
    let large_str = buf_to_string(&large);
    assert!(
        large_str.contains("file_29.rs"),
        "large terminal should show the latest file:\n{}",
        large_str
    );
}
