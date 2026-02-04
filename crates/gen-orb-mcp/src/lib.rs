//! # gen-orb-mcp
//!
//! Generate MCP (Model Context Protocol) servers from CircleCI orb definitions.
//!
//! This tool enables AI coding assistants to understand and work with private
//! CircleCI orbs by generating MCP servers that expose orb commands, jobs,
//! and executors as resources.
//!
//! ## Usage
//!
//! ```bash
//! gen-orb-mcp generate --orb-path ./src/@orb.yml --output ./dist/
//! ```

pub mod generator;
pub mod parser;

use anyhow::Result;
use clap::{Parser, Subcommand};

use generator::CodeGenerator;
use parser::OrbParser;

/// Generate MCP servers from CircleCI orb definitions
#[derive(Debug, Parser)]
#[command(name = "gen-orb-mcp")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Generate an MCP server from an orb definition
    Generate {
        /// Path to the orb YAML file (e.g., src/@orb.yml)
        #[arg(short = 'p', long)]
        orb_path: std::path::PathBuf,

        /// Output directory for generated server
        #[arg(short = 'o', long, default_value = "./dist")]
        output: std::path::PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "source")]
        format: OutputFormat,

        /// Name for the generated orb server (defaults to filename)
        #[arg(short, long)]
        name: Option<String>,

        /// Version for the generated MCP server crate (e.g., "1.0.0")
        ///
        /// Required when regenerating an existing output directory.
        /// For CI workflows, this should match the orb release version.
        #[arg(short = 'V', long)]
        version: Option<String>,

        /// Overwrite existing files without confirmation
        ///
        /// Required for non-interactive CI environments when output exists.
        #[arg(long)]
        force: bool,
    },
    /// Validate an orb definition without generating
    Validate {
        /// Path to the orb YAML file
        #[arg(short = 'p', long)]
        orb_path: std::path::PathBuf,
    },
}

/// Output format for generated MCP server
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Compile to native binary (Linux x86_64)
    Binary,
    /// Generate Rust source code
    Source,
}

/// Default version for fresh generation when no version is specified.
const DEFAULT_VERSION: &str = "0.1.0";

impl Cli {
    /// Execute the CLI command
    pub fn run(&self) -> Result<()> {
        match &self.command {
            Commands::Generate {
                orb_path,
                output,
                format,
                name,
                version,
                force,
            } => {
                tracing::info!(?orb_path, ?output, ?format, "Generating MCP server");

                // Parse the orb definition
                let orb = OrbParser::parse(orb_path).map_err(|e| anyhow::anyhow!("{}", e))?;
                tracing::info!(
                    commands = orb.commands.len(),
                    jobs = orb.jobs.len(),
                    executors = orb.executors.len(),
                    "Parsed orb definition"
                );

                // Derive orb name from path if not specified
                let orb_name = name.clone().unwrap_or_else(|| derive_orb_name(orb_path));

                // Resolve version based on output state
                let resolved_version = resolve_version(output, version.as_deref(), *force)?;
                tracing::info!(version = %resolved_version, "Using version");

                // Create generator and generate code
                let generator = CodeGenerator::new().map_err(|e| anyhow::anyhow!("{}", e))?;
                let server = generator
                    .generate(&orb, &orb_name, &resolved_version)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                // Write output
                match format {
                    OutputFormat::Source => {
                        server
                            .write_to(output)
                            .map_err(|e| anyhow::anyhow!("{}", e))?;
                        println!("Generated MCP server source code:");
                        println!("  Output: {}", output.display());
                        println!("  Crate: {}", server.crate_name);
                        println!("  Version: {}", resolved_version);
                        println!("  Commands: {}", orb.commands.len());
                        println!("  Jobs: {}", orb.jobs.len());
                        println!("  Executors: {}", orb.executors.len());
                        println!();
                        println!("To build: cd {} && cargo build --release", output.display());
                    }
                    OutputFormat::Binary => {
                        // Write source first
                        server
                            .write_to(output)
                            .map_err(|e| anyhow::anyhow!("{}", e))?;

                        // Attempt to compile
                        println!("Compiling MCP server...");
                        let status = std::process::Command::new("cargo")
                            .args(["build", "--release"])
                            .current_dir(output)
                            .status();

                        match status {
                            Ok(s) if s.success() => {
                                let binary_path =
                                    output.join("target/release").join(&server.crate_name);
                                println!("Successfully compiled MCP server:");
                                println!("  Binary: {}", binary_path.display());
                                println!("  Version: {}", resolved_version);
                            }
                            Ok(_) => {
                                anyhow::bail!(
                                    "Compilation failed. Source code is available at: {}",
                                    output.display()
                                );
                            }
                            Err(e) => {
                                anyhow::bail!(
                                    "Failed to run cargo: {}. Source code is available at: {}",
                                    e,
                                    output.display()
                                );
                            }
                        }
                    }
                }

                Ok(())
            }
            Commands::Validate { orb_path } => {
                tracing::info!(?orb_path, "Validating orb definition");

                // Parse and validate the orb definition
                let orb = OrbParser::parse(orb_path).map_err(|e| anyhow::anyhow!("{}", e))?;

                println!("Orb validation successful!");
                println!("  Version: {}", orb.version);
                if let Some(desc) = &orb.description {
                    println!("  Description: {}", desc);
                }
                println!("  Commands: {}", orb.commands.len());
                for name in orb.commands.keys() {
                    println!("    - {}", name);
                }
                println!("  Jobs: {}", orb.jobs.len());
                for name in orb.jobs.keys() {
                    println!("    - {}", name);
                }
                println!("  Executors: {}", orb.executors.len());
                for name in orb.executors.keys() {
                    println!("    - {}", name);
                }
                Ok(())
            }
        }
    }
}

