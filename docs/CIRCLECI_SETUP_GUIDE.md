# CircleCI Setup Guide for gen-orb-mcp

## Overview

gen-orb-mcp uses two separate CircleCI configuration files following the pcu repository pattern:

1. **`.circleci/config.yml`** - Main validation workflow (runs on every push)
2. **`.circleci/release.yml`** - Release workflow (runs on scheduled trigger or manual)

This separation allows independent management of validation and release processes.

---

## Configuration Files

### File 1: `.circleci/config.yml` (Main Validation)

**Location:** `.circleci/config.yml`

**Purpose:** 
- Runs on every push to validate code quality
- Handles bot vs. human commits differently
- Updates PRLOG.md automatically
- Triggers validation pipeline

**Workflows:**
1. `check_last_commit` - Routes based on committer (bot vs. human)
2. `validation` - Full test suite for main branch and PRs
3. `on_success` - Cleanup after successful validation
4. `release` - Triggers release workflow (scheduled or manual)

**Key Jobs:**
- `toolkit/required_builds` - Build on minimum Rust version
- `toolkit/optional_builds` - Build on stable/nightly
- `toolkit/common_tests` - Run test suite
- `toolkit/idiomatic_rust` - Clippy, rustfmt, doc tests
- `toolkit/security` - cargo-deny, SonarCloud
- `toolkit/update_prlog` - Auto-update PRLOG.md

**Pipeline Parameters:**
```yaml
parameters:
  min_rust_version: "1.86"           # Minimum Rust version
  fingerprint: "SHA256:..."          # SSH key fingerprint
  validation_flag: false             # Trigger validation
  success_flag: false                # Trigger success
  release_flag: false                # Trigger release
```

---

### File 2: `.circleci/release.yml` (Release Workflow)

**Location:** `.circleci/release.yml`

**Purpose:**
- Calculate next version using nextsv
- Check if version exists on crates.io (recovery)
- Run cargo release (with conditional publish)
- Create GitHub release and update PRLOG

**Workflows:**
1. `release` - Sequential release process

**Key Jobs:**
1. `tools` - Verify nextsv, pcu, cargo-release, jq installed
2. `release-crate` - Full release sequence

**Custom Commands:**
- `get_next_version` - Calculate semantic version from commits
- `check_crates_io_version` - Verify if already published
- `make_cargo_release` - Execute cargo release
- `make_github_release` - Create GitHub release via pcu

**Tag Format:** `gen-orb-mcp-v{VERSION}` (e.g., `gen-orb-mcp-v0.1.0`)

---

## How They Work Together

### Normal Development Flow

```
Push to PR branch
    ↓
config.yml → check_last_commit workflow
    ↓
Determines if commit is from bot or human
    ↓
If human → validation workflow
    ├── Required builds
    ├── Optional builds  
    ├── Common tests
    ├── Idiomatic rust (clippy, fmt, doc)
    ├── Security (cargo-deny, SonarCloud)
    └── Update PRLOG (if on PR branch)
    ↓
on_success workflow (cleanup)
```

### Release Flow

```
Scheduled trigger OR manual release_flag=true
    ↓
config.yml → release workflow
    ↓
Triggers release.yml → release workflow
    ↓
1. tools job (verify tooling)
    ↓
2. release-crate job:
    ├── Calculate next version (nextsv)
    ├── Check crates.io (recovery check)
    ├── cargo release (publish to crates.io)
    ├── Create GitHub release
    └── Update PRLOG.md
    ↓
Tag pushed: gen-orb-mcp-v0.1.0
GitHub Release created
Crates.io published
PRLOG.md updated
```

---

## Setup Instructions

### Step 1: Create Configuration Files

**Create `.circleci/config.yml`:**
```bash
mkdir -p .circleci
# Copy circleci-config.yml content to .circleci/config.yml
```

**Create `.circleci/release.yml`:**
```bash
# Copy circleci-release.yml content to .circleci/release.yml
```

### Step 2: Configure CircleCI Contexts

Required contexts in CircleCI project settings:

