# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**gen-orb-mcp** generates Model Context Protocol (MCP) servers from CircleCI orb definitions. This enables AI coding assistants to understand and work with private CircleCI orbs by exposing orb commands, jobs, and executors as MCP resources.

## Architecture

### Pipeline Overview

```
Orb YAML → Parser → Generator → MCP Server Source → (optional) Binary
```

### CLI Commands

- **generate**: Parse orb YAML and generate MCP server (binary or source output).
  Optional flags: `--migrations <dir>` embeds conformance rules and enables MCP Tools;
  `--prior-versions <dir>` embeds prior orb version snapshots as version-aware Resources.
- **validate**: Validate orb definition without generating
- **diff**: Compare two orb versions and emit a JSON array of `ConformanceRule` values
  describing what changed. Used to produce the rules files consumed by `--migrations` and `migrate`.
- **migrate**: Apply conformance rules to a consumer's `.circleci/` directory.
  Supports `--dry-run` to preview changes without writing files.

## Implementation Status

**Phase 1 MVP Complete** - Released as v0.1.0
**Phase 2 Migration Tooling Complete** - merged 2026-03-18

Implemented:
- CLI: `generate`, `validate`, `diff`, `migrate` subcommands
- `OrbParser` — full orb YAML parsing (commands, jobs, executors, parameters)
- `CodeGenerator` — MCP server source generation using Handlebars templates;
  supports `with_prior_versions()` and `with_conformance_rules_json()` builder methods
- `OrbDiffer` — semantic diff producing `Vec<ConformanceRule>` (JobRenamed, JobAbsorbed,
  ParameterRemoved, EnumValueRemoved, CommandRenamed)
- `ConsumerParser` — parses consumer `.circleci/*.yml` into a job-graph model;
  resolves orb aliases to versions; provides `requires_chain()` traversal
- `Migrator` — plan + apply conformance rules to consumer CI configs with in-place
  YAML editing that preserves comments and formatting
- Generated server — multi-version Resources (`orb://v{version}/...`) and MCP Tools
  (`plan_migration`, `apply_migration`) when `--migrations` is supplied
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
