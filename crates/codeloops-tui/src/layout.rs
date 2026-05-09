//! Layout helpers (path display).

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

/// Truncate `text` to at most `budget` codepoints, appending `…` when it had
/// to be cut. Used for content that wraps over multiple lines where
/// `budget = width * max_lines` is a pessimistic upper bound (whitespace-aware
/// wrap may need fewer chars, so we may truncate slightly earlier than
/// strictly needed — that's fine for an "overflow" indicator).
pub fn ellipsize_to_budget(text: &str, budget: usize) -> String {
    let len = text.chars().count();
    if len <= budget {
        return text.to_string();
    }
    if budget == 0 {
        return String::new();
    }
    if budget == 1 {
        return "…".to_string();
    }
    let keep: String = text.chars().take(budget - 1).collect();
    format!("{}…", keep)
}

/// Truncate `path` so its codepoint count is at most `max`, keeping the tail
/// (the most informative end of a path) and prefixing with `…`. If `max == 0`
/// returns an empty string. Codepoint-counted, so this is conservative but
/// safe for ASCII paths and won't panic on multi-byte chars.
pub fn truncate_path(path: &str, max: usize) -> String {
    let len = path.chars().count();
    if len <= max {
        return path.to_string();
    }
    if max == 0 {
        return String::new();
    }
    if max == 1 {
        return "…".to_string();
    }
    // Reserve one codepoint for the leading ellipsis.
    let keep = max - 1;
    let skip = len - keep;
    let tail: String = path.chars().skip(skip).collect();
    format!("…{}", tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorten_home_replaces_prefix() {
        if let Some(home) = dirs::home_dir() {
            let p = home.join("code/foo").to_string_lossy().to_string();
            assert_eq!(shorten_home(&p), "~/code/foo");
        }
    }

    #[test]
    fn shorten_home_passthrough_for_unrelated_paths() {
        assert_eq!(shorten_home("/etc/hosts"), "/etc/hosts");
    }

    #[test]
    fn truncate_path_passthrough_when_fits() {
        assert_eq!(truncate_path("src/lib.rs", 20), "src/lib.rs");
        assert_eq!(truncate_path("src/lib.rs", 10), "src/lib.rs");
    }

    #[test]
    fn truncate_path_keeps_tail_with_leading_ellipsis() {
        let p = "crates/codeloops-tui/src/render.rs";
        // 30 chars: keep 29 chars of tail, prefix with …
        let out = truncate_path(p, 30);
        assert_eq!(out.chars().count(), 30);
        assert!(out.starts_with('…'));
        assert!(out.ends_with("render.rs"));
    }

    #[test]
    fn truncate_path_handles_zero_max() {
        assert_eq!(truncate_path("anything", 0), "");
    }

    #[test]
    fn truncate_path_handles_max_one() {
        assert_eq!(truncate_path("anything", 1), "…");
    }

    #[test]
    fn ellipsize_to_budget_passthrough_when_fits() {
        assert_eq!(ellipsize_to_budget("hello", 10), "hello");
        assert_eq!(ellipsize_to_budget("hello", 5), "hello");
    }

    #[test]
    fn ellipsize_to_budget_appends_ellipsis_on_overflow() {
        let out = ellipsize_to_budget("the quick brown fox", 10);
        assert_eq!(out.chars().count(), 10);
        assert!(out.ends_with('…'));
        assert!(out.starts_with("the quick"));
    }

    #[test]
    fn ellipsize_to_budget_handles_zero_and_one() {
        assert_eq!(ellipsize_to_budget("hello", 0), "");
        assert_eq!(ellipsize_to_budget("hello", 1), "…");
    }
}
