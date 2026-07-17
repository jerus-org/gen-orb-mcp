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

gen-orb-mcp also ships as a **public CircleCI orb** (`jerus-org/gen-orb-mcp`) that **any orb
author can add to their own release pipeline** — not just jerus-org projects. Adopting the orb
gives your orb an automatically generated, version-aware MCP server (and AI-guided migrations)
with no local tooling to install and no jerus-org-specific setup: the env-var names it reads are
configurable, so you map them to your own CI secrets. See
[Adopt the orb in your own pipeline](#adopt-the-orb-in-your-own-pipeline).

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

## CircleCI Orb

gen-orb-mcp is published as a **public** CircleCI orb at `jerus-org/gen-orb-mcp` — usable by any
CircleCI project, not only jerus-org's. It runs inside a pre-built Docker image with gen-orb-mcp
pre-installed, so there is nothing to install at build time.

### Add to your config

```yaml
orbs:
  gen-orb-mcp: jerus-org/gen-orb-mcp@0.2.0
```

Pin a concrete version and let [Renovate](https://docs.renovatebot.com/) (CircleCI orb
datasource) keep it current; check the [registry](https://circleci.com/developer/orbs/orb/jerus-org/gen-orb-mcp)
for the latest release.

### Available jobs

The orb offers the individual subcommands as jobs, plus one composed job that runs the whole
MCP-release pipeline as a single step:

| Job | Description |
|-----|-------------|
| `build_mcp_server` | **Composite** — prime prior versions, generate and compile the MCP server, publish the binary to a GitHub release, and commit the artifacts back, in one job |
| `generate` | Generate an MCP server (source or binary) from an orb YAML file |
| `validate` | Validate an orb definition |
| `diff` | Compute conformance rules between two orb versions |
| `migrate` | Apply migration rules to a consumer CI directory |

The standalone jobs map one-to-one to the CLI subcommands (`gen-orb-mcp <subcommand> --help`).
`build_mcp_server` does **not** — it has no matching subcommand. It is a *composed* job assembled
from several of gen-orb-mcp's commands (`prime`, `generate`, `publish`, `save`) plus checkout,
workspace, and git-setup steps, authored in gen-circleci-orb's configuration rather than derived
from `--help`. That is what lets a consumer treat "build and publish my MCP server" as one
activity instead of wiring five jobs. See gen-circleci-orb's
[Advanced Configuration Guide](https://github.com/jerus-org/gen-circleci-orb/blob/main/docs/advanced-configuration.md)
for how such a job is composed — `build_mcp_server` is its worked example.

### Example: generate an MCP server from your orb in CI

```yaml
orbs:
  gen-orb-mcp: jerus-org/gen-orb-mcp@0.2.0

workflows:
  build:
    jobs:
      - gen-orb-mcp/generate:
          orb_path: src/@orb.yml
          output: ./mcp-server
          version: "1.0.0"
```

### Example: the full release pipeline in one job

`build_mcp_server` primes, generates, compiles, publishes, and commits back — one job:

```yaml
orbs:
  gen-orb-mcp: jerus-org/gen-orb-mcp@0.2.0

workflows:
  release:
    jobs:
      - gen-orb-mcp/build_mcp_server:
          binary_name: my-orb-mcp
          tag_prefix: my-orb-v
          earliest_version: "1.0.0"
          context: [my-release-context]   # signing + GitHub release credentials
```

The orb source is regenerated automatically on every build by
[gen-circleci-orb](https://github.com/jerus-org/gen-circleci-orb), which introspects
gen-orb-mcp's `--help` output and keeps the orb in sync whenever the CLI changes.

### Adopt the orb in your own pipeline

The orb is public, so **any orb author** can add `build_mcp_server` to their release pipeline to
ship an MCP server for their own orb — it works for any orb, not just gen-orb-mcp's. Its `publish`
and `save` steps need credentials, and the env-var **names** they read are configurable so they
map to whatever your CI secrets are called (there is no jerus-org-specific convention to adopt):

- The **publish** step reads `GITHUB_TOKEN` (plus CircleCI's own `CIRCLE_*` vars) to attach the
  binary to a GitHub release. Override the tag source with `--tag-env` or `[publish].tag_env` in
  `gen-orb-mcp.toml` (default `CIRCLE_TAG`).
- The **save** step GPG-signs and pushes the regenerated artifacts. It reads the signing material
  from env vars whose names default to `GPG_KEY`, `GPG_TRUST`, `GIT_USER_NAME`, `GIT_USER_EMAIL`,
  and `GPG_SIGN_KEY`, each overridable via a `--*-env` flag or the `[sign]` section.

Only the **names** are configured (via flag or file); the secret **values** always come from the
CI context at runtime — nothing sensitive is committed. Resolution precedence is
`--*-env flag > gen-orb-mcp.toml > built-in default`. A minimal mapping to existing secrets:

```toml
# gen-orb-mcp.toml — map the generic names onto your own CI secret names
[sign]
gpg_key_env    = "MY_GPG_KEY"
trust_env      = "MY_GPG_TRUST"
user_name_env  = "MY_BOT_NAME"
user_email_env = "MY_BOT_EMAIL"
sign_key_env   = "MY_GPG_SIGN_KEY"

[publish]
tag_env = "CIRCLE_TAG"
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

### `build` — Compile generated MCP server source

```
gen-orb-mcp build [OPTIONS] --input <DIR>

Options:
  -i, --input <DIR>     Directory containing generated Cargo.toml (required)
  -n, --name <NAME>     Binary name (default: read from Cargo.toml [package] name)
      --target <TRIPLE> Cargo target triple for cross-compilation (optional)
      --dry-run         Print what would run without executing cargo build
```

Runs `cargo build --release` inside `<input>`. On success, prints the path to the compiled binary.
The release does not need a pre-existing Rust toolchain beyond what is available in the CI executor.

### `publish` — Upload a binary to a GitHub release

```
gen-orb-mcp publish [OPTIONS] --binary <PATH> --asset-name <NAME>

Options:
  -b, --binary <PATH>       Path to the compiled binary file (required)
  -a, --asset-name <NAME>   Name of the release asset as it appears on GitHub (required)
      --tag <TAG>            Git tag identifying the release [default: $CIRCLE_TAG]
      --tag-env <NAME>       Env-var NAME holding the tag (overrides [publish].tag_env
                             in gen-orb-mcp.toml; built-in default CIRCLE_TAG)
      --dry-run              Print what would be uploaded without calling the API
```

**Prerequisite**: the GitHub release must already exist. `publish` only adds an asset — it does
not create the release. Environment variables read at runtime:

| Variable | Purpose |
|---|---|
| `GITHUB_TOKEN` | Personal access token with `contents: write` |
| `CIRCLE_PROJECT_USERNAME` | Repository owner |
| `CIRCLE_PROJECT_REPONAME` | Repository name |
| `CIRCLE_TAG` | Tag used to locate the release (overridden by `--tag`) |

### `save` — Stage, commit, and push generated artifacts

```
gen-orb-mcp save [OPTIONS] <PATHS>...

Arguments:
  <PATHS>...   Paths to stage (files or directories)

Options:
  -m, --message <MSG>       Commit message [default: "chore: update generated MCP server artifacts [skip ci]"]
      --push                Push to origin after committing [default: true]
      --no-push             Skip the push step
      --gpg-key-env <NAME>  Env-var NAME for the base64 GPG key      (default GPG_KEY)
      --trust-env <NAME>    Env-var NAME for the GPG ownertrust      (default GPG_TRUST)
      --user-name-env <N>   Env-var NAME for the commit author name  (default GIT_USER_NAME)
      --user-email-env <N>  Env-var NAME for the commit author email (default GIT_USER_EMAIL)
      --sign-key-env <NAME> Env-var NAME for the GPG signing key id  (default GPG_SIGN_KEY)
      --dry-run             Stage and diff without creating a commit
```

Each `--*-env` flag overrides the matching `[sign]` entry in `gen-orb-mcp.toml`; unset, the
built-in generic defaults apply. Only names are configured — the secret values are read from
the named env vars at runtime.

Idempotent: if the working tree is clean after staging all paths (no changes), `save` exits 0
without creating a commit. The default commit message includes `[skip ci]` to prevent CI
from triggering a new pipeline on the generated artifact commit.

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
