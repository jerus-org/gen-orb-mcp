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

In your orb's CircleCI release workflow, add a step after the orb is published:

```yaml
- run:
    name: Generate migration rules
    command: |
      gen-orb-mcp diff \
        --current src/@orb.yml \
        --previous "$(cat previous-version.yml)" \
        --since-version "$ORB_VERSION" \
        --output migrations/"$ORB_VERSION".json

- run:
    name: Generate MCP server
    command: |
      gen-orb-mcp generate \
        --orb-path src/@orb.yml \
        --output dist/mcp \
        --version "$ORB_VERSION" \
        --migrations migrations/ \
        --prior-versions prior-versions/ \
        --force
```

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
