# gen-orb-mcp: Quick Start

## Installation

```bash
cargo install gen-orb-mcp
# or
cargo binstall gen-orb-mcp
```

## Scenario A: Basic orb documentation server

You have a private CircleCI orb and want Claude Code (or another AI assistant) to understand it.

```bash
# Generate the MCP server source code
gen-orb-mcp generate \
  --orb-path ./my-orb/src/@orb.yml \
  --output ./my-orb-mcp \
  --version 1.0.0

# Compile to a binary
cd my-orb-mcp && cargo build --release

# The binary is at: my-orb-mcp/target/release/my_orb_mcp
```

Add to `.claude.json` or `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "my-orb": {
      "command": "/absolute/path/to/my_orb_mcp"
    }
  }
}
```

The AI assistant can now answer questions about your orb's commands, jobs, and executors.

---

## Scenario B: Multi-version server with migration Tools

You have a breaking change in your orb and want the AI assistant to help users migrate.

### Step 1: Save the previous orb version

```bash
mkdir prior-versions
cp my-orb-4.7.1.yml prior-versions/4.7.1.yml
```

### Step 2: Compute conformance rules

```bash
mkdir migrations
gen-orb-mcp diff \
  --current ./my-orb/src/@orb.yml \
  --previous ./prior-versions/4.7.1.yml \
  --since-version 5.0.0 \
  --output ./migrations/5.0.0.json
```

The output is a JSON file describing what changed: renamed jobs, removed parameters, absorbed
jobs, etc.

### Step 3: Generate the server with everything embedded

```bash
gen-orb-mcp generate \
  --orb-path ./my-orb/src/@orb.yml \
  --output ./my-orb-mcp \
  --version 5.0.0 \
  --migrations ./migrations/ \
  --prior-versions ./prior-versions/ \
  --force

cd my-orb-mcp && cargo build --release
```

The generated server now exposes:
- Current-version resources (`orb://commands/...`, `orb://jobs/...`)
- Prior-version resources (`orb://v4.7.1/commands/...`)
- `plan_migration` and `apply_migration` MCP Tools

### Step 4: Use with Claude Code

With the MCP server connected, ask Claude:

```
"My .circleci/config.yml uses my-orb@4.7.1. Plan a migration to 5.0.0."
```

Claude will call `plan_migration`, show you the diff, and on approval call `apply_migration`
to update the files in place.

---

## Scenario C: Bulk migration without MCP

Migrate all consumer repos from the CLI directly (no MCP server needed):

```bash
# Dry run — see what would change
gen-orb-mcp migrate \
  --ci-dir /path/to/consumer/.circleci \
  --orb my-orb \
  --rules ./migrations/5.0.0.json \
  --dry-run

# Apply
gen-orb-mcp migrate \
  --ci-dir /path/to/consumer/.circleci \
  --orb my-orb \
  --rules ./migrations/5.0.0.json
```

---

## Integrating into a release pipeline

### On CircleCI: use the orb

The fastest path on CircleCI is the public `jerus-org/gen-orb-mcp` orb. Its `build_mcp_server`
job runs the whole journey — prime prior versions, generate and compile the MCP server, publish
the binary to the release, and commit the artifacts back — in a single step:

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

**Prerequisite**: the GitHub release for the tag must already exist before the publish step
runs — create it earlier in your workflow (e.g. via `pcu` or `gh release create`).

The individual subcommand jobs (`generate`, `validate`, `diff`, `migrate`, `build`) are also
available when you want a single step; see the
[README](../crates/gen-orb-mcp/README.md#circleci-orb) for the full job reference.
`build_mcp_server` is a composed job — how it is assembled is covered in gen-circleci-orb's
[Advanced Configuration Guide](https://github.com/jerus-org/gen-circleci-orb/blob/main/docs/advanced-configuration.md).

### On other CI systems

gen-orb-mcp is a plain CLI, so it integrates with any CI (GitHub Actions, GitLab CI, and others
for which no orb is provided). The [CI Integration Guide](CI_INTEGRATION_GUIDE.md) shows the
manual pattern — generate the binary and upload it to the release — as reusable steps you can
translate to your platform.

---

## Common options reference

| Flag | Command | Description |
|---|---|---|
| `--orb-path` | generate, validate | Path to orb YAML |
| `--output` | generate | Output directory |
| `--version` | generate | Crate version for generated server |
| `--format binary` | generate | Compile to binary instead of source |
| `--force` | generate | Overwrite existing output |
| `--migrations <dir>` | generate | Embed conformance rules + enable Tools |
| `--prior-versions <dir>` | generate | Embed prior orb snapshots |
| `--current / --previous` | diff | Orb YAMLs to compare |
| `--since-version` | diff | Version string to embed in rules |
| `--ci-dir` | migrate | Consumer `.circleci/` directory |
| `--orb` | migrate | Orb alias in consumer config |
| `--rules` | migrate | Conformance rules JSON from `diff` |
| `--dry-run` | migrate | Preview without writing files |
