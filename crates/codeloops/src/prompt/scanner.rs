//! Project scanner for gathering context before the interview.
//!
//! Scans the project directory to detect the project type, languages used,
//! frameworks, and key files that provide context for prompt generation.

use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Context gathered from scanning a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    /// Detected project type
    pub project_type: ProjectType,
    /// Primary programming languages detected
    pub languages: Vec<String>,
    /// Frameworks or libraries detected
    pub frameworks: Vec<String>,
    /// Key files that provide context
    pub key_files: Vec<KeyFile>,
    /// Directory structure summary
    pub directory_structure: Vec<String>,
    /// Project name (from config file or directory name)
    pub project_name: Option<String>,
    /// Project description (from config file)
    pub project_description: Option<String>,
}

/// Type of project detected
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Unknown,
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectType::Rust => write!(f, "Rust"),
            ProjectType::Node => write!(f, "Node.js"),
            ProjectType::Python => write!(f, "Python"),
            ProjectType::Go => write!(f, "Go"),
            ProjectType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// A key file in the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFile {
    /// Path relative to project root
    pub path: String,
    /// Description of what this file contains
    pub description: String,
}

impl ProjectContext {
    /// Format the project context for inclusion in the agent prompt
    pub fn to_prompt_context(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref name) = self.project_name {
            parts.push(format!("Project: {}", name));
        }

        parts.push(format!("Type: {}", self.project_type));

        if !self.languages.is_empty() {
            parts.push(format!("Languages: {}", self.languages.join(", ")));
        }

        if !self.frameworks.is_empty() {
            parts.push(format!("Frameworks: {}", self.frameworks.join(", ")));
        }

        if let Some(ref desc) = self.project_description {
            parts.push(format!("Description: {}", desc));
        }

        if !self.key_files.is_empty() {
            parts.push("Key Files:".to_string());
            for kf in &self.key_files {
                parts.push(format!("  - {}: {}", kf.path, kf.description));
            }
        }

        if !self.directory_structure.is_empty() {
            parts.push("Structure:".to_string());
            for dir in &self.directory_structure {
                parts.push(format!("  {}", dir));
            }
        }

        parts.join("\n")
    }
}

/// Scan a project directory to gather context
pub fn scan_project(working_dir: &Path) -> Result<ProjectContext> {
    let mut context = ProjectContext {
        project_type: ProjectType::Unknown,
        languages: Vec::new(),
        frameworks: Vec::new(),
        key_files: Vec::new(),
        directory_structure: Vec::new(),
        project_name: None,
        project_description: None,
    };

    // Detect project type and gather info from config files
    if working_dir.join("Cargo.toml").exists() {
        context.project_type = ProjectType::Rust;
        context.languages.push("Rust".to_string());
        scan_cargo_toml(working_dir, &mut context)?;
    } else if working_dir.join("package.json").exists() {
        context.project_type = ProjectType::Node;
        scan_package_json(working_dir, &mut context)?;
    } else if working_dir.join("pyproject.toml").exists()
        || working_dir.join("setup.py").exists()
        || working_dir.join("requirements.txt").exists()
    {
        context.project_type = ProjectType::Python;
        context.languages.push("Python".to_string());
        scan_python_project(working_dir, &mut context)?;
    } else if working_dir.join("go.mod").exists() {
        context.project_type = ProjectType::Go;
        context.languages.push("Go".to_string());
        scan_go_mod(working_dir, &mut context)?;
    }

    // Scan directory structure
    scan_directory_structure(working_dir, &mut context)?;

    // Detect additional key files
    detect_key_files(working_dir, &mut context);

    // Fallback to directory name if no project name found
    if context.project_name.is_none() {
        context.project_name = working_dir
            .file_name()
            .and_then(|n| n.to_str())
            .map(String::from);
    }

    Ok(context)
}

fn scan_cargo_toml(working_dir: &Path, context: &mut ProjectContext) -> Result<()> {
    let content = std::fs::read_to_string(working_dir.join("Cargo.toml"))?;

    // Parse as TOML to extract name and description
    if let Ok(value) = content.parse::<toml::Value>() {
        if let Some(package) = value.get("package") {
            if let Some(name) = package.get("name").and_then(|v| v.as_str()) {
                context.project_name = Some(name.to_string());
            }
            if let Some(desc) = package.get("description").and_then(|v| v.as_str()) {
                context.project_description = Some(desc.to_string());
            }
        }

        // Detect frameworks from dependencies
        let deps = value.get("dependencies");
        let dev_deps = value.get("dev-dependencies");

        for deps_table in [deps, dev_deps].into_iter().flatten() {
            if let Some(table) = deps_table.as_table() {
                for key in table.keys() {
                    match key.as_str() {
                        "tokio" => context.frameworks.push("Tokio".to_string()),
                        "axum" => context.frameworks.push("Axum".to_string()),
                        "actix-web" => context.frameworks.push("Actix Web".to_string()),
                        "rocket" => context.frameworks.push("Rocket".to_string()),
                        "serde" => context.frameworks.push("Serde".to_string()),
                        "clap" => context.frameworks.push("Clap".to_string()),
                        "ratatui" => context.frameworks.push("Ratatui".to_string()),
                        _ => {}
                    }
                }
            }
        }
    }

    context.key_files.push(KeyFile {
        path: "Cargo.toml".to_string(),
        description: "Rust project manifest".to_string(),
    });

    Ok(())
}

