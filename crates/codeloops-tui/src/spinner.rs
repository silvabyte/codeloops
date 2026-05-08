//! Spinner frames and elapsed-time formatting.

pub const BRAILLE_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
pub const ASCII_FRAMES: &[char] = &['|', '/', '-', '\\'];

/// Format elapsed seconds into a human-readable duration string.
///
/// Examples: `0s`, `12s`, `1m 23s`, `12m 5s`
pub fn format_elapsed(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else {
        let m = secs / 60;
        let s = secs % 60;
        format!("{}m {}s", m, s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_elapsed_seconds() {
        assert_eq!(format_elapsed(0), "0s");
        assert_eq!(format_elapsed(5), "5s");
        assert_eq!(format_elapsed(59), "59s");
    }

    #[test]
    fn format_elapsed_minutes() {
        assert_eq!(format_elapsed(60), "1m 0s");
        assert_eq!(format_elapsed(83), "1m 23s");
        assert_eq!(format_elapsed(725), "12m 5s");
    }
}
