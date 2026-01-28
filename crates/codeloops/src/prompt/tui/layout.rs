//! Layout calculations for the TUI.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Main layout areas
pub struct MainLayout {
    pub header: Rect,
    pub left_panel: Rect,
    pub right_panel: Rect,
    pub footer: Rect,
}

/// Left panel (interview) layout
pub struct InterviewLayout {
    pub question: Rect,
    pub input: Rect,
    pub status: Rect,
}

/// Right panel (draft) layout
pub struct DraftLayout {
    pub title: Rect,
    pub progress: Rect,
    pub content: Rect,
}

impl MainLayout {
    /// Calculate the main layout from the terminal area
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Main content
                Constraint::Length(3), // Footer
            ])
            .split(area);

        let header = chunks[0];
        let main_area = chunks[1];
        let footer = chunks[2];

        // Split main area into left (interview) and right (draft preview)
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(55), // Interview
                Constraint::Percentage(45), // Draft preview
            ])
            .split(main_area);

        Self {
            header,
            left_panel: panels[0],
            right_panel: panels[1],
            footer,
        }
    }
}

impl InterviewLayout {
    /// Calculate the interview panel layout
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(5),    // Question area
                Constraint::Length(5), // Input area
                Constraint::Length(2), // Status
            ])
            .split(area);

        Self {
            question: chunks[0],
            input: chunks[1],
            status: chunks[2],
        }
    }
}

impl DraftLayout {
    /// Calculate the draft panel layout
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Length(2), // Progress bar
                Constraint::Min(5),    // Content
            ])
            .split(area);

        Self {
            title: chunks[0],
            progress: chunks[1],
            content: chunks[2],
        }
    }
}
