//! Orb parser module for parsing CircleCI orb YAML definitions.
//!
//! This module provides functionality to parse both packed (single file) and
//! unpacked (directory structure) CircleCI orb definitions into typed Rust
//! structs.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use gen_orb_mcp::parser::OrbParser;
//!
//! // Parse an unpacked orb from directory
//! let orb = OrbParser::parse(Path::new("./src/@orb.yml")).unwrap();
//!
//! // Parse a packed orb from single file
//! let orb = OrbParser::parse(Path::new("./orb.yml")).unwrap();
//! ```

pub mod error;
pub mod types;

pub use error::ParseError;
pub use types::*;

use std::fs;
use std::path::Path;

/// Parser for CircleCI orb definitions.
///
/// Supports both packed (single YAML file) and unpacked (directory structure)
/// orb formats.
#[derive(Debug, Default)]
pub struct OrbParser;

impl OrbParser {
    /// Create a new orb parser.
    pub fn new() -> Self {
        Self
    }

    /// Auto-detect format and parse an orb definition.
    ///
    /// If the path is a directory or points to `@orb.yml`, parses as unpacked.
    /// Otherwise, parses as a packed single-file orb.
    pub fn parse(path: &Path) -> Result<OrbDefinition, ParseError> {
        if path.is_dir() {
            Self::parse_unpacked(path)
        } else if path.file_name().is_some_and(|f| f == "@orb.yml") {
            // Unpacked orb with @orb.yml entry point
            Self::parse_unpacked(path.parent().unwrap_or(path))
        } else {
            Self::parse_packed(path)
        }
    }