/// Derive orb name from the orb path.
///
/// Uses the parent directory name if the file is `@orb.yml`, otherwise
/// uses the file stem (filename without extension).
fn derive_orb_name(path: &std::path::Path) -> String {
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("orb");

    if filename == "@orb.yml" {
        // Use parent directory name
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("orb")
            .to_string()
    } else {
        // Use filename without extension
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("orb")
            .to_string()
    }
}

/// Resolve the version to use for the generated MCP server.
///
/// # Version Resolution Rules
///
/// 1. If `--version` is provided, use it
/// 2. If output directory exists with Cargo.toml and no `--version`:
///    - Error: must specify version to regenerate
/// 3. If fresh generation and no `--version`: use DEFAULT_VERSION
///
/// The `--force` flag is required when overwriting existing output.
fn resolve_version(output: &std::path::Path, version: Option<&str>, force: bool) -> Result<String> {
    let cargo_toml = output.join("Cargo.toml");
    let output_exists = cargo_toml.exists();

    match (version, output_exists) {
        // Explicit version provided - use it
        (Some(v), false) => {
            tracing::debug!("Using provided version for fresh generation");
            Ok(v.to_string())
        }
        (Some(v), true) => {
            if !force {
                anyhow::bail!(
                    "Output directory '{}' already exists. Use --force to overwrite.",
                    output.display()
                );
            }
            tracing::debug!("Using provided version, overwriting existing output");
            Ok(v.to_string())
        }

        // No version provided
        (None, false) => {
            tracing::debug!("Fresh generation with default version");
            Ok(DEFAULT_VERSION.to_string())
        }
        (None, true) => {
            anyhow::bail!(
                "Output directory '{}' already exists.\n\
                 To regenerate, you must specify the version explicitly:\n\n\
                 \x20   gen-orb-mcp generate --orb-path <PATH> --output {} --version <VERSION> --force\n\n\
                 For CI release workflows, use the orb release version (e.g., --version 1.6.0).",
                output.display(),
                output.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cli_parse_generate() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "generate",
            "--orb-path",
            "test.yml",
            "--output",
            "./out",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_generate_with_version() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "generate",
            "--orb-path",
            "test.yml",
            "--output",
            "./out",
            "--version",
            "1.2.3",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_generate_with_force() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "generate",
            "--orb-path",
            "test.yml",
            "--output",
            "./out",
            "--version",
            "1.2.3",
            "--force",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_validate() {
        let cli = Cli::try_parse_from(["gen-orb-mcp", "validate", "--orb-path", "test.yml"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_derive_orb_name_from_orb_yml() {
        use std::path::Path;
        let path = Path::new("/path/to/my-toolkit/src/@orb.yml");
        assert_eq!(derive_orb_name(path), "src");

        let path = Path::new("my-orb/@orb.yml");
        assert_eq!(derive_orb_name(path), "my-orb");
    }

    #[test]
    fn test_derive_orb_name_from_packed() {
        use std::path::Path;
        let path = Path::new("/path/to/my-toolkit.yml");
        assert_eq!(derive_orb_name(path), "my-toolkit");

        let path = Path::new("orb.yml");
        assert_eq!(derive_orb_name(path), "orb");
    }

    #[test]
    fn test_resolve_version_fresh_with_explicit() {
        let temp_dir = TempDir::new().unwrap();
        let result = resolve_version(temp_dir.path(), Some("2.0.0"), false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2.0.0");
    }

    #[test]
    fn test_resolve_version_fresh_with_default() {
        let temp_dir = TempDir::new().unwrap();
        let result = resolve_version(temp_dir.path(), None, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), DEFAULT_VERSION);
    }

    #[test]
    fn test_resolve_version_existing_without_version_fails() {
        let temp_dir = TempDir::new().unwrap();
        // Create a Cargo.toml to simulate existing output
        std::fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"",
        )
        .unwrap();

        let result = resolve_version(temp_dir.path(), None, false);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already exists"));
        assert!(err.contains("--version"));
    }

    #[test]
    fn test_resolve_version_existing_with_version_no_force_fails() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"",
        )
        .unwrap();

        let result = resolve_version(temp_dir.path(), Some("1.5.0"), false);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("--force"));
    }

    #[test]
    fn test_resolve_version_existing_with_version_and_force_succeeds() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"",
        )
        .unwrap();

        let result = resolve_version(temp_dir.path(), Some("1.5.0"), true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1.5.0");
    }
}
