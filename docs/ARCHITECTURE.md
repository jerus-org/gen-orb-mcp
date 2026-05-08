# Architecture

## Overview

gen-orb-mcp transforms CircleCI orb definitions into MCP (Model Context Protocol) servers.
The tool also provides tooling for orb authors to compute migration rules, apply them to
consumer CI configurations, and prime a version history from git tags.

```
Orb YAML ──► OrbParser ──► OrbDefinition ──► CodeGenerator ──► GeneratedServer (Rust source)
```

A generated server exposes orb commands, jobs, and executors as MCP Resources. When
migration data is provided, it additionally exposes prior-version resources and
`plan_migration` / `apply_migration` MCP Tools.

---

## Workspace Structure

```
gen-orb-mcp/
├── Cargo.toml                     # Workspace manifest
├── crates/
│   └── gen-orb-mcp/
│       ├── src/
│       │   ├── main.rs            # Entry point: tracing setup, dispatch to Commands
│       │   ├── lib.rs             # Cli struct and Commands enum
│       │   ├── conformance_rule.rs
│       │   ├── consumer_parser/
│       │   ├── differ/
│       │   ├── generator/
│       │   ├── migrator/
│       │   ├── parser/
│       │   └── primer/
│       └── tests/
│           └── cmd/               # trycmd integration tests
└── docs/
```

---

## Module Reference

### `parser` — Orb YAML ingestion

| Type | Description |
|------|-------------|
| `OrbParser` | Parses an orb YAML file into an `OrbDefinition` |
| `OrbDefinition` | Top-level parsed representation: commands, jobs, executors, description |
| `Command` | Name, description, parameters, steps |
| `Job` | Name, description, executor, parameters, steps |
| `Executor` | Name, description, docker/machine/macos configuration |
| `Parameter` | Long name, type, default, description, required flag |
| `ParameterType` | `String`, `Boolean`, `Integer`, `Enum(Vec<String>)`, `Steps`, `Executor` |
| `Step` / `StructuredStep` | Orb step: run, checkout, or orb command reference |

Error type: `parser::ParseError` (wraps serde_yaml errors with file context).

### `generator` — MCP server code generation

| Type | Description |
|------|-------------|
| `CodeGenerator<'a>` | Builder: holds `OrbDefinition`, optional prior versions, optional conformance rules JSON |
| `GeneratedServer` | Output: a map of `PathBuf → String` (generated Rust source files) |

Builder methods:
- `CodeGenerator::new(orb, version, name)` — basic server (Resources only)
- `.with_prior_versions(snapshots)` — embeds prior-version Resources (`orb://v{ver}/...`)
- `.with_conformance_rules_json(rules_json)` — embeds rules, enables `plan_migration` and `apply_migration` Tools

Template engine: Handlebars (`handlebars` 6.x). Templates are embedded via `include_str!`
at compile time (`generator/templates.rs`). Context types for template rendering live in
`generator/context.rs`.

### `conformance_rule` — Rule types shared across diff and migration

| Variant | Meaning |
|---------|---------|
| `JobRemoved { name }` | Job was deleted with no replacement |
| `JobRenamed { old_name, new_name }` | Job was renamed |
| `JobAbsorbed { removed_job, absorbing_job }` | Separate job merged into another |
| `ParameterRemoved { job, param }` | Parameter removed from a job |
| `ParameterAdded { job, param, default }` | New required parameter added to a job |
| `ParameterEnumValueRemoved { job, param, value }` | Enum value removed from a parameter |
| `CommandRemoved { name }` | Command was deleted |

Rules are serialised as JSON. A `Vec<ConformanceRule>` from `diff` is consumed by `generate`
(`--migrations`) and `migrate`.

### `differ` — Semantic diff between two orb versions

| Symbol | Description |
|--------|-------------|
| `OrbDiffer<'a>` | Holds current and previous `OrbDefinition` |
| `diff(current, previous)` | Free function: returns `Vec<ConformanceRule>` |
| `diff_with_hints(current, previous, hints)` | Accepts rename hints to resolve ambiguous job identity |

The differ compares job names, parameter names, parameter types, and enum values. It emits
one rule per discrete change.

### `consumer_parser` — Consumer CI config analysis

| Type | Description |
|------|-------------|
| `ConsumerParser` | Parses `.circleci/*.yml` files in a consumer repository |
| `ConsumerConfig` | Parsed representation: workflows, jobs, orb references |
| `CiFile` | Single parsed CircleCI config file |
| `OrbRef` | An orb alias with a resolved version string |

Key operations:
- `requires_chain(job)` — returns the full upstream dependency chain for a job
- `find_absorbed_candidates(rule)` — identifies jobs that may match a `JobAbsorbed` rule
- `transitively_requires(job, target)` — tests whether `job` depends on `target`