fn scan_package_json(working_dir: &Path, context: &mut ProjectContext) -> Result<()> {
    let content = std::fs::read_to_string(working_dir.join("package.json"))?;

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
        if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
            context.project_name = Some(name.to_string());
        }
        if let Some(desc) = value.get("description").and_then(|v| v.as_str()) {
            context.project_description = Some(desc.to_string());
        }

        // Detect languages
        context.languages.push("JavaScript".to_string());
        if working_dir.join("tsconfig.json").exists() {
            context.languages.push("TypeScript".to_string());
        }

        // Detect frameworks from dependencies
        let deps = value.get("dependencies");
        let dev_deps = value.get("devDependencies");

        for deps_obj in [deps, dev_deps].into_iter().flatten() {
            if let Some(obj) = deps_obj.as_object() {
                for key in obj.keys() {
                    match key.as_str() {
                        "react" => context.frameworks.push("React".to_string()),
                        "vue" => context.frameworks.push("Vue".to_string()),
                        "svelte" => context.frameworks.push("Svelte".to_string()),
                        "next" => context.frameworks.push("Next.js".to_string()),
                        "express" => context.frameworks.push("Express".to_string()),
                        "fastify" => context.frameworks.push("Fastify".to_string()),
                        "nestjs" | "@nestjs/core" => context.frameworks.push("NestJS".to_string()),
                        _ => {}
                    }
                }
            }
        }
    }

    context.key_files.push(KeyFile {
        path: "package.json".to_string(),
        description: "Node.js project manifest".to_string(),
    });

    Ok(())
}

fn scan_python_project(working_dir: &Path, context: &mut ProjectContext) -> Result<()> {
    // Try pyproject.toml first
    if working_dir.join("pyproject.toml").exists() {
        let content = std::fs::read_to_string(working_dir.join("pyproject.toml"))?;

        if let Ok(value) = content.parse::<toml::Value>() {
            if let Some(project) = value.get("project") {
                if let Some(name) = project.get("name").and_then(|v| v.as_str()) {
                    context.project_name = Some(name.to_string());
                }
                if let Some(desc) = project.get("description").and_then(|v| v.as_str()) {
                    context.project_description = Some(desc.to_string());
                }
            }

            // Check for poetry
            if value.get("tool").and_then(|t| t.get("poetry")).is_some() {
                context.frameworks.push("Poetry".to_string());
            }
        }

        context.key_files.push(KeyFile {
            path: "pyproject.toml".to_string(),
            description: "Python project manifest".to_string(),
        });
    }

    // Check for common frameworks in requirements.txt
    if working_dir.join("requirements.txt").exists() {
        let content = std::fs::read_to_string(working_dir.join("requirements.txt"))?;

        for line in content.lines() {
            let pkg = line.split(['=', '>', '<', '[', ' ']).next().unwrap_or("");
            match pkg.to_lowercase().as_str() {
                "django" => context.frameworks.push("Django".to_string()),
                "flask" => context.frameworks.push("Flask".to_string()),
                "fastapi" => context.frameworks.push("FastAPI".to_string()),
                "pytest" => context.frameworks.push("pytest".to_string()),
                _ => {}
            }
        }

        context.key_files.push(KeyFile {
            path: "requirements.txt".to_string(),
            description: "Python dependencies".to_string(),
        });
    }

    Ok(())
}

fn scan_go_mod(working_dir: &Path, context: &mut ProjectContext) -> Result<()> {
    let content = std::fs::read_to_string(working_dir.join("go.mod"))?;

    // Extract module name from first line
    for line in content.lines() {
        if line.starts_with("module ") {
            let module = line.strip_prefix("module ").unwrap_or("").trim();
            // Use the last part of the module path as project name
            context.project_name = module.split('/').next_back().map(String::from);
            break;
        }
    }

    // Detect common frameworks
    if content.contains("github.com/gin-gonic/gin") {
        context.frameworks.push("Gin".to_string());
    }
    if content.contains("github.com/labstack/echo") {
        context.frameworks.push("Echo".to_string());
    }
    if content.contains("github.com/gofiber/fiber") {
        context.frameworks.push("Fiber".to_string());
    }

    context.key_files.push(KeyFile {
        path: "go.mod".to_string(),
        description: "Go module definition".to_string(),
    });

    Ok(())
}

