# CircleCI Configuration Based on pcu Repository Pattern

## Summary

I've created CircleCI configuration files for gen-orb-mcp based on the **pcu repository pattern** you provided. This follows your organizational standards exactly.

---

## What's Been Created

### 1. `circleci-config.yml` → `.circleci/config.yml`
**Main validation workflow** that runs on every push:

**Key Features:**
- Routes commits based on committer (bot vs. human) using `toolkit/choose_pipeline`
- Full validation suite using standard toolkit jobs:
  - `required_builds` - Build on min Rust version (1.85)
  - `optional_builds` - Build on stable/nightly
  - `common_tests` - Run test suite
  - `idiomatic_rust` - Clippy, rustfmt, doc tests
  - `security` - cargo-deny, SonarCloud
  - `update_prlog` - Auto-update PRLOG.md on PR branches
- Separate workflows for validation, success, and release triggers

**Customizations from pcu:**
- Removed pcu-specific test jobs (test-setup, test-commit, test-push, test-bsky)
- Changed context from `pcu-app` to `gen-orb-mcp-app`
- Kept all standard toolkit validation patterns

### 2. `circleci-release.yml` → `.circleci/release.yml`
**Release workflow** triggered by schedule or manually:

**Key Features:**
- Custom commands extending toolkit functionality:
  - `get_next_version` - Calculate semantic version with `--prefix gen-orb-mcp-v`
  - `check_crates_io_version` - Check if version already published (recovery)
  - `make_cargo_release` - Run cargo release with conditional publish
  - `make_github_release` - Create GitHub release and update PRLOG
- Sequential release job: `tools` → `release-crate`
- Tag format: `gen-orb-mcp-v{VERSION}` (e.g., `gen-orb-mcp-v0.1.0`)

**Customizations from pcu:**
- Simplified from multi-crate to single-crate release
- Removed sequential crate chain (gen-bsky → gen-linkedin → pcu → prlog)
- Single `release-crate` job handles everything
- Removed `release-prlog` job (handled in `make_github_release` with `--update-prlog`)

### 3. `CIRCLECI_SETUP_GUIDE.md`
**Comprehensive setup and usage guide** covering:
- Overview of both config files and how they work together
- Complete setup instructions (contexts, SSH keys, scheduled triggers)
- How to trigger workflows (automatic and manual)
- Detailed release process walkthrough
- Troubleshooting guide
- Testing procedures

### 4. Updated Documentation
- **QUICKSTART.md** - Updated to reference actual CircleCI configs
- **FINALIZED_PLAN.md** - Updated CI/CD section with pcu pattern details

---

## How It Differs from Standard Toolkit

### Standard Toolkit Usage (Simple)
```yaml
workflows:
  main:
    jobs:
      - toolkit/rust-check
      - toolkit/rust-test
```

### Your Pattern (Advanced)
```yaml
# Two separate config files
# config.yml - validation
# release.yml - releases

# Extended commands
commands:
  get_next_version:  # Enhanced with --prefix
  check_crates_io_version:  # New - recovery support
  make_cargo_release:  # Enhanced with conditional publish
  make_github_release:  # New - pcu integration

# Sophisticated workflows
workflows:
  check_last_commit:  # Route by committer
  validation:  # Full test suite
  on_success:  # Cleanup
  release:  # Scheduled/manual
```

---

## Why This Pattern?

### Advantages

**1. Separation of Concerns**
- Validation always runs
- Releases only when needed
- Independent troubleshooting

**2. Recovery Support**
- `check_crates_io_version` handles partial failures
- Can re-run release after crates.io publish succeeded
- Prevents duplicate publishes

**3. Automation**
- PRLOG.md auto-updated on PR branches
- GitHub releases created automatically
- Version calculation from conventional commits

**4. Organizational Consistency**
- Same pattern as pcu, gen-changelog, other tools
- Familiar to team members
- Reusable custom commands

**5. Flexibility**
- Schedule releases (weekly/monthly)
- Manual trigger anytime
- Test validation independently

---

## Setup Checklist

Use this checklist to set up CircleCI for gen-orb-mcp:

### Repository Setup
- [ ] Copy `circleci-config.yml` to `.circleci/config.yml`
- [ ] Copy `circleci-release.yml` to `.circleci/release.yml`
- [ ] Update SSH fingerprint in both files (parameter)
- [ ] Commit and push to trigger first validation