**Context: `release`**
- `CARGO_REGISTRY_TOKEN` - crates.io API token
- `GPG_KEY` - GPG private key for signing
- `GPG_PASS` - GPG passphrase

**Context: `bot-check`**
- `BOT_USER` - Bot username (e.g., "jerus-bot")

**Context: `gen-orb-mcp-app`** (optional)
- Application-specific secrets if needed

**Context: `SonarCloud`** (optional, for security scanning)
- `SONAR_TOKEN` - SonarCloud token

### Step 3: Add SSH Key

1. Generate SSH key for CircleCI:
   ```bash
   ssh-keygen -t ed25519 -C "circleci@gen-orb-mcp"
   ```

2. Add public key to GitHub (Settings → SSH keys)

3. Add private key to CircleCI (Project Settings → SSH Keys)

4. Copy fingerprint to `config.yml` and `release.yml` parameters

### Step 4: Configure Scheduled Release

In CircleCI project settings:

1. Go to Triggers → Add Scheduled Trigger
2. Name: `release check`
3. Schedule: Weekly (or as desired)
4. Branch: `main`
5. Parameters:
   ```json
   {
     "release_flag": true
   }
   ```

---

## How to Trigger Workflows

### Automatic (Standard Flow)

**On Push to PR:**
```bash
git push origin feature-branch
# Triggers: validation workflow
# Result: Tests run, PRLOG updated on PR branch
```

**On Push to Main:**
```bash
git push origin main
# Triggers: validation workflow
# Result: Full validation, no PRLOG update
```

### Manual Release

**Option 1: Via CircleCI Web UI**
1. Go to CircleCI project
2. Select branch: `main`
3. Trigger Pipeline with parameters:
   ```json
   {
     "release_flag": true
   }
   ```

**Option 2: Via CircleCI API**
```bash
curl -X POST \
  https://circleci.com/api/v2/project/github/jerus-org/gen-orb-mcp/pipeline \
  -H "Circle-Token: $CIRCLECI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "branch": "main",
    "parameters": {
      "release_flag": true
    }
  }'
```

**Option 3: Via Local Development (Testing)**
```bash
# Trigger validation only
circleci local execute --job toolkit/common_tests

# Note: Cannot trigger release workflow locally
# (requires CircleCI infrastructure for secrets)
```

---

## Release Process Details

### What Happens During Release

**1. Version Calculation (`get_next_version`)**
```bash
# Uses nextsv to analyze commits since last tag
nextsv -q -bn calculate --package gen-orb-mcp --prefix gen-orb-mcp-v

# Sets environment variables:
# NEXT_VERSION=0.1.0
# SEMVER=0.1.0
```

**2. Crates.io Check (`check_crates_io_version`)**
```bash
# Queries crates.io API
curl https://crates.io/api/v1/crates/gen-orb-mcp/versions

# Sets SKIP_PUBLISH=true if version exists
# (Handles recovery from failed workflow)
```

**3. Cargo Release (`make_cargo_release`)**
```bash
# Generates CHANGELOG.md via release-hook.sh
./crates/gen-orb-mcp/release-hook.sh 0.1.0

# Runs cargo release
cargo release --execute --no-confirm --sign-tag \
  --package gen-orb-mcp -vv 0.1.0

# Creates:
# - Git commit with version bump
# - Git tag: gen-orb-mcp-v0.1.0 (signed)
# - Publishes to crates.io (unless SKIP_PUBLISH=true)
```

**4. GitHub Release (`make_github_release`)**
```bash
# Uses pcu to create GitHub release
pcu -vv release --package gen-orb-mcp --update-prlog

# Creates:
# - GitHub release from tag
# - Updates PRLOG.md with release date
# - Pushes PRLOG.md changes
```

---

## Troubleshooting

### Issue: Validation Fails on PR

**Symptoms:** Clippy errors, test failures, formatting issues

**Solution:**
```bash
# Run locally before pushing
just test      # Runs clippy, check, doc, tests
just fmt       # Format code
just clippy    # Check lints
```

### Issue: Release Workflow Doesn't Trigger

