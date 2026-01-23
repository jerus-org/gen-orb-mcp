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
Orb YAML → Parser → Analyzer → Generator → Builder → MCP Server Binary
```

### Workspace Structure

```
gen-orb-mcp/
├── Cargo.toml                 # Workspace manifest
├── crates/
│   └── gen-orb-mcp/           # Main crate
│       ├── src/
│       │   ├── main.rs        # CLI entry point with tracing setup
│       │   └── lib.rs         # CLI definition (Cli struct, Commands enum)
│       └── tests/
└── docs/                      # Design documentation
    ├── ARCHITECTURE.md        # Technical design and data structures
    ├── IMPLEMENTATION_PLAN.md # Task breakdown and timeline
    ├── FINALIZED_PLAN.md      # Project decisions and scope
    └── QUICKSTART.md          # Development setup guide
```

### CLI Commands

- **generate**: Parse orb YAML and generate MCP server (binary or source output)
- **validate**: Validate orb definition without generating

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `pmcp` | MCP protocol SDK |
| `serde_yaml` | CircleCI orb YAML parsing |
| `clap` | CLI argument parsing with derive macros |
| `handlebars` | Template engine for code generation |
| `tracing` | Structured logging |
| `anyhow`/`thiserror` | Error handling |

### Planned Module Structure

```rust
// Parser layer - parse orb YAML into typed structs
pub struct OrbParser;
pub struct OrbDefinition {
    commands: HashMap<String, Command>,
    jobs: HashMap<String, Job>,
    executors: HashMap<String, Executor>,
}

// Generator layer - produce MCP server code from OrbDefinition
pub struct CodeGenerator {
    templates: TemplateRegistry,  // Handlebars templates
}

// Builder layer - compile generated code to binary
pub struct RustCompiler;
```

## Implementation Status

Currently in Phase 1 MVP. Implemented:
- CLI structure with `generate` and `validate` subcommands
- Argument parsing

Not yet implemented:
- Orb YAML parsing (`OrbParser`)
- MCP server code generation (`CodeGenerator`)
- Binary compilation (`RustCompiler`)

## Output Formats

- **binary**: Compiles generated MCP server to native Linux x86_64 binary
- **source**: Generates Rust source code for the MCP server

## Privacy Requirements

This tool handles private orbs - generated servers must support:
- Private Docker registries
- No telemetry or external data transmission
- Fully offline operation at runtime
