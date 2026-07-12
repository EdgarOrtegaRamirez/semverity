//! Package file analysis — parse dependency version constraints from
//! package.json, Cargo.toml, pyproject.toml, and go.mod files.

use crate::ecosystem::{parse_range, Ecosystem};
use crate::range::VersionRange;
use std::collections::BTreeMap;
use std::path::Path;

/// A dependency entry found in a package file.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub constraint: String,
    pub ecosystem: Ecosystem,
    pub section: String, // e.g., "dependencies", "devDependencies"
}

/// Analysis result for a package file.
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub file: String,
    pub ecosystem: Ecosystem,
    pub dependencies: Vec<Dependency>,
    pub conflicts: Vec<Conflict>,
}

/// A version conflict between two constraints on the same package.
#[derive(Debug, Clone)]
pub struct Conflict {
    pub package: String,
    pub constraints: Vec<String>,
    #[allow(dead_code)]
    pub intersection_empty: bool,
}

/// Analyze a package file for version constraints.
///
/// # Errors
/// Returns an error if the file cannot be read or parsed.
pub fn analyze_file(path: &Path) -> Result<AnalysisResult, AnalyzeError> {
    let content = std::fs::read_to_string(path).map_err(|e| AnalyzeError::Io(e.to_string()))?;

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    match name {
        "package.json" => analyze_package_json(&content, path),
        "Cargo.toml" => analyze_cargo_toml(&content, path),
        "pyproject.toml" => analyze_pyproject_toml(&content, path),
        "go.mod" => analyze_go_mod(&content, path),
        _ => {
            // Try by extension or content
            if ext == "json" {
                analyze_package_json(&content, path)
            } else if ext == "toml" {
                // Could be Cargo.toml or pyproject.toml
                if content.contains("[package]") || content.contains("[dependencies]") {
                    analyze_cargo_toml(&content, path)
                } else {
                    analyze_pyproject_toml(&content, path)
                }
            } else if content.contains("module ") && content.contains("go ") {
                analyze_go_mod(&content, path)
            } else {
                Err(AnalyzeError::UnknownFormat(
                    "could not determine file type".to_string(),
                ))
            }
        }
    }
}

/// Analyze a package.json file (npm/Node.js).
fn analyze_package_json(content: &str, path: &Path) -> Result<AnalysisResult, AnalyzeError> {
    let json: serde_json::Value =
        serde_json::from_str(content).map_err(|e| AnalyzeError::Parse(e.to_string()))?;

    let mut deps = Vec::new();

    let sections = [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ];

    if let Some(obj) = json.as_object() {
        for section in &sections {
            if let Some(section_deps) = obj.get(*section).and_then(|v| v.as_object()) {
                for (name, version) in section_deps {
                    if let Some(version_str) = version.as_str() {
                        deps.push(Dependency {
                            name: name.clone(),
                            constraint: version_str.to_string(),
                            ecosystem: Ecosystem::Npm,
                            section: section.to_string(),
                        });
                    }
                }
            }
        }
    }

    let conflicts = detect_conflicts(&deps, path);
    Ok(AnalysisResult {
        file: path.to_string_lossy().to_string(),
        ecosystem: Ecosystem::Npm,
        dependencies: deps,
        conflicts,
    })
}