### `migrator` — Apply conformance rules to consumer configs

| Type | Description |
|------|-------------|
| `Migrator` | Orchestrates plan → apply |
| `MigrationPlan` | List of `PlannedChange` values to be applied |
| `PlannedChange` | One change: `ChangeType` + location (file, line, context) |
| `AppliedChanges` | Report of what was written |

Sub-modules:
- `migrator::planner` — builds a `MigrationPlan` from rules + consumer config
- `migrator::applicator` — writes in-place YAML edits that preserve comments and formatting
- `migrator::reporter` — formats human-readable or JSON output
- `migrator::types` — shared types (`ChangeType`, `PlannedChange`, etc.)

### `primer` — Populate version history from git tags

| Type | Description |
|------|-------------|
| `PrimeConfig` | Input: repo path, orb path, output directories, tag prefix, earliest version |
| `PrimeResult` | Output: list of discovered versions with counts |
| `TagWithDate` | A `(semver::Version, chrono::DateTime)` pair from a git tag |
| `WorktreeGuard` | RAII guard: creates a temporary git worktree, cleans up on drop |

Key functions:
- `prime(config)` — top-level entry: orchestrates all steps
- `discover_tags(repo, prefix)` — walks git refs, extracts semver tags
- `checkout_and_parse(repo, tag, orb_path)` — creates a worktree, parses the orb at that tag
- `compute_diff(current, previous, version)` — runs the differ for a tag pair
- `serialize_orb(definition)` — serialises an `OrbDefinition` back to YAML for storage

---

## Data Flows

### Generation pipeline

```
orb YAML file
    │
    ▼
OrbParser::parse_file()
    │
    ▼  OrbDefinition
    │    commands: HashMap<String, Command>
    │    jobs:     HashMap<String, Job>
    │    executors: HashMap<String, Executor>
    │
    ▼
CodeGenerator::new(orb, version, name)
    ├── .with_prior_versions(snapshots)     [optional]
    └── .with_conformance_rules_json(json)  [optional]
    │
    ▼
CodeGenerator::generate()
    │
    ▼  GeneratedServer
         files: HashMap<PathBuf, String>
           ├── src/main.rs
           ├── src/lib.rs (MCP resource handlers)
           ├── Cargo.toml
           └── ...
```

### Migration pipeline

```
current orb YAML ──┐
previous orb YAML ──┤──► OrbDiffer::diff() ──► Vec<ConformanceRule> ──► JSON file
                   ┘

consumer .circleci/*.yml ──┐
Vec<ConformanceRule>       ├──► ConsumerParser + Migrator::plan() ──► MigrationPlan
orb alias                  ┘
                                       │
                                       ▼
                                Migrator::apply()
                                       │
                                       ▼
                              edited .circleci/*.yml
                              (comments and formatting preserved)
```

### Prime pipeline

```
git repository
    │
    ▼
discover_tags()  ──► Vec<TagWithDate>   (semver-tagged releases)
    │
    ▼ (for each adjacent tag pair)
WorktreeGuard::new()  ──► temporary worktree at tag
    │
    ▼
checkout_and_parse()  ──► OrbDefinition snapshot
    │
    ├──► serialize_orb()      ──► prior-versions/<version>.yml
    └──► compute_diff(prev)   ──► migrations/<version>.json
```

---

## Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `pmcp` | 2.6.0 | MCP protocol SDK (in generated servers) |
| `serde` / `serde_yaml` / `serde_json` | workspace | YAML/JSON parsing and serialisation |
| `clap` | 4.6.1 | CLI argument parsing with derive macros |
| `handlebars` | 6.4.0 | Template engine for code generation |
| `tokio` | 1.52.1 | Async runtime (used in generated servers) |
| `tracing` / `tracing-subscriber` | workspace | Structured logging |
| `anyhow` / `thiserror` | workspace | Error handling |
| `chrono` | 0.4.44 | Timestamp parsing on git tags (prime) |
| `semver` | 1.0.28 | Version ordering and comparison (prime, differ) |

---

## Output Format

The `generate` subcommand produces a self-contained Rust crate at `<output>/`:

```
<output>/
├── Cargo.toml          # Declares pmcp, serde, tokio as dependencies
├── src/
│   ├── main.rs         # MCP server entry point
│   └── lib.rs          # Resource and Tool handlers
└── (optional) .cargo/config.toml
```

The crate compiles to a standalone binary. Orb content (commands, jobs, executors,
prior-version snapshots, conformance rules) is embedded at compile time via `include_str!`
— the running binary has no external file dependencies.
