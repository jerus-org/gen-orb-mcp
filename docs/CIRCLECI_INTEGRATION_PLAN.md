# CircleCI-Toolkit Integration Plan

## Overview

This document outlines the integration of gen-orb-mcp into the jerus-org/circleci-toolkit orb release workflow, enabling automatic MCP server generation on every orb release.

---

## Current State

**Repository:** https://github.com/jerus-org/circleci-toolkit  
**Orb Namespace:** jerus-org/circleci-toolkit  
**Current Version:** v3.x (assumed)  
**Release Process:** Standard CircleCI orb release workflow

**Typical Release Workflow:**
1. Developer makes changes to orb
2. Creates PR with version bump
3. PR merged to main
4. Tag pushed (e.g., v3.1.0)
5. CircleCI publishes orb to registry

---

## Integration Goals

1. **Automatic Generation:** MCP server generated on every orb release
2. **Version Alignment:** MCP server version matches orb version
3. **Multiple Formats:** Produce binary, container, and skill formats
4. **Artifact Storage:** Store generated artifacts for distribution
5. **Zero Manual Work:** Fully automated process

---

## Architecture

### Release Workflow with MCP Generation

```
┌─────────────────────────────────────────────────────────┐
│  CircleCI Release Workflow                               │
│                                                          │
│  1. Tag Push (v3.1.0)                                   │
│      ↓                                                   │
│  2. Validate Orb                                        │
│      ↓                                                   │
│  3. Publish Orb to CircleCI Registry                    │
│      ↓                                                   │
│  4. Generate MCP Server (NEW)                           │
│      ├── Binary (Linux, macOS, Windows)                 │
│      ├── Container (ghcr.io/jerus-org/toolkit-mcp)     │
│      └── Skill (SKILL.md)                               │
│      ↓                                                   │
│  5. Upload Artifacts                                    │
│      ├── GitHub Release Attachments                     │
│      └── Container Registry                             │
│      ↓                                                   │
│  6. Update Documentation                                │
└─────────────────────────────────────────────────────────┘
```

---

## Implementation Steps

### Step 1: Add gen-orb-mcp to CI Environment

**File:** `.circleci/config.yml`

**Changes:**

```yaml
version: 2.1

# Add Rust orb for building gen-orb-mcp
orbs:
  orb-tools: circleci/orb-tools@12.0
  rust: circleci/rust@1.6

# Executors
executors:
  rust-executor:
    docker:
      - image: cimg/rust:1.75
```

---

### Step 2: Create MCP Generation Job

```yaml
jobs:
  generate-mcp-server:
    executor: rust-executor
    steps:
      - checkout
      
      # Install gen-orb-mcp
      - run:
          name: Install gen-orb-mcp
          command: |
            cargo install gen-orb-mcp
            gen-orb-mcp --version
      
      # Generate MCP server (all formats)
      - run:
          name: Generate MCP Server
          command: |
            gen-orb-mcp generate \
              --orb-path ./src/@orb.yml \
              --orb-name circleci-toolkit \
              --namespace jerus-org \
              --version ${CIRCLE_TAG#v} \
              --formats binary,container,skill \
              --output ./mcp-output/
      
      # Build binary for multiple platforms
      - run:
          name: Build Multi-Platform Binaries
          command: |
            cd ./mcp-output/binary
            
            # Linux
            cargo build --release --target x86_64-unknown-linux-gnu
            
            # macOS (cross-compile)
            rustup target add x86_64-apple-darwin
            cargo build --release --target x86_64-apple-darwin
            
            # Copy binaries
            mkdir -p ../../artifacts/binaries
            cp target/x86_64-unknown-linux-gnu/release/circleci-toolkit-mcp \
               ../../artifacts/binaries/circleci-toolkit-mcp-linux-amd64
            cp target/x86_64-apple-darwin/release/circleci-toolkit-mcp \
               ../../artifacts/binaries/circleci-toolkit-mcp-darwin-amd64
      
      # Build and push container
      - setup_remote_docker:
          docker_layer_caching: true
      
      - run:
          name: Build and Push Container
          command: |
            cd ./mcp-output/container
            
            # Login to GitHub Container Registry
            echo "$GITHUB_TOKEN" | docker login ghcr.io -u "$GITHUB_USERNAME" --password-stdin
            
            # Build
            docker build -t ghcr.io/jerus-org/circleci-toolkit-mcp:${CIRCLE_TAG#v} .
            docker tag ghcr.io/jerus-org/circleci-toolkit-mcp:${CIRCLE_TAG#v} \
                       ghcr.io/jerus-org/circleci-toolkit-mcp:latest
            
            # Push
            docker push ghcr.io/jerus-org/circleci-toolkit-mcp:${CIRCLE_TAG#v}
            docker push ghcr.io/jerus-org/circleci-toolkit-mcp:latest
      
      # Copy skill file
      - run:
          name: Prepare Skill File
          command: |
            mkdir -p ./artifacts/skill
            cp ./mcp-output/skill/SKILL.md ./artifacts/skill/
      
      # Store artifacts
      - store_artifacts:
          path: ./artifacts
          
      # Persist for GitHub release
      - persist_to_workspace:
          root: ./artifacts
          paths:
            - binaries/*
            - skill/*
```