**Symptoms:** Scheduled release doesn't run

**Check:**
1. Scheduled trigger configured correctly
2. `release_flag: true` in parameters
3. Main branch has commits since last release
4. CircleCI project is enabled

**Manual Trigger:**
```bash
# Trigger release manually via API
curl -X POST \
  https://circleci.com/api/v2/project/github/jerus-org/gen-orb-mcp/pipeline \
  -H "Circle-Token: $CIRCLECI_TOKEN" \
  -d '{"branch":"main","parameters":{"release_flag":true}}'
```

### Issue: Cargo Release Fails

**Symptoms:** "Version already exists on crates.io"

**This is handled automatically!**
- `check_crates_io_version` sets `SKIP_PUBLISH=true`
- `make_cargo_release` adds `--no-publish` flag
- Rest of release process continues (tag, GitHub release)

**Manual Recovery:**
```bash
# If workflow failed after crates.io publish:
# 1. Re-trigger release workflow
# 2. It will skip publish, complete GitHub release
```

### Issue: PRLOG Not Updated

**Symptoms:** PRLOG.md not showing latest release

**Solution:**
```bash
# Verify pcu is installed in release job
pcu --version

# Check GitHub release was created
# PRLOG update happens in make_github_release step

# Manual fix if needed:
git pull origin main  # Get latest PRLOG changes
```

---

## Differences from Standard Toolkit Usage

### Enhanced Commands

This project extends `circleci-toolkit` with custom commands:

1. **`get_next_version`**
   - Adds `--prefix` support for crate-specific tags
   - Sets both `NEXT_VERSION` and `SEMVER`

2. **`check_crates_io_version`**
   - New command for recovery scenarios
   - Prevents duplicate publishes

3. **`make_cargo_release`**
   - Respects `SKIP_PUBLISH` environment variable
   - Handles recovery from partial failures

4. **`make_github_release`**
   - Uses `pcu release --package`
   - Auto-updates PRLOG.md

### Why Two Config Files?

**Separation of Concerns:**
- `config.yml` - Always active, validates every push
- `release.yml` - Only runs when explicitly triggered

**Benefits:**
- Independent workflows
- Easier to test release process
- Clear separation of validation vs. release
- Follows organizational pattern (see pcu repo)

---

## Testing Before First Release

### Test Validation Workflow

```bash
# Push to PR branch
git checkout -b test-ci
git commit --allow-empty -m "test: CI validation"
git push origin test-ci

# Check CircleCI:
# - validation workflow should run
# - All toolkit jobs should pass
# - PRLOG should be updated
```

### Test Release Workflow (Dry Run)

**Not recommended for first release** - Use actual release instead.

For testing, you can:
1. Create test tag manually
2. Verify nextsv detects it
3. Trigger release with `release_flag`
4. Monitor each step in CircleCI

---

## CircleCI Toolkit Jobs Reference

Jobs used from `jerus-org/circleci-toolkit@4.2.0`:

| Job | Purpose |
|-----|---------|
| `choose_pipeline` | Routes based on committer (bot vs. human) |
| `label` | Adds labels to PRs |
| `required_builds` | Build on minimum Rust version |
| `optional_builds` | Build on stable/nightly |
| `test_doc_build` | Generate and test documentation |
| `common_tests` | Run test suite |
| `idiomatic_rust` | Clippy, rustfmt, doc tests |
| `security` | cargo-deny, SonarCloud scanning |
| `update_prlog` | Update PRLOG.md on PR branch |
| `end_success` | Cleanup after validation |
| `make_release` | Triggers release workflow |

---

## Next Steps

1. **Create configuration files** in `.circleci/`
2. **Configure contexts** in CircleCI project settings
3. **Add SSH key** to GitHub and CircleCI
4. **Test validation** by pushing to PR branch
5. **Configure scheduled release** trigger
6. **Perform first release** when MVP is complete

All configuration files are in `/mnt/user-data/outputs/`:
- `circleci-config.yml` → Copy to `.circleci/config.yml`
- `circleci-release.yml` → Copy to `.circleci/release.yml`
