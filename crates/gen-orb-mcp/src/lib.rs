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

pub mod parser;

use anyhow::Result;
use clap::{Parser, Subcommand};

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
        #[arg(short, long, value_enum, default_value = "binary")]
        format: OutputFormat,
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

                // TODO: Implement code generation
                println!(
                    "Parsed orb with {} commands, {} jobs, {} executors",
                    orb.commands.len(),
                    orb.jobs.len(),
                    orb.executors.len()
                );
                println!(
                    "Code generation not yet implemented. Output: {}, Format: {:?}",
                    output.display(),
                    format
                );
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
}
