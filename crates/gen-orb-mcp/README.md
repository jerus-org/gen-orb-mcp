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

### 2. Automatically populate prior-version history

Use `prime` to discover version tags from git history, snapshot each version's orb, and compute
conformance rule diffs — all in one step. The sliding window keeps only the last 6 months (or
from a fixed anchor) to keep binary size bounded.

```bash
# Populate prior-versions/ and migrations/ from the last 6 months of tags
gen-orb-mcp prime \
  --orb-path ./circleci-toolkit/src/@orb.yml

# Anchor at a specific earliest version (covers your full estate)
gen-orb-mcp prime \
  --orb-path ./circleci-toolkit/src/@orb.yml \
  --earliest-version 4.1.0

# In CI: write to /tmp (ephemeral) and print paths for the next step
eval "$(gen-orb-mcp prime \
  --orb-path ./src/@orb.yml \
  --earliest-version 4.1.0 \
  --ephemeral)"
# → PRIME_PV_DIR and PRIME_MIG_DIR are now set; pass them to generate:
gen-orb-mcp generate \
  --orb-path ./src/@orb.yml \
  --output ./mcp-build \
  --version "${ORB_VERSION}" \
  --prior-versions "${PRIME_PV_DIR}" \
  --migrations "${PRIME_MIG_DIR}"
```

`prime` is idempotent: existing files are not overwritten; out-of-window files are removed.

### 3. Generate with migration Tools embedded

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

### 4. Migrate a consumer CI directory

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

### `prime` — Populate prior-versions/ and migrations/ from git history

```
gen-orb-mcp prime [OPTIONS]

Options:
  -p, --orb-path <PATH>            Path to the orb YAML entry point [default: src/@orb.yml]
      --git-repo <PATH>            Git repository root (default: walk up from orb-path to .git)
      --tag-prefix <PREFIX>        Git tag prefix [default: v]
      --earliest-version <VER>     Fixed anchor (e.g. "4.1.0"); conflicts with --since
      --since <DURATION>           Rolling window (e.g. "6 months") [default when neither set: 6 months]
      --prior-versions-dir <DIR>   Output dir for snapshots [default: prior-versions]
      --migrations-dir <DIR>       Output dir for rule JSON files [default: migrations]
      --ephemeral                  Write to /tmp/gen-orb-mcp-prime-<pid>/ and print
                                   PRIME_PV_DIR=... / PRIME_MIG_DIR=... to stdout
      --dry-run                    Describe actions without writing any files
```

Discovers all semver tags matching `<tag-prefix><version>` in the repository, filters to those
within the window, and for each version:
- Checks out the tag into a temporary git worktree (RAII cleanup, safe on panic)
- Saves the parsed orb to `prior-versions/<version>.yml`
- Computes conformance-rule diff vs the previous version, writes `migrations/<version>.json`
  (only if diff is non-empty)

Out-of-window snapshots and their matching migration files are removed. Idempotent: existing
files are not overwritten.

### `migrate` — Apply migration rules to a consumer CI directory

```
gen-orb-mcp migrate [OPTIONS] --orb <ALIAS> --rules <FILE>

Options:
      --ci-dir <DIR>    Path to consumer .circleci/ directory [default: .circleci]
      --orb <ALIAS>     Orb alias used in the consumer's orbs: section (e.g. toolkit)
      --rules <FILE>    Path to conformance rules JSON (produced by diff)
      --dry-run         Show planned changes without modifying files
```

## CircleCI Orb

A CircleCI orb for gen-orb-mcp is published to the CircleCI registry as
`jerus-org/gen-orb-mcp`. It provides one job and one command per subcommand, allowing you
to run gen-orb-mcp directly in your CircleCI pipeline without installing it manually.

To add the orb to your `.circleci/config.yml`:

```yaml
orbs:
  gen-orb-mcp: jerus-org/gen-orb-mcp@<version>
```

Available jobs: `generate`, `validate`, `diff`, `migrate`, `prime`.

The orb was itself generated by
[gen-circleci-orb](https://github.com/jerus-org/gen-circleci-orb), which introspects
the binary's `--help` output and produces orb source automatically.

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
