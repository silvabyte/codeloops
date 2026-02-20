//! Braille spinner animation for in-place terminal updates.
//!
//! Cycles through braille characters at ~100ms per frame to provide
//! a subtle activity pulse while agents are executing.

const BRAILLE_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const ASCII_FRAMES: &[char] = &['|', '/', '-', '\\'];

pub struct Spinner {
    frame: usize,
    frames: &'static [char],
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            frame: 0,
            frames: BRAILLE_FRAMES,
        }
    }

    pub fn ascii() -> Self {
        Self {
            frame: 0,
            frames: ASCII_FRAMES,
        }
    }

    pub fn tick(&mut self) -> char {
        let ch = self.frames[self.frame % self.frames.len()];
        self.frame = self.frame.wrapping_add(1);
        ch
    }

    pub fn current(&self) -> char {
        self.frames[self.frame % self.frames.len()]
    }
}

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
    fn spinner_cycles_through_frames() {
        let mut s = Spinner::new();
        assert_eq!(s.tick(), '⠋');
        assert_eq!(s.tick(), '⠙');
        assert_eq!(s.tick(), '⠹');
        assert_eq!(s.tick(), '⠸');
        assert_eq!(s.tick(), '⠼');
        assert_eq!(s.tick(), '⠴');
        assert_eq!(s.tick(), '⠦');
        assert_eq!(s.tick(), '⠧');
        assert_eq!(s.tick(), '⠇');
        assert_eq!(s.tick(), '⠏');
        // wraps around
        assert_eq!(s.tick(), '⠋');
    }

    #[test]
    fn ascii_spinner_cycles() {
        let mut s = Spinner::ascii();
        assert_eq!(s.tick(), '|');
        assert_eq!(s.tick(), '/');
        assert_eq!(s.tick(), '-');
        assert_eq!(s.tick(), '\\');
        assert_eq!(s.tick(), '|');
    }

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
