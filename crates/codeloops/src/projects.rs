//! Project matching utilities.
//!
//! Contains a plain function for matching filesystem paths to registered projects.
//! No trait, no chain, no scoring — just path prefix comparison.

use std::path::Path;

use codeloops_db::ProjectRecord;

/// Match a filesystem path to a registered project.
///
/// Checks if `path` is equal to or a subdirectory of any project's path.
/// When multiple projects match (e.g., `/a` and `/a/b` both match `/a/b/c`),
/// the deepest (most specific) project wins.
pub fn find_project_for_path(path: &Path, projects: &[ProjectRecord]) -> Option<ProjectRecord> {
    let path_str = path.to_string_lossy();

    projects
        .iter()
        .filter(|p| {
            path_str == p.path
                || path_str.starts_with(&format!("{}/", p.path))
        })
        .max_by_key(|p| p.path.len())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_project(path: &str) -> ProjectRecord {
        let now = Utc::now();
        ProjectRecord {
            id: format!("proj-{}", path.replace('/', "-")),
            path: path.to_string(),
            name: path.rsplit('/').next().unwrap_or("unknown").to_string(),
            config_overrides: None,
            is_default: false,
            created_at: now,
            last_accessed_at: now,
        }
    }

    #[test]
    fn test_exact_match() {
        let projects = vec![make_project("/home/user/project")];
        let result = find_project_for_path(Path::new("/home/user/project"), &projects);
        assert!(result.is_some());
        assert_eq!(result.unwrap().path, "/home/user/project");
    }

    #[test]
    fn test_subdirectory_match() {
        let projects = vec![make_project("/home/user/project")];
        let result = find_project_for_path(Path::new("/home/user/project/src/main.rs"), &projects);
        assert!(result.is_some());
        assert_eq!(result.unwrap().path, "/home/user/project");
    }

    #[test]
    fn test_no_match() {
        let projects = vec![make_project("/home/user/project")];
        let result = find_project_for_path(Path::new("/home/user/other"), &projects);
        assert!(result.is_none());
    }

    #[test]
    fn test_deepest_match_wins() {
        let projects = vec![
            make_project("/home/user"),
            make_project("/home/user/project"),
            make_project("/home/user/project/packages/core"),
        ];
        let result =
            find_project_for_path(Path::new("/home/user/project/packages/core/src"), &projects);
        assert!(result.is_some());
        assert_eq!(result.unwrap().path, "/home/user/project/packages/core");
    }

    #[test]
    fn test_no_false_prefix_match() {
        // /home/user/project-v2 should NOT match /home/user/project
        let projects = vec![make_project("/home/user/project")];
        let result = find_project_for_path(Path::new("/home/user/project-v2"), &projects);
        assert!(result.is_none());
    }

    #[test]
    fn test_empty_projects() {
        let result = find_project_for_path(Path::new("/home/user/project"), &[]);
        assert!(result.is_none());
    }
}
