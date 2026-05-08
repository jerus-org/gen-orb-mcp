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

### Option A: Using the gen-orb-mcp CircleCI orb

The `jerus-org/gen-orb-mcp` orb exposes each subcommand as a reusable job with gen-orb-mcp pre-installed. Add it to your workflow and combine with whatever build or upload steps fit your deployment:

```yaml
orbs:
  gen-orb-mcp: jerus-org/gen-orb-mcp@0.1

workflows:
  release:
    jobs:
      - gen-orb-mcp/prime:
          orb_path: src/@orb.yml
          earliest_version: "4.1.0"
          ephemeral: true

      - gen-orb-mcp/generate:
          orb_path: src/@orb.yml
          output: /tmp/mcp-build
          version: "${CIRCLE_TAG#v}"
          migrations: /tmp/gen-orb-mcp-prime/migrations
          prior_versions: /tmp/gen-orb-mcp-prime/prior-versions
          requires: [gen-orb-mcp/prime]
```

What to do with the generated source (compile it, store it, upload it) is left to you. See `CI_INTEGRATION_GUIDE.md` for storage and upload options.

### Option B: Run commands directly in a job

Install gen-orb-mcp in a custom executor and run the commands as run steps:

```yaml
jobs:
  generate-mcp:
    docker:
      - image: cimg/rust:1.85
    steps:
      - checkout
      - run:
          name: Install gen-orb-mcp
          command: cargo binstall gen-orb-mcp --no-confirm
      - run:
          name: Populate prior-version history
          command: |
            eval "$(gen-orb-mcp prime \
              --orb-path src/@orb.yml \
              --earliest-version "${EARLIEST_VERSION}" \
              --ephemeral)"
      - run:
          name: Generate MCP server source
          command: |
            gen-orb-mcp generate \
              --orb-path src/@orb.yml \
              --output /tmp/mcp-build \
              --version "${CIRCLE_TAG#v}" \
              --migrations "${PRIME_MIG_DIR}" \
              --prior-versions "${PRIME_PV_DIR}"
      - run:
          name: Compile
          no_output_timeout: 15m
          command: cd /tmp/mcp-build && cargo build --release
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
