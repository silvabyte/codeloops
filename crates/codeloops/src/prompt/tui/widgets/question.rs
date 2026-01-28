//! Question display widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

/// Widget for displaying the current question
pub struct QuestionWidget<'a> {
    question: &'a str,
    context: Option<&'a str>,
    section: Option<&'a str>,
}

impl<'a> QuestionWidget<'a> {
    pub fn new(question: &'a str) -> Self {
        Self {
            question,
            context: None,
            section: None,
        }
    }

    pub fn context(mut self, ctx: Option<&'a str>) -> Self {
        self.context = ctx;
        self
    }

    pub fn section(mut self, section: Option<&'a str>) -> Self {
        self.section = section;
        self
    }
}

impl Widget for QuestionWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut lines = vec![];

        // Section indicator
        if let Some(section) = self.section {
            lines.push(Line::from(vec![Span::styled(
                format!("[{}] ", section),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));
        }

        // Main question
        lines.push(Line::from(Span::styled(
            self.question,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));

        // Context/help text
        if let Some(context) = self.context {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                context,
                Style::default().fg(Color::DarkGray),
            )));
        }

        let text = Text::from(lines);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
            .title(" Question ");

        Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}

/// Widget for displaying thinking/processing status
#[allow(dead_code)]
pub struct ThinkingWidget<'a> {
    message: &'a str,
}

#[allow(dead_code)]
impl<'a> ThinkingWidget<'a> {
    pub fn new(message: &'a str) -> Self {
        Self { message }
    }
}

impl Widget for ThinkingWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = Text::from(vec![
            Line::from(Span::styled(
                "⏳ Thinking...",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                self.message,
                Style::default().fg(Color::DarkGray),
            )),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}
