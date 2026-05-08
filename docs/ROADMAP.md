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

## Tier 2: Build, Publish, Save (Planned)

### Objective

A consumer should be able to wire a complete, automated pipeline that:
1. Populates version history (`prime`)
2. Generates MCP server source with history embedded (`generate`)
3. Compiles the generated source to a native binary (`build`)
4. Uploads the binary to an existing GitHub release (`publish`)
5. Commits the generated artifacts back to the repo (`save`)

Steps 3–5 are the tier 2 additions. Full design: `docs/NEXT_PHASE_PLAN.md`.

### `build` subcommand

Compiles generated MCP server source to a native binary.

```
gen-orb-mcp build --input <DIR> [--output <DIR>] [--name <NAME>] [--target <TRIPLE>] [--dry-run]
```

- Verifies `<input>/Cargo.toml` exists, then runs `cargo build --release`
- Reports binary path on success
- Orb job: `build` — generated automatically by `gen-circleci-orb`

### `publish` subcommand

Uploads a compiled binary to an existing GitHub release as a release asset.

```
gen-orb-mcp publish --binary <PATH> --asset-name <NAME> [--tag <TAG>] [--dry-run]
```

- **Prerequisite**: the GitHub release must already exist; `publish` only uploads assets
- Resolves the release ID for the tag via the GitHub Releases API
- Uploads via `octocrate::repos::upload_release_asset` (typed GitHub API client)
- Environment variables: `GITHUB_TOKEN`, `CIRCLE_PROJECT_USERNAME`,
  `CIRCLE_PROJECT_REPONAME`, `CIRCLE_TAG`
- Orb job: `publish` — generated automatically by `gen-circleci-orb`

### `save` subcommand

Stages, commits, and pushes generated artifacts back to the repository.

```
gen-orb-mcp save [OPTIONS] <PATHS>...
```

- Idempotent: exits 0 without creating a commit if the working tree is clean after staging
- Default commit message includes `[skip ci]` to prevent CI re-triggering
- Implementation: `git2` + `git2_credentials` (same underlying crates as `pcu`)
- Orb job: `save` — generated automatically by `gen-circleci-orb`

### New Dependencies

```toml
octocrate = { version = "2.2.0", default-features = false, features = [
    "repos", "file-body", "rustls-tls",
] }
git2 = "0.20.4"
git2_credentials = "0.15.0"
```

### Complete Tier 2 Workflow

```yaml
orbs:
  gen-orb-mcp: jerus-org/gen-orb-mcp@<version>

workflows:
  release:
    jobs:
      - gen-orb-mcp/prime:
          orb_path: src/@orb.yml
          earliest_version: "1.0.0"
          ephemeral: true

      - gen-orb-mcp/generate:
          requires: [gen-orb-mcp/prime]
          orb_path: src/@orb.yml
          output: /tmp/mcp-build
          version: "${CIRCLE_TAG#v}"
          migrations: /tmp/gen-orb-mcp-prime/migrations
          prior_versions: /tmp/gen-orb-mcp-prime/prior-versions

      - gen-orb-mcp/build:
          requires: [gen-orb-mcp/generate]
          input: /tmp/mcp-build

      - gen-orb-mcp/publish:
          requires: [gen-orb-mcp/build]
          binary: /tmp/mcp-build/target/release/my_orb_mcp
          asset_name: my-orb-mcp-linux-x86_64
          context: github-release

      - gen-orb-mcp/save:
          requires: [gen-orb-mcp/generate]
          paths: prior-versions migrations orb/src
          context: github-push
```

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
