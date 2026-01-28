//! Draft preview widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget, Wrap,
    },
};

use crate::prompt::session::PromptDraft;

/// Widget for displaying the draft prompt preview
pub struct DraftWidget<'a> {
    draft: &'a PromptDraft,
    scroll_offset: u16,
}

impl<'a> DraftWidget<'a> {
    pub fn new(draft: &'a PromptDraft) -> Self {
        Self {
            draft,
            scroll_offset: 0,
        }
    }

    pub fn scroll(mut self, offset: u16) -> Self {
        self.scroll_offset = offset;
        self
    }
}

impl Widget for DraftWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta))
            .title(" Draft Preview ");

        let inner_area = block.inner(area);
        block.render(area, buf);

        let mut lines = vec![];

        // Title
        if let Some(ref title) = self.draft.title {
            lines.push(Line::from(Span::styled(
                format!("# {}", title),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
        }

        // Goal
        if let Some(ref goal) = self.draft.goal {
            lines.push(Line::from(Span::styled(
                "## Goal",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for line in goal.lines() {
                lines.push(Line::from(line));
            }
            lines.push(Line::from(""));
        }

        // Context
        if let Some(ref context) = self.draft.context {
            lines.push(Line::from(Span::styled(
                "## Context",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for line in context.lines() {
                lines.push(Line::from(line));
            }
            lines.push(Line::from(""));
        }

        // Requirements
        if !self.draft.requirements.is_empty() {
            lines.push(Line::from(Span::styled(
                "## Requirements",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for req in &self.draft.requirements {
                lines.push(Line::from(Span::styled(
                    format!("• {}", req),
                    Style::default().fg(Color::White),
                )));
            }
            lines.push(Line::from(""));
        }

        // Constraints
        if !self.draft.constraints.is_empty() {
            lines.push(Line::from(Span::styled(
                "## Constraints",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for constraint in &self.draft.constraints {
                lines.push(Line::from(Span::styled(
                    format!("• {}", constraint),
                    Style::default().fg(Color::Red),
                )));
            }
            lines.push(Line::from(""));
        }

        // Files to modify
        if !self.draft.files_to_modify.is_empty() {
            lines.push(Line::from(Span::styled(
                "## Files to Modify",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for file in &self.draft.files_to_modify {
                lines.push(Line::from(Span::styled(
                    format!("• {}", file),
                    Style::default().fg(Color::Green),
                )));
            }
            lines.push(Line::from(""));
        }

        // Acceptance criteria
        if !self.draft.acceptance_criteria.is_empty() {
            lines.push(Line::from(Span::styled(
                "## Acceptance Criteria",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for criteria in &self.draft.acceptance_criteria {
                lines.push(Line::from(Span::styled(
                    format!("☐ {}", criteria),
                    Style::default().fg(Color::White),
                )));
            }
            lines.push(Line::from(""));
        }

        // Notes
        if let Some(ref notes) = self.draft.notes {
            lines.push(Line::from(Span::styled(
                "## Notes",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for line in notes.lines() {
                lines.push(Line::from(line));
            }
        }

        // Show placeholder if empty
        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "Draft will appear here as you answer questions...",
                Style::default().fg(Color::DarkGray),
            )));
        }

        let text = Text::from(lines);
        let content_height = text.height() as u16;
        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));

        paragraph.render(inner_area, buf);

        // Render scrollbar if content is larger than area
        if content_height > inner_area.height {
            let mut scrollbar_state = ScrollbarState::default()
                .content_length(content_height as usize)
                .position(self.scroll_offset as usize)
                .viewport_content_length(inner_area.height as usize);

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));

            scrollbar.render(inner_area, buf, &mut scrollbar_state);
        }
    }
}
