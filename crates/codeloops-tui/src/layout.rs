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
}