---

### Step 3: Update Release Workflow

```yaml
workflows:
  version: 2
  
  # Standard development workflow
  build-and-test:
    jobs:
      - orb-tools/lint
      - orb-tools/pack
  
  # Release workflow (on tags)
  release:
    jobs:
      # Existing orb release
      - orb-tools/lint:
          filters: &release-filters
            tags:
              only: /^v.*/
            branches:
              ignore: /.*/
      
      - orb-tools/pack:
          requires:
            - orb-tools/lint
          filters: *release-filters
      
      - orb-tools/publish:
          orb-name: jerus-org/circleci-toolkit
          version: ${CIRCLE_TAG#v}
          requires:
            - orb-tools/pack
          filters: *release-filters
      
      # NEW: MCP server generation
      - generate-mcp-server:
          requires:
            - orb-tools/publish
          filters: *release-filters
      
      # NEW: Create GitHub release
      - create-github-release:
          requires:
            - generate-mcp-server
          filters: *release-filters
```

---

### Step 4: GitHub Release Job

```yaml
jobs:
  create-github-release:
    docker:
      - image: cibuilds/github:0.13
    steps:
      - attach_workspace:
          at: ./artifacts
      
      - run:
          name: Create GitHub Release
          command: |
            VERSION=${CIRCLE_TAG}
            
            # Create release notes
            cat > release-notes.md <<EOF
            # CircleCI Toolkit ${VERSION}
            
            ## Orb
            Published to CircleCI registry: \`jerus-org/circleci-toolkit@${VERSION#v}\`
            
            ## MCP Server
            
            AI coding assistants can now access toolkit documentation via MCP server.
            
            ### Installation Options
            
            **Binary:**
            \`\`\`bash
            # Download for your platform
            curl -L -o circleci-toolkit-mcp \\
              https://github.com/jerus-org/circleci-toolkit/releases/download/${VERSION}/circleci-toolkit-mcp-linux-amd64
            chmod +x circleci-toolkit-mcp
            \`\`\`
            
            **Container:**
            \`\`\`bash
            docker pull ghcr.io/jerus-org/circleci-toolkit-mcp:${VERSION#v}
            \`\`\`
            
            **Skill File:**
            Download SKILL.md and place in Claude's skills directory.
            
            ### Usage
            
            Configure Claude Code:
            \`\`\`json
            {
              "mcp_servers": {
                "circleci-toolkit": {
                  "command": "./circleci-toolkit-mcp"
                }
              }
            }
            \`\`\`
            EOF
            
            # Create release with artifacts
            ghr -t ${GITHUB_TOKEN} \
                -u ${CIRCLE_PROJECT_USERNAME} \
                -r ${CIRCLE_PROJECT_REPONAME} \
                -c ${CIRCLE_SHA1} \
                -n "Release ${VERSION}" \
                -b "$(cat release-notes.md)" \
                ${VERSION} \
                ./artifacts/binaries/
```

---

## Environment Variables

Required in CircleCI project settings:

```bash
# GitHub (for container registry and releases)
GITHUB_TOKEN=<personal_access_token>  # With packages:write, repo permissions
GITHUB_USERNAME=<github_username>

# Optional: CircleCI API (if needed)
CIRCLECI_API_TOKEN=<api_token>
```

---

## Repository Changes

### 1. Add Documentation

**File:** `docs/mcp-server.md`

```markdown
# CircleCI Toolkit MCP Server

The CircleCI Toolkit MCP Server enables AI coding assistants like Claude Code
to understand and work with the circleci-toolkit orb.

## Installation

### Binary

Download the binary for your platform:

- [Linux (amd64)](https://github.com/jerus-org/circleci-toolkit/releases/latest/download/circleci-toolkit-mcp-linux-amd64)
- [macOS (amd64)](https://github.com/jerus-org/circleci-toolkit/releases/latest/download/circleci-toolkit-mcp-darwin-amd64)

Make it executable:
```bash
chmod +x circleci-toolkit-mcp
```

### Container

```bash
docker pull ghcr.io/jerus-org/circleci-toolkit-mcp:latest
```

### Skill File

Download [SKILL.md](https://github.com/jerus-org/circleci-toolkit/releases/latest/download/SKILL.md)
and place in `~/.claude/skills/circleci-toolkit/`.

## Configuration

### Claude Code

Add to `.claude/config.json`:

```json
{
  "mcp_servers": {
    "circleci-toolkit": {
      "command": "/path/to/circleci-toolkit-mcp"
    }
  }
}
```

### Claude Desktop

Add to config file:

```json
{
  "mcpServers": {
    "circleci-toolkit": {
      "command": "/path/to/circleci-toolkit-mcp"
    }
  }
}
```

## Usage

Once configured, you can ask Claude about the toolkit:

- "What parameters does build-job accept?"
- "Show me an example of using deploy-job"
- "What executors are available in the toolkit?"

## Version Compatibility

The MCP server version matches the orb version. If you're using
circleci-toolkit@3.1.0, use the v3.1.0 MCP server.

## Troubleshooting

### Server Won't Start

Check that the binary is executable and in your PATH.

### Claude Can't Connect

Verify the path in your configuration is correct and absolute.

### Wrong Version Information

Make sure the MCP server version matches your orb version.
```

---

### 2. Update Main README

**File:** `README.md`

Add section:

```markdown
## AI Assistant Integration

The circleci-toolkit orb includes an MCP (Model Context Protocol) server that enables
AI coding assistants like Claude Code to understand and work with the orb.

[View MCP Server Documentation](docs/mcp-server.md)

Quick start:
```bash
# Download binary
curl -L -o circleci-toolkit-mcp \
  https://github.com/jerus-org/circleci-toolkit/releases/latest/download/circleci-toolkit-mcp-linux-amd64
chmod +x circleci-toolkit-mcp

# Configure Claude Code
echo '{
  "mcp_servers": {
    "circleci-toolkit": {
      "command": "'$(pwd)'/circleci-toolkit-mcp"
    }
  }
}' > ~/.claude/config.json
```

Now Claude can help you work with the toolkit!
```

---

## Testing Plan

### Pre-Integration Testing

1. **Test gen-orb-mcp locally**
   ```bash
   cd circleci-toolkit
   gen-orb-mcp generate \
     --orb-path ./src/@orb.yml \
     --output ./test-output/
   
   # Verify output
   ls -la ./test-output/
   ```

2. **Test generated binary**
   ```bash
   cd ./test-output/binary
   cargo build --release
   ./target/release/circleci-toolkit-mcp &
   
   # Test MCP connection
   # (manual test with Claude Code)
   ```

3. **Test in branch**
   - Create feature branch
   - Add MCP generation job
   - Push with test tag
   - Verify artifacts generated

### Post-Integration Testing

1. **Verify Release Process**
   - Tag release: v3.1.0-rc1
   - Monitor CI pipeline
   - Check artifacts uploaded
   - Verify GitHub release created

2. **Test Artifacts**
   - Download binaries from release
   - Pull container from registry
   - Download skill file
   - Configure Claude Code
   - Test queries

3. **User Acceptance**
   - Distribute to 3+ team members
   - Collect feedback
   - Fix any issues
   - Full release