fn scan_directory_structure(working_dir: &Path, context: &mut ProjectContext) -> Result<()> {
    // List top-level directories (excluding common non-essential ones)
    let ignore = [
        "node_modules",
        "target",
        ".git",
        ".venv",
        "venv",
        "__pycache__",
        ".idea",
        ".vscode",
        "dist",
        "build",
        ".next",
        "coverage",
    ];

    if let Ok(entries) = std::fs::read_dir(working_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !ignore.contains(&name) && !name.starts_with('.') {
                        context.directory_structure.push(format!("{}/", name));
                    }
                }
            }
        }
    }

    context.directory_structure.sort();

    Ok(())
}

fn detect_key_files(working_dir: &Path, context: &mut ProjectContext) {
    let key_file_patterns = [
        ("README.md", "Project documentation"),
        ("README", "Project documentation"),
        (".env.example", "Environment variable template"),
        ("docker-compose.yml", "Docker Compose configuration"),
        ("docker-compose.yaml", "Docker Compose configuration"),
        ("Dockerfile", "Docker build configuration"),
        ("Makefile", "Build automation"),
        (".github/workflows", "GitHub Actions workflows"),
        ("src/main.rs", "Rust application entry point"),
        ("src/lib.rs", "Rust library entry point"),
        ("src/index.ts", "TypeScript entry point"),
        ("src/index.js", "JavaScript entry point"),
        ("src/main.py", "Python entry point"),
        ("main.go", "Go entry point"),
        ("cmd/", "Go command packages"),
    ];

    for (pattern, description) in key_file_patterns {
        let path = working_dir.join(pattern);
        if path.exists() {
            // Avoid duplicates
            if !context.key_files.iter().any(|kf| kf.path == pattern) {
                context.key_files.push(KeyFile {
                    path: pattern.to_string(),
                    description: description.to_string(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_detect_rust_project() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test-project"
description = "A test project"
version = "0.1.0"

[dependencies]
tokio = "1.0"
serde = "1.0"
"#,
        )
        .unwrap();

        let context = scan_project(dir.path()).unwrap();
        assert_eq!(context.project_type, ProjectType::Rust);
        assert_eq!(context.project_name, Some("test-project".to_string()));
        assert_eq!(
            context.project_description,
            Some("A test project".to_string())
        );
        assert!(context.languages.contains(&"Rust".to_string()));
        assert!(context.frameworks.contains(&"Tokio".to_string()));
        assert!(context.frameworks.contains(&"Serde".to_string()));
    }

    #[test]
    fn test_detect_node_project() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{
  "name": "my-app",
  "description": "A Node.js app",
  "dependencies": {
    "react": "^18.0.0",
    "express": "^4.0.0"
  }
}"#,
        )
        .unwrap();

        let context = scan_project(dir.path()).unwrap();
        assert_eq!(context.project_type, ProjectType::Node);
        assert_eq!(context.project_name, Some("my-app".to_string()));
        assert!(context.languages.contains(&"JavaScript".to_string()));
        assert!(context.frameworks.contains(&"React".to_string()));
        assert!(context.frameworks.contains(&"Express".to_string()));
    }

    #[test]
    fn test_detect_go_project() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("go.mod"),
            r#"module github.com/user/myproject

go 1.21

require github.com/gin-gonic/gin v1.9.0
"#,
        )
        .unwrap();

        let context = scan_project(dir.path()).unwrap();
        assert_eq!(context.project_type, ProjectType::Go);
        assert_eq!(context.project_name, Some("myproject".to_string()));
        assert!(context.languages.contains(&"Go".to_string()));
        assert!(context.frameworks.contains(&"Gin".to_string()));
    }

    #[test]
    fn test_to_prompt_context() {
        let context = ProjectContext {
            project_type: ProjectType::Rust,
            languages: vec!["Rust".to_string()],
            frameworks: vec!["Tokio".to_string(), "Axum".to_string()],
            key_files: vec![KeyFile {
                path: "Cargo.toml".to_string(),
                description: "Rust manifest".to_string(),
            }],
            directory_structure: vec!["src/".to_string(), "tests/".to_string()],
            project_name: Some("myproject".to_string()),
            project_description: Some("A cool project".to_string()),
        };

        let prompt = context.to_prompt_context();
        assert!(prompt.contains("Project: myproject"));
        assert!(prompt.contains("Type: Rust"));
        assert!(prompt.contains("Languages: Rust"));
        assert!(prompt.contains("Frameworks: Tokio, Axum"));
        assert!(prompt.contains("Description: A cool project"));
    }
}
