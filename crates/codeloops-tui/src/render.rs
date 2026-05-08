//! ratatui draw function for the inline live viewport.

use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use codeloops_logging::FileChangeType;

use crate::app::{AppState, Phase};
use crate::spinner::{format_elapsed, BRAILLE_FRAMES};

const MARGIN: &str = "  ";

pub fn draw(state: &AppState, f: &mut Frame) {
    let area = f.area();
    if area.height == 0 || area.width == 0 {
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(1), // blank
        Constraint::Length(1), // codeloops · elapsed
        Constraint::Length(1), // blank
        Constraint::Length(2), // prompt (wrapped)
        Constraint::Length(1), // blank
        Constraint::Length(1), // iteration rule
        Constraint::Length(1), // blank
        Constraint::Fill(1),   // file list
        Constraint::Length(1), // blank
        Constraint::Length(1), // status line
        Constraint::Length(1), // blank
    ])
    .split(area);

    // Title: "codeloops · 3m 47s"
    let total = format_elapsed(state.elapsed_total().as_secs());
    let title = Line::from(vec![
        Span::raw(MARGIN),
        Span::styled(
            "codeloops",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", Style::default().add_modifier(Modifier::DIM)),
        Span::styled(total, Style::default().add_modifier(Modifier::DIM)),
    ]);
    f.render_widget(Paragraph::new(title), chunks[1]);

    // Prompt (quoted, may wrap into the 2 reserved rows)
    let prompt_text = if state.prompt.is_empty() {
        String::new()
    } else {
        format!("\"{}\"", state.prompt)
    };
    let prompt = Paragraph::new(prompt_text)
        .style(Style::default().add_modifier(Modifier::DIM))
        .wrap(Wrap { trim: true });
    f.render_widget(prompt.clone(), prompt_with_margin_area(chunks[3]));

    // Iteration rule
    let iter_label = match state.max_iterations {
        Some(max) => format!("Iteration {} of {}", state.current_iteration, max),
        None => format!("Iteration {}", state.current_iteration),
    };
    let rule_area = chunks[5];
    let prefix = format!("── {} ", iter_label);
    let rest = (rule_area.width as usize).saturating_sub(MARGIN.len() + prefix.chars().count());
    let dashes = "─".repeat(rest);
    let rule = Line::from(vec![
        Span::raw(MARGIN),
        Span::styled(prefix, Style::default().add_modifier(Modifier::DIM)),
        Span::styled(dashes, Style::default().add_modifier(Modifier::DIM)),
    ]);
    f.render_widget(Paragraph::new(rule), rule_area);

    // File list region
    let list_area = chunks[7];
    let visible = list_area.height as usize;
    if visible > 0 {
        let total_events = state.total_events_this_iter;
        let recent_count = state.recent.len();

        let overflow = total_events > visible || total_events > recent_count;
        let max_rows = if overflow {
            visible.saturating_sub(1)
        } else {
            visible
        };
        let overflow_line = overflow;

        let start = recent_count.saturating_sub(max_rows);
        let mut lines: Vec<Line> = state
            .recent
            .iter()
            .skip(start)
            .map(|fe| {
                let (sigil, color) = match fe.change_type {
                    FileChangeType::Created => ("+", Color::Green),
                    FileChangeType::Modified => ("~", Color::Yellow),
                    FileChangeType::Deleted => ("-", Color::Red),
                };
                Line::from(vec![
                    Span::raw("     "),
                    Span::styled(sigil, Style::default().fg(color)),
                    Span::raw(" "),
                    Span::raw(fe.path.clone()),
                ])
            })
            .collect();

        if overflow_line {
            lines.push(Line::from(vec![
                Span::raw("     "),
                Span::styled(
                    format!(
                        "showing {} of {} events",
                        lines.len().min(max_rows),
                        total_events
                    ),
                    Style::default().add_modifier(Modifier::DIM),
                ),
            ]));
        }

        let para = Paragraph::new(lines);
        f.render_widget(para, list_area);
    }

    // Status line
    let frame_idx = state.spinner_frame % BRAILLE_FRAMES.len();
    let frame = BRAILLE_FRAMES[frame_idx];
    let (label, show_spinner) = match state.phase {
        Phase::Actor => ("actor working", true),
        Phase::Critic => ("critic reviewing", true),
        Phase::Idle => ("idle", false),
        Phase::Done => ("done", false),
    };
    let phase_elapsed = format_elapsed(state.elapsed_phase().as_secs());
    let mut spans = vec![Span::raw(MARGIN)];
    if show_spinner {
        spans.push(Span::styled(
            format!("{}  ", frame),
            Style::default().fg(Color::Cyan),
        ));
    } else {
        spans.push(Span::raw("   "));
    }
    spans.push(Span::styled(
        label,
        Style::default().add_modifier(Modifier::DIM),
    ));
    if state.phase_started_at.is_some() {
        spans.push(Span::styled(
            format!(" · {}", phase_elapsed),
            Style::default().add_modifier(Modifier::DIM),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), chunks[9]);
}

fn prompt_with_margin_area(area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    use ratatui::layout::Rect;
    let mx = MARGIN.len() as u16;
    if area.width <= mx {
        return area;
    }
    Rect {
        x: area.x + mx,
        y: area.y,
        width: area.width - mx,
        height: area.height,
    }
}