---

## Rollout Strategy

### Phase 1: Alpha (Week 1)

**Goal:** Validate integration works

**Actions:**
1. Add MCP generation to CI (feature branch)
2. Test with RC tags
3. Fix critical issues
4. Internal team testing only

**Success Criteria:**
- CI pipeline completes
- Artifacts generated correctly
- No blocking issues

### Phase 2: Beta (Week 2)

**Goal:** Community testing

**Actions:**
1. Merge to main
2. Release v3.1.0 with MCP support
3. Announce in project README
4. Gather feedback

**Success Criteria:**
- 5+ users try MCP server
- No critical bugs reported
- Positive feedback

### Phase 3: General Availability (Week 3+)

**Goal:** Full rollout

**Actions:**
1. Document in orb registry
2. Announce in CircleCI Discuss
3. Blog post (optional)
4. Monitor adoption

**Success Criteria:**
- Stable releases
- Growing usage
- Community contributions

---

## Maintenance

### Regular Updates

**On every orb release:**
- MCP server automatically regenerated
- Version stays in sync
- No manual intervention needed

**Quarterly:**
- Review gen-orb-mcp version
- Update if new features available
- Test compatibility

### Monitoring

**Check:**
- GitHub release artifacts
- Container registry pushes
- Download statistics
- User feedback/issues

---

## Rollback Plan

If MCP generation causes issues:

1. **Immediate:**
   - Comment out MCP generation job
   - Push emergency fix
   - Release orb without MCP

2. **Investigation:**
   - Review CI logs
   - Test locally
   - Identify root cause

3. **Fix:**
   - Update gen-orb-mcp version
   - Fix template issues
   - Re-enable job

---

## Cost Analysis

### CI/CD Time

**Per Release:**
- Orb publish: ~2 minutes (existing)
- MCP generation: ~5 minutes
- Binary builds: ~3 minutes
- Container build: ~4 minutes
- Total added: ~12 minutes

**Credits:**
- Estimated: 15-20 credits per release
- Releases per month: ~2-4
- Monthly cost: ~30-80 credits (~$0-5)

### Storage

**GitHub:**
- Binaries: ~15MB × 2 platforms = 30MB per release
- Artifacts: 100MB free tier (sufficient for 3+ releases)

**Container Registry:**
- Container: ~20MB compressed
- GitHub Container Registry: Unlimited for public repos

**Total Cost:** Negligible (~$0-5/month)

---

## Success Metrics

### Technical Metrics
- ✅ CI pipeline success rate > 95%
- ✅ Artifact generation time < 15 minutes
- ✅ All formats generated successfully

### Adoption Metrics
- ✅ Binary downloads: 10+ in first month
- ✅ Container pulls: 5+ in first month
- ✅ GitHub stars increase: +5

### Quality Metrics
- ✅ Critical bugs: 0
- ✅ User satisfaction: 8/10+
- ✅ Time to resolution: < 24 hours

---

## Future Enhancements

### Potential Additions

1. **Multi-Version Support**
   - Generate MCP servers for last N versions
   - Enable version-specific queries

2. **Enhanced Metadata**
   - Include usage examples in MCP responses
   - Add deprecation warnings

3. **Analytics**
   - Track most-used commands
   - Identify common questions

4. **Integration Tests**
   - Automated Claude Code testing
   - Regression testing

---

## Questions for Clarification

Before implementing, please confirm:

1. **Repository Access:**
   - Do you have admin access to jerus-org/circleci-toolkit?
   - Can you configure CircleCI environment variables?

2. **Container Registry:**
   - Should we use GitHub Container Registry (ghcr.io)?
   - Or prefer Docker Hub or other registry?

3. **Release Frequency:**
   - How often are new orb versions released?
   - Should we support patch releases (v3.1.1)?

4. **Binary Platforms:**
   - Linux + macOS sufficient?
   - Need Windows binaries?

5. **Testing Access:**
   - Can we test with a pre-release tag first?
   - Preferred tag format for testing (e.g., v3.1.0-rc1)?

6. **Documentation:**
   - Should MCP docs be in main README or separate file?
   - Preferred announcement channel?

Please provide answers to proceed with implementation.
