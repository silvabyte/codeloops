//! Input widgets for different response types.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::prompt::protocol::SelectOption;

/// Text input widget with cursor
pub struct TextInput<'a> {
    value: &'a str,
    cursor_position: usize,
    placeholder: &'a str,
    focused: bool,
}

impl<'a> TextInput<'a> {
    pub fn new(value: &'a str, cursor_position: usize) -> Self {
        Self {
            value,
            cursor_position,
            placeholder: "Type your answer...",
            focused: true,
        }
    }

    #[allow(dead_code)]
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl Widget for TextInput<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused {
            Color::Green
        } else {
            Color::DarkGray
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(if self.focused {
                " Input "
            } else {
                " Input (press Enter to edit) "
            });

        let inner_area = block.inner(area);
        block.render(area, buf);

        if self.value.is_empty() && !self.focused {
            // Show placeholder
            let placeholder = Paragraph::new(Span::styled(
                self.placeholder,
                Style::default().fg(Color::DarkGray),
            ));
            placeholder.render(inner_area, buf);
        } else {
            // Show value with cursor
            let display = if self.focused {
                let before: String = self.value.chars().take(self.cursor_position).collect();
                let cursor_char = self.value.chars().nth(self.cursor_position).unwrap_or(' ');
                let after: String = self.value.chars().skip(self.cursor_position + 1).collect();

                Line::from(vec![
                    Span::raw(before),
                    Span::styled(
                        cursor_char.to_string(),
                        Style::default().bg(Color::White).fg(Color::Black),
                    ),
                    Span::raw(after),
                ])
            } else {
                Line::from(self.value)
            };

            Paragraph::new(display)
                .wrap(Wrap { trim: false })
                .render(inner_area, buf);
        }
    }
}

/// Widget for the input area (combines input + help text)
#[allow(dead_code)]
pub struct InputWidget<'a> {
    input: TextInput<'a>,
}

#[allow(dead_code)]
impl<'a> InputWidget<'a> {
    pub fn new(value: &'a str, cursor_position: usize) -> Self {
        Self {
            input: TextInput::new(value, cursor_position),
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.input = self.input.focused(focused);
        self
    }
}

impl Widget for InputWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.input.render(area, buf);
    }
}

/// Single-select input widget
pub struct SelectInput<'a> {
    options: &'a [SelectOption],
    selected: usize,
    focused: bool,
}

impl<'a> SelectInput<'a> {
    pub fn new(options: &'a [SelectOption], selected: usize) -> Self {
        Self {
            options,
            selected,
            focused: true,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl Widget for SelectInput<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused {
            Color::Green
        } else {
            Color::DarkGray
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" Select (↑↓ to move, Enter to confirm) ");

        let inner_area = block.inner(area);
        block.render(area, buf);

        let mut lines = vec![];
        for (i, option) in self.options.iter().enumerate() {
            let is_selected = i == self.selected;
            let prefix = if is_selected { "▶ " } else { "  " };

            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let mut line_spans = vec![
                Span::styled(prefix, style),
                Span::styled(&option.label, style),
            ];

            if let Some(ref desc) = option.description {
                line_spans.push(Span::styled(
                    format!(" - {}", desc),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            lines.push(Line::from(line_spans));
        }

        Paragraph::new(lines).render(inner_area, buf);
    }
}

/// Multi-select input widget
pub struct MultiSelectInput<'a> {
    options: &'a [SelectOption],
    selected: &'a [bool],
    cursor: usize,
    focused: bool,
}

impl<'a> MultiSelectInput<'a> {
    pub fn new(options: &'a [SelectOption], selected: &'a [bool], cursor: usize) -> Self {
        Self {
            options,
            selected,
            cursor,
            focused: true,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl Widget for MultiSelectInput<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused {
            Color::Green
        } else {
            Color::DarkGray
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" Multi-Select (Space to toggle, Enter to confirm) ");

        let inner_area = block.inner(area);
        block.render(area, buf);

        let mut lines = vec![];
        for (i, option) in self.options.iter().enumerate() {
            let is_cursor = i == self.cursor;
            let is_checked = self.selected.get(i).copied().unwrap_or(false);

            let checkbox = if is_checked { "[✓]" } else { "[ ]" };
            let prefix = if is_cursor { "▶ " } else { "  " };

            let style = if is_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if is_checked {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };

            let mut line_spans = vec![
                Span::styled(prefix, style),
                Span::styled(checkbox, style),
                Span::styled(format!(" {}", option.label), style),
            ];

            if let Some(ref desc) = option.description {
                line_spans.push(Span::styled(
                    format!(" - {}", desc),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            lines.push(Line::from(line_spans));
        }

        Paragraph::new(lines).render(inner_area, buf);
    }
}

/// Yes/No confirmation widget
pub struct ConfirmInput {
    selected: bool, // true = Yes, false = No
    focused: bool,
}

impl ConfirmInput {
    pub fn new(selected: bool) -> Self {
        Self {
            selected,
            focused: true,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl Widget for ConfirmInput {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused {
            Color::Green
        } else {
            Color::DarkGray
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" Confirm (←→ to select, Enter to confirm) ");

        let inner_area = block.inner(area);
        block.render(area, buf);

        let yes_style = if self.selected {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let no_style = if !self.selected {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled(if self.selected { "▶ " } else { "  " }, yes_style),
            Span::styled("Yes", yes_style),
            Span::raw("     "),
            Span::styled(if !self.selected { "▶ " } else { "  " }, no_style),
            Span::styled("No", no_style),
        ]);

        Paragraph::new(line).render(inner_area, buf);
    }
}