/// Analyze a Cargo.toml file (Rust).
fn analyze_cargo_toml(content: &str, path: &Path) -> Result<AnalysisResult, AnalyzeError> {
    let mut deps = Vec::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let line = line.trim();

        // Section header
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            continue;
        }

        // Dependency entry: name = "version" or name = { version = "x", ... }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().trim_matches('"').to_string();
            let value = line[eq_pos + 1..].trim();

            // Only parse dependency sections
            if current_section == "dependencies"
                || current_section == "dev-dependencies"
                || current_section == "build-dependencies"
                || current_section.starts_with("dependencies.")
            {
                // Simple string version
                if value.starts_with('"') {
                    let version = value.trim_matches('"').to_string();
                    if !version.is_empty() {
                        deps.push(Dependency {
                            name: key,
                            constraint: version,
                            ecosystem: Ecosystem::Cargo,
                            section: current_section.clone(),
                        });
                    }
                } else if value.starts_with('{') {
                    // Inline table: extract version = "x"
                    if let Some(vstart) = value.find("version") {
                        let rest = &value[vstart..];
                        if let Some(q1) = rest.find('"') {
                            if let Some(q2) = rest[q1 + 1..].find('"') {
                                let version = &rest[q1 + 1..q1 + 1 + q2];
                                deps.push(Dependency {
                                    name: key,
                                    constraint: version.to_string(),
                                    ecosystem: Ecosystem::Cargo,
                                    section: current_section.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    let conflicts = detect_conflicts(&deps, path);
    Ok(AnalysisResult {
        file: path.to_string_lossy().to_string(),
        ecosystem: Ecosystem::Cargo,
        dependencies: deps,
        conflicts,
    })
}

/// Analyze a pyproject.toml file (Python).
fn analyze_pyproject_toml(content: &str, path: &Path) -> Result<AnalysisResult, AnalyzeError> {
    let mut deps = Vec::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            continue;
        }

        // project.dependencies section (PEP 621)
        if current_section == "project" && line.starts_with("dependencies") {
            // Parse array of "package>=1.0,<2.0" strings
            if let Some(start) = line.find('[') {
                let array_str = &line[start..];
                parse_pip_dep_array(array_str, &mut deps, "dependencies");
            }
        }

        // [project.optional-dependencies] sections
        if current_section.starts_with("project.optional-dependencies") {
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let value = line[eq_pos + 1..].trim();
                if value.starts_with('[') {
                    parse_pip_dep_array(value, &mut deps, &format!("optional-dependencies.{key}"));
                }
            }
        }

        // [tool.poetry.dependencies] sections
        if current_section.starts_with("tool.poetry") {
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().trim_matches('"').to_string();
                let value = line[eq_pos + 1..].trim();

                if current_section == "tool.poetry.dependencies"
                    || current_section.starts_with("tool.poetry.group")
                {
                    // Simple string version
                    if value.starts_with('"') {
                        let version = value.trim_matches('"').to_string();
                        if !version.is_empty() {
                            deps.push(Dependency {
                                name: key,
                                constraint: version,
                                ecosystem: Ecosystem::Pip,
                                section: current_section.clone(),
                            });
                        }
                    } else if value.starts_with('{') {
                        // Table: extract version = "x"
                        if let Some(vstart) = value.find("version") {
                            let rest = &value[vstart..];
                            if let Some(q1) = rest.find('"') {
                                if let Some(q2) = rest[q1 + 1..].find('"') {
                                    let version = &rest[q1 + 1..q1 + 1 + q2];
                                    deps.push(Dependency {
                                        name: key,
                                        constraint: version.to_string(),
                                        ecosystem: Ecosystem::Pip,
                                        section: current_section.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let conflicts = detect_conflicts(&deps, path);
    Ok(AnalysisResult {
        file: path.to_string_lossy().to_string(),
        ecosystem: Ecosystem::Pip,
        dependencies: deps,
        conflicts,
    })
}

/// Parse a PEP 621 dependency array (e.g., ["package>=1.0,<2.0", "other~=3.0"]).
fn parse_pip_dep_array(array_str: &str, deps: &mut Vec<Dependency>, section: &str) {
    // Extract strings between quotes
    let mut in_string = false;
    let mut current = String::new();
    for ch in array_str.chars() {
        if ch == '"' {
            if in_string {
                // End of string
                if let Some(dep) = parse_pip_dep_string(&current) {
                    deps.push(Dependency {
                        name: dep.0,
                        constraint: dep.1,
                        ecosystem: Ecosystem::Pip,
                        section: section.to_string(),
                    });
                }
                current.clear();
            }
            in_string = !in_string;
        } else if in_string {
            current.push(ch);
        }
    }
}

/// Parse a PEP 508 dependency string like "package>=1.0,<2.0" or "package[extra]>=1.0".
/// Returns (name, constraint) or None.
fn parse_pip_dep_string(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // Find where the version constraint starts (first occurrence of an operator)
    let ops = [">=", "<=", "~=", "==", "!=", ">", "<", "="];
    let mut split_pos = None;
    for op in &ops {
        if let Some(pos) = s.find(op) {
            if split_pos.is_none() || pos < split_pos.unwrap() {
                split_pos = Some(pos);
            }
        }
    }

    match split_pos {
        Some(pos) => {
            let name = s[..pos].trim().to_string();
            let constraint = s[pos..].trim().to_string();
            if name.is_empty() || constraint.is_empty() {
                None
            } else {
                // Strip extras like package[extra]
                let name = name.split('[').next().unwrap_or(&name).to_string();
                Some((name, constraint))
            }
        }
        None => {
            // No version constraint — just a package name
            let name = s.split('[').next().unwrap_or(s).trim().to_string();
            if name.is_empty() {
                None
            } else {
                Some((name, "*".to_string()))
            }
        }
    }
}

/// Analyze a go.mod file (Go).
fn analyze_go_mod(content: &str, path: &Path) -> Result<AnalysisResult, AnalyzeError> {
    let mut deps = Vec::new();
    let mut in_require_block = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "require (" {
            in_require_block = true;
            continue;
        }
        if in_require_block && line == ")" {
            in_require_block = false;
            continue;
        }

        // Single-line require
        if line.starts_with("require ") {
            let rest = line["require ".len()..].trim();
            if let Some(dep) = parse_go_dep(rest) {
                deps.push(dep);
            }
        } else if in_require_block && !line.is_empty() && !line.starts_with("//") {
            if let Some(dep) = parse_go_dep(line) {
                deps.push(dep);
            }
        }
    }

    let conflicts = detect_conflicts(&deps, path);
    Ok(AnalysisResult {
        file: path.to_string_lossy().to_string(),
        ecosystem: Ecosystem::Cargo, // Go doesn't use semver ranges, but we store as-is
        dependencies: deps,
        conflicts,
    })
}

/// Parse a Go module dependency line: "github.com/pkg/errors v1.0.0"
fn parse_go_dep(line: &str) -> Option<Dependency> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let name = parts[0].to_string();
    let version = parts[1].to_string();
    // Go uses pseudo-versions and exact versions, not ranges
    // Strip the "v" prefix for consistency
    let constraint = version.strip_prefix('v').unwrap_or(&version).to_string();
    Some(Dependency {
        name,
        constraint,
        ecosystem: Ecosystem::Cargo, // Reuse for display
        section: "require".to_string(),
    })
}

/// Detect conflicts: same package with incompatible version constraints.
fn detect_conflicts(deps: &[Dependency], _path: &Path) -> Vec<Conflict> {
    // Group by package name
    let mut groups: BTreeMap<String, Vec<&Dependency>> = BTreeMap::new();
    for dep in deps {
        groups.entry(dep.name.clone()).or_default().push(dep);
    }

    let mut conflicts = Vec::new();
    for (name, group) in &groups {
        if group.len() < 2 {
            continue;
        }

        // Check if all constraints can be simultaneously satisfied
        let mut combined = VersionRange::all();
        for dep in group {
            if let Ok(range) = parse_range(&dep.constraint, dep.ecosystem) {
                combined = combined.intersect(&range);
            }
        }

        if combined.is_empty() {
            conflicts.push(Conflict {
                package: name.clone(),
                constraints: group.iter().map(|d| d.constraint.clone()).collect(),
                intersection_empty: true,
            });
        }
    }

    conflicts
}

/// Errors during analysis.
#[derive(Debug)]
pub enum AnalyzeError {
    Io(String),
    Parse(String),
    UnknownFormat(String),
}

impl std::fmt::Display for AnalyzeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "IO error: {msg}"),
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
            Self::UnknownFormat(msg) => write!(f, "unknown format: {msg}"),
        }
    }
}

impl std::error::Error for AnalyzeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_temp_file(name: &str, content: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("semverity_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_analyze_package_json() {
        let content = r#"{
            "dependencies": {
                "express": "^4.18.0",
                "lodash": "~4.17.21"
            },
            "devDependencies": {
                "jest": ">=29.0.0 <30.0.0"
            }
        }"#;
        let path = create_temp_file("test_package.json", content);
        let result = analyze_file(&path).unwrap();
        assert_eq!(result.ecosystem, Ecosystem::Npm);
        assert_eq!(result.dependencies.len(), 3);
        assert!(result.dependencies.iter().any(|d| d.name == "express"));
        assert!(result.dependencies.iter().any(|d| d.name == "lodash"));
        assert!(result.dependencies.iter().any(|d| d.name == "jest"));
    }

    #[test]
    fn test_analyze_cargo_toml() {
        let content = r#"
[package]
name = "myapp"
version = "1.0.0"

[dependencies]
serde = "1.0"
clap = { version = "4.5", features = ["derive"] }

[dev-dependencies]
pytest = "0.1"
"#;
        let path = create_temp_file("test_Cargo.toml", content);
        let result = analyze_file(&path).unwrap();
        assert_eq!(result.ecosystem, Ecosystem::Cargo);
        assert!(result
            .dependencies
            .iter()
            .any(|d| d.name == "serde" && d.constraint == "1.0"));
        assert!(result
            .dependencies
            .iter()
            .any(|d| d.name == "clap" && d.constraint == "4.5"));
        assert!(result.dependencies.iter().any(|d| d.name == "pytest"));
    }

    #[test]
    fn test_analyze_go_mod() {
        let content = r#"
module github.com/example/myapp

go 1.21

require (
    github.com/pkg/errors v0.9.1
    github.com/spf13/cobra v1.8.0
)
"#;
        let path = create_temp_file("test_go.mod", content);
        let result = analyze_file(&path).unwrap();
        assert!(result
            .dependencies
            .iter()
            .any(|d| d.name == "github.com/pkg/errors"));
        assert!(result
            .dependencies
            .iter()
            .any(|d| d.name == "github.com/spf13/cobra"));
    }

    #[test]
    fn test_parse_pip_dep_string() {
        let (name, constraint) = parse_pip_dep_string("requests>=2.28.0,<3.0.0").unwrap();
        assert_eq!(name, "requests");
        assert_eq!(constraint, ">=2.28.0,<3.0.0");

        let (name, constraint) = parse_pip_dep_string("numpy~=1.24").unwrap();
        assert_eq!(name, "numpy");
        assert_eq!(constraint, "~=1.24");

        let (name, constraint) = parse_pip_dep_string("flask").unwrap();
        assert_eq!(name, "flask");
        assert_eq!(constraint, "*");

        // With extras
        let (name, _) = parse_pip_dep_string("package[extra]>=1.0").unwrap();
        assert_eq!(name, "package");
    }

    #[test]
    fn test_detect_conflicts() {
        let deps = vec![
            Dependency {
                name: "pkg".to_string(),
                constraint: ">=2.0.0".to_string(),
                ecosystem: Ecosystem::Npm,
                section: "dependencies".to_string(),
            },
            Dependency {
                name: "pkg".to_string(),
                constraint: "<1.0.0".to_string(),
                ecosystem: Ecosystem::Npm,
                section: "devDependencies".to_string(),
            },
        ];
        let conflicts = detect_conflicts(&deps, Path::new("test"));
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].package, "pkg");
        assert!(conflicts[0].intersection_empty);
    }

    #[test]
    fn test_no_conflict() {
        let deps = vec![
            Dependency {
                name: "pkg".to_string(),
                constraint: ">=1.0.0".to_string(),
                ecosystem: Ecosystem::Npm,
                section: "dependencies".to_string(),
            },
            Dependency {
                name: "pkg".to_string(),
                constraint: "<2.0.0".to_string(),
                ecosystem: Ecosystem::Npm,
                section: "devDependencies".to_string(),
            },
        ];
        let conflicts = detect_conflicts(&deps, Path::new("test"));
        assert!(conflicts.is_empty());
    }
}
