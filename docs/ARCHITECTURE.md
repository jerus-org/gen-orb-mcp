# Architecture

## Overview

gen-orb-mcp is a CLI tool that transforms CircleCI orb definitions into MCP (Model Context
Protocol) servers, and provides the full supporting toolchain for orb authors to manage
version history, compute migration rules, and help consumers migrate to new orb versions.

The CLI is the primary artifact. It can be used directly in any CI system, scripted
locally, or invoked from shell steps in a pipeline. The CircleCI orb (`jerus-org/gen-orb-mcp`)
is a convenience layer that wraps each CLI subcommand as a job for CircleCI users — it does
not add new capabilities.

---

## CLI Tool

### Subcommands

| Subcommand | Purpose |
|------------|---------|
| `generate` | Parse an orb YAML and emit a complete MCP server as Rust source |
| `validate` | Validate an orb definition without generating |
| `diff` | Compute conformance rules between two orb versions → JSON |
| `migrate` | Apply conformance rules to a consumer's `.circleci/` directory |
| `prime` | Populate `prior-versions/` and `migrations/` from git tag history |

### How the Subcommands Relate

The subcommands are designed to be composed in a pipeline. In the most complete workflow:

```
git tags
    │
    ▼  prime
prior-versions/   migrations/
    │                  │
    └────────┬──────────┘
             │
             ▼  generate (--prior-versions, --migrations)
         MCP server source
             │
             ▼  cargo build --release
         MCP server binary
```

A minimal workflow uses only `generate` (no version history, no migration tools). The
full workflow adds `prime` first to build the history, and `diff` to produce migration
rules for individual version transitions.

`migrate` is an independent consumer-side tool: it applies conformance rules from `diff`
directly to a consumer's CI configuration without needing the MCP server.

`validate` is a standalone check that can be wired into any CI pipeline to catch orb
YAML errors early.

### Usage Without CircleCI

Users on GitHub Actions, GitLab CI, Jenkins, or any other CI platform call the CLI binary
directly. Install via `cargo install gen-orb-mcp` or `cargo binstall gen-orb-mcp`, then
invoke the subcommands as shell steps:

```bash
# Populate version history
gen-orb-mcp prime \
  --orb-path src/@orb.yml \
  --output-dir /tmp/prime-output \
  --earliest-version 1.0.0

# Generate MCP server source
gen-orb-mcp generate \
  --orb-path src/@orb.yml \
  --output ./mcp-server \
  --version "${RELEASE_VERSION}" \
  --migrations /tmp/prime-output/migrations \
  --prior-versions /tmp/prime-output/prior-versions

# Compile
cd mcp-server && cargo build --release
```

---

## Internal Architecture

### Source Structure

```
crates/gen-orb-mcp/src/
├── main.rs                # Entry point: tracing setup, dispatch to Commands
├── lib.rs                 # Cli struct and Commands enum
├── conformance_rule.rs    # ConformanceRule enum — shared across diff, generate, migrate
├── parser/                # OrbParser: YAML → OrbDefinition
├── generator/             # CodeGenerator: OrbDefinition → Rust source
├── differ/                # OrbDiffer: two OrbDefinitions → Vec<ConformanceRule>
├── consumer_parser/       # ConsumerParser: consumer .circleci/*.yml → job graph
├── migrator/              # Migrator: conformance rules + consumer config → edits
└── primer/                # prime(): git tags → version snapshots + migration files
```

### Module Reference

#### `parser` — Orb YAML ingestion

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

#### `generator` — MCP server code generation

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

#### `conformance_rule` — Rule types shared across diff and migration

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

#### `differ` — Semantic diff between two orb versions

| Symbol | Description |
|--------|-------------|
| `OrbDiffer<'a>` | Holds current and previous `OrbDefinition` |
| `diff(current, previous)` | Free function: returns `Vec<ConformanceRule>` |
| `diff_with_hints(current, previous, hints)` | Accepts rename hints to resolve ambiguous job identity |

The differ compares job names, parameter names, parameter types, and enum values. It emits
one rule per discrete change.

#### `consumer_parser` — Consumer CI config analysis

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

#### `migrator` — Apply conformance rules to consumer configs

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

#### `primer` — Populate version history from git tags

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

### Data Flows

#### Generation pipeline

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

#### Migration pipeline

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

#### Prime pipeline

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

### Key Dependencies

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

### Generated MCP Server Output

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
— the running binary has no external file dependencies at runtime.

---

## CircleCI Orb

The orb (`jerus-org/gen-orb-mcp`) is a convenience layer for CircleCI users. It provides
a pre-built Docker executor with gen-orb-mcp installed, and exposes each CLI subcommand as
a CircleCI job so consumers do not need to write shell steps manually.

The orb is generated automatically from the CLI binary by
[gen-circleci-orb](https://github.com/jerus-org/gen-circleci-orb), which introspects
`gen-orb-mcp --help` and produces one command file and one job file per subcommand. The
orb's job structure is directly determined by the CLI's subcommand structure — it adds no
logic of its own.

### Orb Structure

```
orb/src/
├── @orb.yml              # Metadata (description, display URLs)
├── executors/
│   └── default.yml       # Docker image jerusdp/gen-orb-mcp:<< parameters.tag >>
├── commands/             # One file per CLI subcommand (generate, validate, diff, migrate, prime)
├── jobs/                 # One file per CLI subcommand — wraps checkout + command
├── scripts/              # Supporting shell scripts
└── examples/             # Usage examples
```

Each job follows the same pattern:
1. `checkout` — check out the repository
2. Invoke the corresponding orb command, forwarding all parameters

### Executor

The orb executor uses the `jerusdp/gen-orb-mcp` Docker image, which has gen-orb-mcp
pre-installed. The `tag` parameter (default `latest`) allows consumers to pin to a
specific version of the tool.

### Relationship to the CLI

A CircleCI consumer using the orb is invoking exactly the same CLI subcommands that any
other CI platform would use in shell steps. The orb provides:
- A pre-built environment (no install step needed)
- Declarative job invocation (parameters map directly to CLI flags)
- Integration with CircleCI workspace and context mechanisms

Non-CircleCI consumers replicate the same behaviour by running the CLI binary directly
in their own CI environment, using the equivalent shell flags documented in
`docs/QUICKSTART.md` and `docs/CI_INTEGRATION_GUIDE.md`.