### CircleCI Project Setup
- [ ] Enable project in CircleCI
- [ ] Create context: `release`
  - [ ] Add `CARGO_REGISTRY_TOKEN`
  - [ ] Add `GPG_KEY`
  - [ ] Add `GPG_PASS`
- [ ] Create context: `bot-check`
  - [ ] Add `BOT_USER`
- [ ] Create context: `gen-orb-mcp-app` (optional)
- [ ] Create context: `SonarCloud` (optional)
  - [ ] Add `SONAR_TOKEN`

### SSH Key Setup
- [ ] Generate SSH key: `ssh-keygen -t ed25519 -C "circleci@gen-orb-mcp"`
- [ ] Add public key to GitHub (Settings → SSH keys)
- [ ] Add private key to CircleCI (Project Settings → SSH Keys)
- [ ] Copy fingerprint to config parameters

### Scheduled Release
- [ ] Go to CircleCI → Project Settings → Triggers
- [ ] Add Scheduled Trigger:
  - Name: `release check`
  - Schedule: Weekly (or as desired)
  - Branch: `main`
  - Parameters: `{"release_flag": true}`

### Testing
- [ ] Push to PR branch → Verify validation runs
- [ ] Check PRLOG.md auto-update
- [ ] Verify all toolkit jobs pass
- [ ] (After MVP) Trigger manual release

---

## How to Use

### Normal Development
```bash
# Create feature branch
git checkout -b feature/add-parser
git commit -m "feat: add orb parser"
git push origin feature/add-parser

# CircleCI automatically:
# 1. Runs validation workflow
# 2. Updates PRLOG.md
# 3. All tests must pass
```

### Merge to Main
```bash
# Merge PR
git checkout main
git pull

# CircleCI automatically:
# 1. Runs validation on main
# 2. No PRLOG update (already done on PR)
```

### Release (Scheduled)
```bash
# Every week (or configured schedule):
# CircleCI automatically:
# 1. Checks for commits since last release
# 2. Calculates next version from conventional commits
# 3. Runs cargo release
# 4. Publishes to crates.io
# 5. Creates GitHub release
# 6. Updates PRLOG.md with release date
```

### Release (Manual)
```bash
# Via CircleCI UI:
# 1. Go to project in CircleCI
# 2. Select branch: main
# 3. Trigger Pipeline
# 4. Add parameter: {"release_flag": true}
# 5. Run Pipeline

# Via API:
curl -X POST \
  https://circleci.com/api/v2/project/github/jerus-org/gen-orb-mcp/pipeline \
  -H "Circle-Token: $CIRCLECI_TOKEN" \
  -d '{"branch":"main","parameters":{"release_flag":true}}'
```

---

## File Mapping

Copy these files from `/mnt/user-data/outputs/` to your repository:

| Output File | Repository Location | Purpose |
|-------------|-------------------|---------|
| `circleci-config.yml` | `.circleci/config.yml` | Main validation workflow |
| `circleci-release.yml` | `.circleci/release.yml` | Release workflow |
| `CIRCLECI_SETUP_GUIDE.md` | `docs/circleci-setup.md` | Setup and usage guide |

---

## Key Differences: pcu vs gen-orb-mcp

| Aspect | pcu (Multi-Crate) | gen-orb-mcp (Single-Crate) |
|--------|------------------|---------------------------|
| Crates | 3 (gen-bsky, gen-linkedin, pcu) | 1 (gen-orb-mcp) |
| Release Jobs | 4 sequential jobs | 1 job |
| Test Jobs | Custom pcu-specific tests | Standard toolkit only |
| Tag Prefix | Multiple (gen-bsky-v, pcu-v, v) | One (gen-orb-mcp-v) |
| Contexts | pcu-app, bluesky | gen-orb-mcp-app |
| Complexity | High (workspace coordination) | Low (single crate) |

---

## Next Steps

1. **Review the configs** - Check `circleci-config.yml` and `circleci-release.yml`
2. **Read setup guide** - See `CIRCLECI_SETUP_GUIDE.md` for detailed instructions
3. **Copy to repository** - When ready to set up CircleCI
4. **Configure contexts** - Add secrets to CircleCI project
5. **Test validation** - Push a PR to verify it works
6. **Schedule releases** - Set up weekly/monthly release checks

---

## Questions?

See `CIRCLECI_SETUP_GUIDE.md` for:
- Complete setup walkthrough
- Troubleshooting guide
- Testing procedures
- Detailed command explanations

The configs follow your exact organizational pattern from pcu - just simplified for a single-crate project!
