//! Skill discovery from the filesystem.
//!
//! Scans well-known directories for SKILL.md files and extracts metadata
//! from their YAML frontmatter.

use serde::Serialize;

/// Directories to scan for skills (in priority order; first directory wins on dedup).
const SKILL_SCAN_DIRS: &[&str] = &[
    "~/.agents/skills",
    "~/.claude/skills",
    "~/.cursor/skills",
    "~/.config/opencode/skills",
];

/// A discovered skill from the filesystem.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInfo {
    /// Kebab-cased directory name (e.g., "brainstorming")
    pub id: String,
    /// From SKILL.md frontmatter `name` field
    pub name: String,
    /// From SKILL.md frontmatter `description` field
    pub description: String,
    /// Which base directory it came from
    pub source_dir: String,
}

/// Discover skills from well-known directories.
///
/// Scans each directory in [`SKILL_SCAN_DIRS`] for subdirectories containing
/// a `SKILL.md` file with valid YAML frontmatter. Skills are deduplicated
/// by ID — the first directory wins.
pub fn discover_skills() -> Vec<SkillInfo> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return vec![],
    };

    let mut skills = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for dir_pattern in SKILL_SCAN_DIRS {
        let dir = std::path::PathBuf::from(dir_pattern.replace('~', &home.to_string_lossy()));

        if !dir.is_dir() {
            continue;
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let skill_md = path.join("SKILL.md");
            if !skill_md.is_file() {
                continue;
            }

            let id = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            // Dedup: first directory wins
            if seen_ids.contains(&id) {
                continue;
            }

            let content = match std::fs::read_to_string(&skill_md) {
                Ok(c) => c,
                Err(_) => continue,
            };

            if let Some((name, description)) = parse_frontmatter(&content) {
                seen_ids.insert(id.clone());
                skills.push(SkillInfo {
                    id,
                    name,
                    description,
                    source_dir: dir.to_string_lossy().to_string(),
                });
            }
        }
    }

    // Sort by name for stable output
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// Parse YAML frontmatter from a SKILL.md file.
///
/// Extracts `name` and `description` from the `---` delimited frontmatter block.
/// Returns `None` if the frontmatter is missing or malformed.
fn parse_frontmatter(content: &str) -> Option<(String, String)> {
    let content = content.trim();
    if !content.starts_with("---") {
        return None;
    }

    // Find closing ---
    let rest = &content[3..];
    let end = rest.find("---")?;
    let frontmatter = &rest[..end];

    let mut name = None;
    let mut description = None;
    let mut in_multiline_desc = false;
    let mut multiline_desc = String::new();
    let mut multiline_indent: Option<usize> = None;

    for line in frontmatter.lines() {
        // Handle multiline description (YAML block scalar with |)
        if in_multiline_desc {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                // Empty line in block scalar
                if !multiline_desc.is_empty() {
                    multiline_desc.push(' ');
                }
                continue;
            }

            // Determine indent level on first content line
            let indent = line.len() - line.trim_start().len();
            if multiline_indent.is_none() {
                multiline_indent = Some(indent);
            }

            // If this line starts a new key (no indent or less indent), stop multiline
            if indent == 0 && line.contains(':') && !line.starts_with(' ') {
                in_multiline_desc = false;
                // Process this line as a normal key
            } else {
                // Append to multiline description
                if !multiline_desc.is_empty() {
                    multiline_desc.push(' ');
                }
                multiline_desc.push_str(trimmed);
                continue;
            }
        }

        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("name:") {
            let value = stripped.trim();
            if !value.is_empty() {
                name = Some(value.to_string());
            }
        } else if let Some(stripped) = trimmed.strip_prefix("description:") {
            let value = stripped.trim();
            if value == "|" || value == "|-" || value == "|+" {
                // Block scalar — read subsequent indented lines
                in_multiline_desc = true;
                multiline_desc.clear();
                multiline_indent = None;
            } else if !value.is_empty() {
                description = Some(value.to_string());
            }
        }
    }

    // If we were reading a multiline description, finalize it
    if in_multiline_desc && !multiline_desc.is_empty() {
        description = Some(multiline_desc);
    }

    match (name, description) {
        (Some(n), Some(d)) => Some((n, d)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_skill(dir: &std::path::Path, name: &str, content: &str) {
        let skill_dir = dir.join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn test_parse_frontmatter_basic() {
        let content = r#"---
name: brainstorming
description: Explore user intent and design before implementation
---
# Rest of content
"#;
        let (name, desc) = parse_frontmatter(content).unwrap();
        assert_eq!(name, "brainstorming");
        assert_eq!(desc, "Explore user intent and design before implementation");
    }

    #[test]
    fn test_parse_frontmatter_multiline() {
        let content = r#"---
name: mcp-server
description: |
  Build MCP servers using the TypeScript SDK. Use when:
  (1) Creating a new MCP server from scratch
  (2) Adding tools or resources
---
"#;
        let (name, desc) = parse_frontmatter(content).unwrap();
        assert_eq!(name, "mcp-server");
        assert!(desc.contains("Build MCP servers"));
        assert!(desc.contains("Adding tools or resources"));
    }

    #[test]
    fn test_parse_frontmatter_missing_name() {
        let content = r#"---
description: Some description
---
"#;
        assert!(parse_frontmatter(content).is_none());
    }

    #[test]
    fn test_parse_frontmatter_missing_description() {
        let content = r#"---
name: test-skill
---
"#;
        assert!(parse_frontmatter(content).is_none());
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let content = "# Just markdown\nNo frontmatter here.";
        assert!(parse_frontmatter(content).is_none());
    }

    #[test]
    fn test_parse_frontmatter_empty() {
        let content = "";
        assert!(parse_frontmatter(content).is_none());
    }

    #[test]
    fn test_parse_frontmatter_no_closing_delimiter() {
        let content = "---\nname: test\ndescription: test\n";
        assert!(parse_frontmatter(content).is_none());
    }

    #[test]
    fn test_discover_skills_with_temp_dir() {
        // This test verifies the parse_frontmatter + file reading logic works end-to-end
        // We can't easily test discover_skills() directly since it reads from fixed paths,
        // but we can verify the parsing pipeline.
        let dir = TempDir::new().unwrap();

        create_skill(
            dir.path(),
            "test-skill",
            "---\nname: test-skill\ndescription: A test skill\n---\n# Content",
        );

        // Read and parse directly
        let content = fs::read_to_string(dir.path().join("test-skill").join("SKILL.md")).unwrap();
        let (name, desc) = parse_frontmatter(&content).unwrap();
        assert_eq!(name, "test-skill");
        assert_eq!(desc, "A test skill");
    }

    #[test]
    fn test_parse_frontmatter_with_extra_fields() {
        let content = r#"---
name: skill-creator
description: Guide for creating effective skills
license: Complete terms in LICENSE.txt
---
"#;
        let (name, desc) = parse_frontmatter(content).unwrap();
        assert_eq!(name, "skill-creator");
        assert_eq!(desc, "Guide for creating effective skills");
    }
}
