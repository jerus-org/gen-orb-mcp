# gen-orb-mcp

Generate MCP (Model Context Protocol) servers from CircleCI orb definitions.

[![Crates.io](https://img.shields.io/crates/v/gen-orb-mcp.svg)](https://crates.io/crates/gen-orb-mcp)
[![Documentation](https://docs.rs/gen-orb-mcp/badge.svg)](https://docs.rs/gen-orb-mcp)
[![License](https://img.shields.io/crates/l/gen-orb-mcp.svg)](https://github.com/jerus-org/gen-orb-mcp#license)

## Overview

**gen-orb-mcp** enables AI coding assistants to understand and work with private CircleCI orbs. It
parses an orb YAML definition and generates a standalone MCP server that exposes the orb's
commands, jobs, and executors as MCP Resources. When conformance rules are provided, the
generated server also gains `plan_migration` and `apply_migration` MCP Tools, allowing an AI
assistant to guide users through orb version migrations interactively.

## Features

- **Parse any CircleCI orb** — supports commands, jobs, executors, and parameters
- **Generate MCP servers** — produces Rust source code or a compiled native binary
- **Orb documentation as Resources** — commands, jobs, executors, and an overview exposed as MCP
  resources at `orb://commands/{name}`, `orb://jobs/{name}`, `orb://executors/{name}`
- **Multi-version embedding** — embed prior orb version snapshots alongside the current version
  so an AI assistant can answer cross-version questions (e.g. "what did job X look like in v4.7.1?")
- **Migration Tools** — when conformance rules are supplied, the generated server exposes
  `plan_migration` and `apply_migration` MCP Tools that call back into gen-orb-mcp at runtime
- **Offline operation** — generated servers run entirely offline with no external calls
- **Conformance-based migration CLI** — `diff` and `migrate` commands for scripted bulk migration

## Installation

```bash
cargo install gen-orb-mcp
```

Or with [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```bash
cargo binstall gen-orb-mcp
```

## Quick Start

### 1. Generate a basic MCP server

```bash
gen-orb-mcp generate \
  --orb-path ./circleci-toolkit/src/@orb.yml \
  --output ./circleci-toolkit-mcp \
  --version 4.9.6
```

This generates Rust source code in `./circleci-toolkit-mcp/`. To compile it:

```bash
cd circleci-toolkit-mcp && cargo build --release
```

### 2. Generate with migration Tools embedded

First compute conformance rules by diffing two orb versions:

```bash
gen-orb-mcp diff \
  --current ./circleci-toolkit/src/@orb.yml \
  --previous ./circleci-toolkit-4.7.1.yml \
  --since-version 4.9.6 \
  --output ./migrations/4.9.6.json
```

Then generate the server with rules and prior-version snapshots embedded:

```bash
gen-orb-mcp generate \
  --orb-path ./circleci-toolkit/src/@orb.yml \
  --output ./circleci-toolkit-mcp \
  --version 4.9.6 \
  --migrations ./migrations/ \
  --prior-versions ./prior-versions/
```

The `--prior-versions` directory should contain `<version>.yml` files (e.g. `4.7.1.yml`).

The generated server now exposes:
- All current-version Resources (`orb://commands/...`, `orb://jobs/...`)
- Prior-version Resources (`orb://v4.7.1/commands/...`, `orb://v4.7.1/jobs/...`)
- A version index at `orb://versions`
- `plan_migration` and `apply_migration` MCP Tools

### 3. Migrate a consumer CI directory

Apply migration rules directly from the CLI (no MCP server required):

```bash
# Dry run — show what would change
gen-orb-mcp migrate \
  --ci-dir ./.circleci \
  --orb toolkit \
  --rules ./migrations/4.9.6.json \
  --dry-run

# Apply changes
gen-orb-mcp migrate \
  --ci-dir ./.circleci \
  --orb toolkit \
  --rules ./migrations/4.9.6.json
```

## CLI Reference

### `generate` — Generate an MCP server

```
gen-orb-mcp generate [OPTIONS] --orb-path <PATH>

Options:
  -p, --orb-path <PATH>          Path to the orb YAML file (e.g. src/@orb.yml)
  -o, --output <DIR>             Output directory [default: ./dist]
  -f, --format <FORMAT>          Output format: source | binary [default: source]
  -n, --name <NAME>              Orb name (defaults to directory/filename)
  -V, --version <VERSION>        Version for the generated crate (e.g. 1.0.0)
      --force                    Overwrite existing output without confirmation
      --migrations <DIR>         Directory of conformance rule JSON files to embed
                                 (enables plan_migration / apply_migration Tools)
      --prior-versions <DIR>     Directory of prior orb YAML snapshots to embed
                                 (files named <version>.yml, e.g. 4.7.1.yml)
```

### `validate` — Validate an orb definition

```
gen-orb-mcp validate --orb-path <PATH>
```

### `diff` — Compute conformance rules between two orb versions

```
gen-orb-mcp diff --current <PATH> --previous <PATH> --since-version <VERSION> [--output <FILE>]
```

Emits a JSON array of `ConformanceRule` values describing what changed between versions. These
rules drive both the `migrate` CLI command and the MCP Tools in generated servers.

### `migrate` — Apply migration rules to a consumer CI directory

```
gen-orb-mcp migrate [OPTIONS] --orb <ALIAS> --rules <FILE>

Options:
      --ci-dir <DIR>    Path to consumer .circleci/ directory [default: .circleci]
      --orb <ALIAS>     Orb alias used in the consumer's orbs: section (e.g. toolkit)
      --rules <FILE>    Path to conformance rules JSON (produced by diff)
      --dry-run         Show planned changes without modifying files
```

## How Generated MCP Servers Work

### Resources

The generated server exposes the following MCP Resources:

| URI pattern | Content |
|---|---|
| `orb://overview` | Full markdown documentation of the orb |
| `orb://commands/{name}` | JSON definition of a command |
| `orb://jobs/{name}` | JSON definition of a job |
| `orb://executors/{name}` | JSON definition of an executor |
| `orb://versions` | List of all embedded versions (when prior versions are present) |
| `orb://v{version}/commands/{name}` | Command definition for a prior version |
| `orb://v{version}/jobs/{name}` | Job definition for a prior version |
| `orb://v{version}/executors/{name}` | Executor definition for a prior version |

### Tools (when `--migrations` is provided)

| Tool | Description |
|---|---|
| `plan_migration` | Analyse a consumer `.circleci/` directory and return a summary of changes needed |
| `apply_migration` | Apply the migration plan; pass `dry_run: true` to preview without writing |

Both tools accept `ci_dir` (path to the consumer's `.circleci/` directory) and `orb_alias`
(the alias used in the consumer's `orbs:` section). `apply_migration` also accepts `dry_run`
(boolean, default false).

### Using with Claude Code

Add the generated binary to your `claude_desktop_config.json` (or `.claude.json`):

```json
{
  "mcpServers": {
    "circleci-toolkit": {
      "command": "/path/to/circleci_toolkit_mcp"
    }
  }
}
```

Once connected, Claude Code can answer questions about your private orb and guide migrations
interactively.

## Migration Design

Migrations are **conformance-based**, not path-dependent. The `diff` command computes what the
target version's contract requires, regardless of which intermediate versions a consumer has
skipped. The `migrate` command (and the embedded Tools) then inspect the consumer's actual CI
state and fix all non-conformant patterns in one pass.

This means migrating from v4.7.0 directly to v5.0.0 is handled correctly even if v4.8.0 through
v4.11.0 were never used.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

[Contributing Guide](https://github.com/jerus-org/gen-orb-mcp/blob/main/CONTRIBUTING.md)
