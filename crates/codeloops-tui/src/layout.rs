//! Layout constants and text formatting utilities.
//!
//! Grid: 2-space left margin. 8-character label column (positions 2-9).
//! Content starts at position 10+.

/// Left margin (2 spaces)
pub const MARGIN: &str = "  ";

/// Label column width (8 chars)
pub const LABEL_WIDTH: usize = 8;

/// Content starts at this column (margin + label)
pub const CONTENT_COL: usize = 10;

/// Maximum width for horizontal rules
pub const MAX_RULE_WIDTH: usize = 60;

/// Minimum terminal width we degrade gracefully below
pub const MIN_WIDTH: usize = 40;

/// Max file events shown before collapsing
pub const MAX_FILE_EVENTS: usize = 15;

/// Number of file events to show when collapsed
pub const COLLAPSED_SHOW: usize = 12;

/// Get the effective terminal width, capped for aesthetics.
pub fn term_width() -> usize {
    crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)
}

/// Get the terminal height in rows, falling back to 24.
pub fn term_height() -> usize {
    crossterm::terminal::size()
        .map(|(_, h)| h as usize)
        .unwrap_or(24)
}

/// Adaptive height for the streaming file box: term_h / 3, clamped to [3, 15].
pub fn stream_box_height() -> usize {
    (term_height() / 3).clamp(3, 15)
}

/// Truncate a file path so the printed line (content indent + sigil + space + path)
/// fits one terminal row. Reuses `truncate_path`, which head-truncates and so
/// preserves the file name at the tail.
pub fn fit_path_to_width(path: &str, term_w: usize) -> String {
    // CONTENT_COL spaces of indent + 1 sigil + 1 space already consumed.
    // If nothing is left for the path, return empty so the line itself fits.
    // (Don't `.max(1)` here — that adds a phantom char and pushes the printed
    // width to term_w + 1, wrapping the row.)
    let avail = term_w.saturating_sub(CONTENT_COL + 2);
    if avail == 0 {
        return String::new();
    }
    truncate_path(path, avail)
}

/// Build a left-aligned label padded to LABEL_WIDTH.
/// Example: `"actor"` -> `"actor   "` (8 chars)
pub fn pad_label(label: &str) -> String {
    format!("{:<width$}", label, width = LABEL_WIDTH)
}

/// Build a horizontal rule that fills from the current position to the edge.
/// The rule is capped at MAX_RULE_WIDTH total line width.
pub fn rule(used_cols: usize) -> String {
    let width = term_width().min(MAX_RULE_WIDTH);
    let remaining = width.saturating_sub(used_cols);
    "─".repeat(remaining)
}

/// Wrap text to fit within the available width, respecting the content column indent.
/// Returns lines that should each be printed at column CONTENT_COL.
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let avail = max_width.saturating_sub(CONTENT_COL);
    if avail == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current_line = String::new();
        for word in paragraph.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= avail {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Build the content-column indent string (spaces to reach column CONTENT_COL).
pub fn content_indent() -> String {
    " ".repeat(CONTENT_COL)
}

/// Truncate a file path for display in narrow terminals.
pub fn truncate_path(path: &str, max_len: usize) -> String {
    let char_count = path.chars().count();
    if char_count <= max_len {
        path.to_string()
    } else if max_len > 1 {
        // Take the last (max_len - 1) chars and prepend ellipsis
        let skip = char_count - (max_len - 1);
        let tail: String = path.chars().skip(skip).collect();
        format!("…{}", tail)
    } else {
        "…".to_string()
    }
}

/// Shorten a home-relative path: `/home/user/code/foo` -> `~/code/foo`
pub fn shorten_home(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        if path.starts_with(home_str.as_ref()) {
            return format!("~{}", &path[home_str.len()..]);
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_label_short() {
        assert_eq!(pad_label("actor"), "actor   ");
        assert_eq!(pad_label("critic"), "critic  ");
        assert_eq!(pad_label("prompt"), "prompt  ");
    }

    #[test]
    fn pad_label_exact() {
        assert_eq!(pad_label("12345678"), "12345678");
    }

    #[test]
    fn wrap_simple() {
        let lines = wrap_text("hello world", 80);
        assert_eq!(lines, vec!["hello world"]);
    }

    #[test]
    fn wrap_long_text() {
        let text = "a ".repeat(50).trim().to_string();
        let lines = wrap_text(&text, 30);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(line.len() <= 20); // 30 - CONTENT_COL(10) = 20
        }
    }

    #[test]
    fn truncate_path_short() {
        assert_eq!(truncate_path("README.md", 20), "README.md");
    }

    #[test]
    fn truncate_path_long() {
        let path = "very/long/path/to/some/deep/file.rs";
        let truncated = truncate_path(path, 15);
        assert!(truncated.chars().count() <= 15);
        assert!(truncated.starts_with('…'));
    }

    #[test]
    fn fit_path_short_passthrough() {
        // term_w = 80, CONTENT_COL+2 = 12, avail = 68 — short path unchanged.
        assert_eq!(fit_path_to_width("src/main.rs", 80), "src/main.rs");
    }

    #[test]
    fn fit_path_long_truncated_to_avail() {
        let path = "apps/timeline-poc/src/preview/components/widgets/Toolbar.tsx";
        // term_w = 30, avail = 30 - 12 = 18.
        let out = fit_path_to_width(path, 30);
        assert!(out.chars().count() <= 18);
        assert!(out.starts_with('…'));
    }

    #[test]
    fn fit_path_narrow_terminal_degrades() {
        // term_w = 12 leaves zero room for the path itself — output is empty so
        // that the printed line (10 indent + 1 sigil + 1 space) fits in 12 cols.
        assert_eq!(fit_path_to_width("anything/at/all.rs", 12), "");
    }

    #[test]
    fn fit_path_total_visible_width_never_exceeds_term_w() {
        // Invariant: indent(CONTENT_COL=10) + sigil(1) + space(1) + path <= term_w.
        // Pre-fix this failed at term_w = 12 because `.max(1)` produced a 1-char
        // path and the line totalled 13 columns.
        let path = "very/long/path/to/some/deep/file.rs";
        for w in [12, 13, 14, 15, 20, 30, 80] {
            let out = fit_path_to_width(path, w);
            let total = CONTENT_COL + 2 + out.chars().count();
            assert!(total <= w, "term_w={}: total visible {} exceeds", w, total);
        }
    }

    #[test]
    fn rule_generation() {
        let r = rule(12);
        // Should have some dashes
        assert!(!r.is_empty());
        // Each char is the unicode em-dash
        for c in r.chars() {
            assert_eq!(c, '─');
        }
    }
}