    /// Parse an unpacked orb from a directory structure.
    ///
    /// Expects the standard CircleCI orb directory layout:
    /// ```text
    /// orb_dir/
    /// ├── @orb.yml           # Root metadata
    /// ├── commands/          # Command definitions
    /// │   └── *.yml
    /// ├── jobs/              # Job definitions
    /// │   └── *.yml
    /// └── executors/         # Executor definitions
    ///     └── *.yml
    /// ```
    pub fn parse_unpacked(orb_dir: &Path) -> Result<OrbDefinition, ParseError> {
        let orb_yml_path = orb_dir.join("@orb.yml");

        // Read and parse @orb.yml for root metadata
        let orb_yml_content = fs::read_to_string(&orb_yml_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ParseError::MissingFile {
                    path: orb_yml_path.clone(),
                }
            } else {
                ParseError::FileRead {
                    path: orb_yml_path.clone(),
                    source: e,
                }
            }
        })?;

        let mut orb: OrbDefinition =
            serde_yaml::from_str(&orb_yml_content).map_err(|e| ParseError::YamlParse {
                path: orb_yml_path,
                source: e,
            })?;

        // Parse commands directory
        let commands_dir = orb_dir.join("commands");
        if commands_dir.is_dir() {
            orb.commands = Self::parse_directory(&commands_dir)?;
        }

        // Parse jobs directory
        let jobs_dir = orb_dir.join("jobs");
        if jobs_dir.is_dir() {
            orb.jobs = Self::parse_directory(&jobs_dir)?;
        }

        // Parse executors directory
        let executors_dir = orb_dir.join("executors");
        if executors_dir.is_dir() {
            orb.executors = Self::parse_directory(&executors_dir)?;
        }

        Ok(orb)
    }

    /// Parse a packed orb from a single YAML file.
    pub fn parse_packed(path: &Path) -> Result<OrbDefinition, ParseError> {
        let content = fs::read_to_string(path).map_err(|e| ParseError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;

        Self::parse_packed_content(&content, path)
    }

    /// Parse a packed orb from YAML content string.
    pub fn parse_packed_content(
        content: &str,
        source_path: &Path,
    ) -> Result<OrbDefinition, ParseError> {
        serde_yaml::from_str(content).map_err(|e| ParseError::YamlParse {
            path: source_path.to_path_buf(),
            source: e,
        })
    }

    /// Parse all YAML files in a directory into a HashMap.
    fn parse_directory<T>(dir: &Path) -> Result<std::collections::HashMap<String, T>, ParseError>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let mut items = std::collections::HashMap::new();

        let entries = fs::read_dir(dir).map_err(|e| ParseError::DirectoryRead {
            path: dir.to_path_buf(),
            source: e,
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| ParseError::DirectoryRead {
                path: dir.to_path_buf(),
                source: e,
            })?;

            let path = entry.path();

            // Skip non-YAML files and directories
            if path.is_dir() {
                continue;
            }

            let extension = path.extension().and_then(|e| e.to_str());
            if extension != Some("yml") && extension != Some("yaml") {
                continue;
            }

            // Get name from filename (without extension)
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| ParseError::InvalidStructure {
                    message: format!("invalid filename: {}", path.display()),
                })?
                .to_string();

            let content = fs::read_to_string(&path).map_err(|e| ParseError::FileRead {
                path: path.clone(),
                source: e,
            })?;

            let item: T = serde_yaml::from_str(&content).map_err(|e| ParseError::YamlParse {
                path: path.clone(),
                source: e,
            })?;

            items.insert(name, item);
        }

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_unpacked_orb(dir: &Path) {
        // Create @orb.yml
        fs::write(
            dir.join("@orb.yml"),
            r#"
version: "2.1"
description: "Test orb"
orbs:
  node: circleci/node@5
"#,
        )
        .unwrap();

        // Create commands directory
        let commands_dir = dir.join("commands");
        fs::create_dir_all(&commands_dir).unwrap();
        fs::write(
            commands_dir.join("greet.yml"),
            r#"
description: "Greet someone"
parameters:
  name:
    type: string
    default: "World"
    description: "Name to greet"
steps:
  - run: echo "Hello, << parameters.name >>!"
"#,
        )
        .unwrap();

        // Create jobs directory
        let jobs_dir = dir.join("jobs");
        fs::create_dir_all(&jobs_dir).unwrap();
        fs::write(
            jobs_dir.join("build.yml"),
            r#"
description: "Build the project"
executor: default
parameters:
  release:
    type: boolean
    default: false
steps:
  - checkout
  - run: cargo build
"#,
        )
        .unwrap();

        // Create executors directory
        let executors_dir = dir.join("executors");
        fs::create_dir_all(&executors_dir).unwrap();
        fs::write(
            executors_dir.join("default.yml"),
            r#"
description: "Default Rust executor"
docker:
  - image: rust:1.75
resource_class: medium
"#,
        )
        .unwrap();
    }

    #[test]
    fn test_parse_unpacked_orb() {
        let temp_dir = TempDir::new().unwrap();
        create_unpacked_orb(temp_dir.path());

        let orb = OrbParser::parse_unpacked(temp_dir.path()).unwrap();

        assert_eq!(orb.version, "2.1");
        assert_eq!(orb.description, Some("Test orb".to_string()));
        assert!(orb.orbs.contains_key("node"));

        // Check commands
        assert!(orb.commands.contains_key("greet"));
        let greet = &orb.commands["greet"];
        assert!(greet.parameters.contains_key("name"));
        assert_eq!(greet.steps.len(), 1);

        // Check jobs
        assert!(orb.jobs.contains_key("build"));
        let build = &orb.jobs["build"];
        assert!(build.parameters.contains_key("release"));

        // Check executors
        assert!(orb.executors.contains_key("default"));
        let default_exec = &orb.executors["default"];
        assert!(default_exec.config.docker.is_some());
    }

    #[test]
    fn test_parse_via_orb_yml_path() {
        let temp_dir = TempDir::new().unwrap();
        create_unpacked_orb(temp_dir.path());

        // Parse via @orb.yml path (should detect as unpacked)
        let orb = OrbParser::parse(&temp_dir.path().join("@orb.yml")).unwrap();
        assert_eq!(orb.version, "2.1");
        assert!(orb.commands.contains_key("greet"));
    }

    #[test]
    fn test_parse_packed_orb() {
        let packed_yaml = r#"
version: "2.1"
description: "Packed test orb"

commands:
  test:
    description: "Run tests"
    steps:
      - run: cargo test

jobs:
  ci:
    docker:
      - image: rust:1.75
    steps:
      - checkout
      - test

executors:
  rust:
    docker:
      - image: rust:1.75
"#;
        let temp_dir = TempDir::new().unwrap();
        let orb_file = temp_dir.path().join("orb.yml");
        fs::write(&orb_file, packed_yaml).unwrap();

        let orb = OrbParser::parse_packed(&orb_file).unwrap();

        assert_eq!(orb.version, "2.1");
        assert!(orb.commands.contains_key("test"));
        assert!(orb.jobs.contains_key("ci"));
        assert!(orb.executors.contains_key("rust"));
    }

    #[test]
    fn test_parse_auto_detect_packed() {
        let packed_yaml = r#"
version: "2.1"
commands:
  hello:
    steps:
      - run: echo hello
"#;
        let temp_dir = TempDir::new().unwrap();
        let orb_file = temp_dir.path().join("my-orb.yml");
        fs::write(&orb_file, packed_yaml).unwrap();

        // Should auto-detect as packed
        let orb = OrbParser::parse(&orb_file).unwrap();
        assert!(orb.commands.contains_key("hello"));
    }

    #[test]
    fn test_parse_missing_orb_yml() {
        let temp_dir = TempDir::new().unwrap();

        let result = OrbParser::parse_unpacked(temp_dir.path());
        assert!(matches!(result, Err(ParseError::MissingFile { .. })));
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let orb_file = temp_dir.path().join("bad.yml");
        fs::write(&orb_file, "{ invalid yaml [[[").unwrap();

        let result = OrbParser::parse_packed(&orb_file);
        assert!(matches!(result, Err(ParseError::YamlParse { .. })));
    }

    #[test]
    fn test_parse_empty_directories() {
        let temp_dir = TempDir::new().unwrap();

        // Create minimal @orb.yml
        fs::write(temp_dir.path().join("@orb.yml"), r#"version: "2.1""#).unwrap();

        // Create empty directories
        fs::create_dir_all(temp_dir.path().join("commands")).unwrap();
        fs::create_dir_all(temp_dir.path().join("jobs")).unwrap();
        fs::create_dir_all(temp_dir.path().join("executors")).unwrap();

        let orb = OrbParser::parse_unpacked(temp_dir.path()).unwrap();
        assert!(orb.commands.is_empty());
        assert!(orb.jobs.is_empty());
        assert!(orb.executors.is_empty());
    }

    #[test]
    fn test_parse_skips_non_yaml_files() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("@orb.yml"), r#"version: "2.1""#).unwrap();

        let commands_dir = temp_dir.path().join("commands");
        fs::create_dir_all(&commands_dir).unwrap();

        // Create a valid YAML file
        fs::write(commands_dir.join("valid.yml"), r#"steps: [checkout]"#).unwrap();

        // Create non-YAML files that should be skipped
        fs::write(commands_dir.join("readme.md"), "# Readme").unwrap();
        fs::write(commands_dir.join("script.sh"), "#!/bin/bash").unwrap();

        let orb = OrbParser::parse_unpacked(temp_dir.path()).unwrap();
        assert_eq!(orb.commands.len(), 1);
        assert!(orb.commands.contains_key("valid"));
    }
}
