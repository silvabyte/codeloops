//! Progress indicator widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Gauge, Widget},
};

/// Widget for showing interview progress
pub struct ProgressWidget {
    percentage: u8,
    label: String,
}

impl ProgressWidget {
    pub fn new(percentage: u8) -> Self {
        Self {
            percentage,
            label: format!("{}% complete", percentage),
        }
    }

    #[allow(dead_code)]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}

impl Widget for ProgressWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let color = match self.percentage {
            0..=25 => Color::Red,
            26..=50 => Color::Yellow,
            51..=75 => Color::Blue,
            _ => Color::Green,
        };

        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(color))
            .percent(self.percentage as u16)
            .label(self.label);

        gauge.render(area, buf);
    }
}
