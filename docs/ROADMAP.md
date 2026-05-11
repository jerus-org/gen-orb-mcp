# Roadmap

## Current Capabilities (v0.1.x)

### CLI Subcommands

| Subcommand | Purpose |
|------------|---------|
| `generate` | Parse an orb YAML and emit a complete MCP server as Rust source |
| `validate` | Validate an orb definition without generating |
| `diff` | Compute conformance rules between two orb versions → JSON |
| `migrate` | Apply conformance rules to a consumer's `.circleci/` directory |
| `prime` | Populate `prior-versions/` and `migrations/` from git tag history |
| `build` | Compile generated MCP server source to a native binary |
| `publish` | Upload a compiled binary to an existing GitHub release |
| `save` | Stage, commit, and push generated artifacts back to the repository |

### CircleCI Orb (`jerus-org/gen-orb-mcp`)

The orb is generated from the binary by `gen-circleci-orb` and exposes each subcommand as
a job. Consumers wire these jobs into their release workflow to automate the full pipeline.

### Generated MCP Server Features

| Feature | Available |
|---------|-----------|
| Current-version Resources (`orb://commands/...`, `orb://jobs/...`) | ✅ |
| Prior-version Resources (`orb://v{ver}/commands/...`) | ✅ (with `--prior-versions`) |
| `plan_migration` Tool | ✅ (with `--migrations`) |
| `apply_migration` Tool | ✅ (with `--migrations`) |
| Compile to native binary | ✅ |

---

## Tier 3: Composed Jobs (Deferred)

The current architecture maps one CLI subcommand to one orb job. Composed jobs — jobs that
combine multiple subcommands, add custom workflow steps, or suppress the default 1:1
mapping — require a configuration module in `gen-circleci-orb` that is not yet implemented.

This is tracked as jerus-org/gen-circleci-orb#42.

Examples of tier 3 composed jobs:
- A single `sync` job that runs `prime` + `generate` + `save` in sequence
- A `build-server` job that runs `generate` + `build` without workspace hand-off
- Suppressing individual subcommand jobs to expose only composed jobs

---

## Out of Scope

- Cross-compilation (macOS, Windows targets) — Linux x86_64 only for the foreseeable future
- Additional `publish` targets (S3, GitLab releases) — `github-release` only
- Dedicated git library crate extracted from `pcu` — future direction, requires a new crate
  in the `pcu` workspace to expose a stable external API

---

## Completed Milestones

### Tier 2 — Build, Publish, Save (2026-05)

Completed the automated end-to-end pipeline from orb YAML to a compiled binary attached to a
GitHub release:

- `build` subcommand: compiles generated MCP server source via `cargo build --release`;
  reads crate name from `Cargo.toml` when `--name` is omitted; supports `--target` for
  cross-compilation
- `publish` subcommand: uploads a compiled binary to an existing GitHub release using
  `octocrate`; resolves release ID by tag; reads credentials from `GITHUB_TOKEN`,
  `CIRCLE_PROJECT_USERNAME`, `CIRCLE_PROJECT_REPONAME`, `CIRCLE_TAG`
- `save` subcommand: stages, commits, and pushes specified paths via `git2` +
  `git2_credentials`; idempotent (exits 0 if working tree is clean after staging);
  default commit message includes `[skip ci]`
- All three subcommands support `--dry-run`
- Orb updated: `build`, `publish`, `save` jobs generated automatically by `gen-circleci-orb`
- Full 5-job pipeline achievable with the orb alone: `prime → generate → build → publish`
  (with `save` running in parallel from `generate`)

### v0.1.0 — Phase 1 MVP (2024)

- `OrbParser`: full orb YAML parsing (commands, jobs, executors, parameters)
- `CodeGenerator`: MCP server source generation using Handlebars templates
- `generate` and `validate` CLI subcommands
- Generated server exposes current-version Resources

### Phase 2 — Migration Tooling (merged 2026-03-18)

- `ConformanceRule` enum (JobRenamed, JobAbsorbed, ParameterRemoved, ParameterAdded,
  EnumValueRemoved, CommandRemoved, JobRemoved)
- `OrbDiffer`: semantic diff producing `Vec<ConformanceRule>`
- `ConsumerParser`: parses consumer `.circleci/*.yml` into a job-graph model; resolves orb
  aliases to versions; provides `requires_chain()` traversal
- `Migrator`: plan + apply conformance rules to consumer CI configs with in-place YAML
  editing that preserves comments and formatting
- `diff` and `migrate` CLI subcommands
- Generated server: prior-version Resources + `plan_migration` / `apply_migration` Tools
  when `--migrations` is supplied
- Binary compilation via `cargo build` in generated output directory

### `prime` Command (2026-03)

- `PrimeConfig`, `PrimeResult`, `WorktreeGuard`
- `prime()`: discovers version tags, creates temporary worktrees, serialises orb snapshots,
  computes adjacent-version diffs
- `prime` CLI subcommand
- Eliminates the need to rebuild version history from scratch on every CI run

### CircleCI Orb via `gen-circleci-orb` (2026-04)

- `gen-circleci-orb` introspects `gen-orb-mcp --help` and generates the orb automatically
- Orb published as `jerus-org/gen-orb-mcp`: one job per subcommand (generate, validate,
  diff, migrate, prime)
- Consumers wire orb jobs rather than shell scripts
- `gen-circleci-orb` dogfoods itself for orb regeneration in gen-orb-mcp's own CI

### LLVM OOM Fix (merged 2026-03-24, PR #77)

- Replaced `include_bytes!` with `include_str!` for large embedded orb content
- Eliminated LLVM out-of-memory errors during compilation of generated servers backed by
  large orbs

### cargo-binstall Support (issue #32, closed 2026-03)

- Binary release assets published on GitHub releases
- `cargo binstall gen-orb-mcp` installs the pre-compiled binary
- Package metadata in `Cargo.toml` (`[package.metadata.binstall]`)
