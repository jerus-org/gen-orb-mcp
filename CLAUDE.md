# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**gen-orb-mcp** generates Model Context Protocol (MCP) servers from CircleCI orb definitions. This enables AI coding assistants to understand and work with private CircleCI orbs by exposing orb commands, jobs, and executors as MCP resources.

## Development Commands

```bash
cargo build                    # Build the project
cargo test                     # Run all tests
cargo clippy                   # Lint
cargo fmt                      # Format code
cargo doc --no-deps            # Generate documentation

# Run the CLI
cargo run -- generate --orb-path <path> --output ./dist
cargo run -- validate --orb-path <path>

# Run a single test
cargo test test_name
cargo test -p gen-orb-mcp test_name
```

## Architecture

### Pipeline Overview

```
Orb YAML → Parser → Generator → MCP Server Source → (optional) Binary
```

### Workspace Structure

```
gen-orb-mcp/
├── Cargo.toml                 # Workspace manifest
├── crates/
│   └── gen-orb-mcp/           # Main crate
│       ├── src/
│       │   ├── main.rs        # CLI entry point with tracing setup
│       │   ├── lib.rs         # CLI definition (Cli struct, Commands enum)
│       │   ├── parser/        # Orb YAML parsing
│       │   │   ├── mod.rs     # OrbParser implementation
│       │   │   ├── types.rs   # OrbDefinition, Command, Job, Executor types
│       │   │   └── error.rs   # Parser error types
│       │   └── generator/     # MCP server code generation
│       │       ├── mod.rs     # CodeGenerator implementation
│       │       ├── templates.rs # Handlebars templates (embedded)
│       │       ├── context.rs # Template context types
│       │       └── error.rs   # Generator error types
│       └── tests/
├── docs/                      # Design documentation
│   ├── ARCHITECTURE.md        # Technical design and data structures
│   ├── IMPLEMENTATION_PLAN.md # Task breakdown and timeline
│   ├── FINALIZED_PLAN.md      # Project decisions and scope
│   └── QUICKSTART.md          # Development setup guide
└── AI_DILIGENCE.md            # AI assistance transparency statement
```

### CLI Commands

- **generate**: Parse orb YAML and generate MCP server (binary or source output)
- **validate**: Validate orb definition without generating

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `rmcp` | MCP protocol SDK (used in generated servers) |
| `serde_yaml` | CircleCI orb YAML parsing |
| `clap` | CLI argument parsing with derive macros |
| `handlebars` | Template engine for code generation |
| `tracing` | Structured logging |
| `anyhow`/`thiserror` | Error handling |

### Module Structure

```rust
// Parser layer - parse orb YAML into typed structs
pub mod parser {
    pub struct OrbParser;           // Parses orb YAML files
    pub struct OrbDefinition {      // Parsed orb representation
        commands: HashMap<String, Command>,
        jobs: HashMap<String, Job>,
        executors: HashMap<String, Executor>,
    }
}

// Generator layer - produce MCP server code from OrbDefinition
pub mod generator {
    pub struct CodeGenerator;       // Generates Rust source code
    pub struct GeneratedServer;     // Output containing generated files
}
```

## Implementation Status

**Phase 1 MVP Complete** - Released as v0.1.0

Implemented:
- CLI structure with `generate` and `validate` subcommands
- `OrbParser` - Full orb YAML parsing (commands, jobs, executors, parameters)
- `CodeGenerator` - MCP server source generation using Handlebars templates
- Binary compilation via `cargo build` in generated output directory

## Output Formats

- **source** (default): Generates Rust source code for the MCP server
- **binary**: Generates source then compiles to native Linux x86_64 binary

## Privacy Requirements

This tool handles private orbs - generated servers must support:
- Private Docker registries
- No telemetry or external data transmission
- Fully offline operation at runtime

## CI/CD Guidance

### Release Workflow Patterns

This project uses a two-stage release workflow in `.circleci/release.yml`:

1. **calculate-versions**: Computes versions using `nextsv` and persists to workspace
2. **release-crate**: Reads version from workspace, publishes crate
3. **release-prlog**: Reads version from workspace, updates PRLOG.md and creates workspace tag

**Key learnings:**

- **Workspace persistence** is the correct way to pass calculated values between CircleCI jobs (not job parameters)
- **Sequential jobs that push to main** must pull latest before pushing (race condition)
- **Version overrides** should default to empty string `""` for auto-detection

### nextsv Version Calculation

nextsv uses git tags with prefixes to determine the next version:

| Scope | Tag Prefix | Example |
|-------|------------|---------|
| Crate | `<crate-name>-v` | `gen-orb-mcp-v0.1.0` |
| Workspace | `v` | `v0.1.0` |

**Important:** Crate and workspace tags should be aligned at the same commit when starting a new version series to ensure correct calculations.

### PRLOG.md Maintenance

PRLOG.md contains links to git tags. When tags are renamed or moved:
- Update the `[Unreleased]` compare link
- Update version section headers and links

## GitHub Interaction

**Prefer `gh` CLI over direct API calls.** GitHub APIs may be deprecated or changed. The `gh` CLI provides a stable interface:

```bash
# Preferred
gh pr create --title "..." --body "..."
gh pr view 123
gh release create v1.0.0

# Avoid direct API calls when gh CLI can accomplish the task
```
