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

impl Cli {
    /// Execute the CLI command
    pub fn run(&self) -> Result<()> {
        match &self.command {
            Commands::Generate {
                orb_path,
                output,
                format,
                name,
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

                // Create generator and generate code
                let generator = CodeGenerator::new().map_err(|e| anyhow::anyhow!("{}", e))?;
                let server = generator
                    .generate(&orb, &orb_name)
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
