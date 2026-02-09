# CI Integration Guide

This guide describes how to integrate gen-orb-mcp into a CI/CD pipeline to automatically generate MCP server binaries when new orb versions are released.

## Overview

When a CircleCI orb is released, gen-orb-mcp can generate a companion MCP server binary and attach it to the release. This enables AI coding assistants to automatically access up-to-date orb documentation and tooling.

The integration involves three components:

1. **CI container** - A Docker image with gen-orb-mcp pre-installed
2. **CI pipeline** - A job that generates the binary and uploads it to the release
3. **Release assets** - The binary attached to the GitHub release for download

## CI Tool Installation Strategies

There are two main approaches to making gen-orb-mcp available in CI:

### Pre-installed in container (recommended)

Install gen-orb-mcp in your CI Docker image at build time. This avoids spending CI minutes compiling or downloading tools on every run.

```dockerfile
# In your CI Dockerfile
# renovate: datasource=crate depName=gen-orb-mcp packageName=gen-orb-mcp versioning=semver-coerced
ENV GEN_ORB_MCP_VERSION=0.1.0

RUN cargo binstall gen-orb-mcp --version "${GEN_ORB_MCP_VERSION}" --no-confirm
```

**Advantages:**
- Fastest CI execution - no tool install overhead
- Reproducible builds - exact version pinned in image
- Works with dependency update bots (e.g., Renovate) via the `renovate:` comment

**Trade-off:**
- Container rebuild required for version updates
- Without pre-built binaries on GitHub releases, `cargo binstall` will fall back to compiling from source during the Docker build (slower image builds, but this only happens once per version)

### Runtime installation

Install gen-orb-mcp as a CI step. Simpler setup but adds time to every pipeline run.

```yaml
steps:
  - run:
      name: Install gen-orb-mcp
      command: cargo binstall gen-orb-mcp --no-confirm
```

**Advantages:**
- No custom Docker image required
- Always gets the latest version

**Trade-off:**
- Adds install time to every CI run
- Rust compilation from source can take several minutes if no pre-built binary is available

## CircleCI Pipeline Configuration

### Reusable Commands

#### generate_mcp_server

Generates an MCP server from an orb definition. Wraps the `gen-orb-mcp generate` CLI command.

```yaml
# In your orb or config commands section
generate_mcp_server:
  description: >
    Generate an MCP server binary from a CircleCI orb definition.
  parameters:
    orb_path:
      type: string
      default: "src/@orb.yml"
    output_dir:
      type: string
      default: "/tmp/mcp-build"
    name:
      type: string
      description: "Name of the orb"
    version:
      type: string
      description: "Version of the orb"
    format:
      type: enum
      enum: ["binary", "source"]
      default: "binary"
  steps:
    - run:
        name: Generate MCP server (<< parameters.format >>)
        no_output_timeout: 15m
        command: |
          gen-orb-mcp generate \
            --orb-path "<< parameters.orb_path >>" \
            --output "<< parameters.output_dir >>" \
            --format "<< parameters.format >>" \
            --name "<< parameters.name >>" \
            --version "<< parameters.version >>"
```

The `no_output_timeout: 15m` is important because the binary compilation step can take several minutes without producing output.

#### upload_release_asset

Uploads a binary file to the GitHub release matching the current tag. Requires `GITHUB_TOKEN` in the environment.

```yaml
upload_release_asset:
  description: >
    Upload a binary asset to a GitHub release.
  parameters:
    asset_path:
      type: string
      description: "Path to the asset file to upload"
    asset_name:
      type: string
      default: ""
      description: "Name for the asset (defaults to filename)"
    github_token_var:
      type: env_var_name
      default: GITHUB_TOKEN
  steps:
    - run:
        name: Upload asset to GitHub release
        command: |
          ASSET_PATH="<< parameters.asset_path >>"
          ASSET_NAME="<< parameters.asset_name >>"
          TOKEN="${<< parameters.github_token_var >>}"

          if [[ -z "${ASSET_NAME}" ]]; then
            ASSET_NAME="$(basename "${ASSET_PATH}")"
          fi

          RELEASE_ID=$(curl -sf \
            -H "Authorization: Bearer ${TOKEN}" \
            "https://api.github.com/repos/${CIRCLE_PROJECT_USERNAME}/${CIRCLE_PROJECT_REPONAME}/releases/tags/${CIRCLE_TAG}" \
            | jq -r '.id')

          curl -sf -X POST \
            -H "Authorization: Bearer ${TOKEN}" \
            -H "Content-Type: application/octet-stream" \
            "https://uploads.github.com/repos/${CIRCLE_PROJECT_USERNAME}/${CIRCLE_PROJECT_REPONAME}/releases/${RELEASE_ID}/assets?name=${ASSET_NAME}" \
            --data-binary "@${ASSET_PATH}"
```

This command uses `curl` and `jq` (both commonly available in CI images) to interact with the GitHub Releases API. It could be replaced by a dedicated CLI tool in the future.

### Complete Job Example

```yaml
jobs:
  generate-mcp-server:
    executor: your-rust-executor
    steps:
      - checkout
      - run:
          name: Extract version from tag
          command: |
            ORB_VERSION="${CIRCLE_TAG#v}"
            echo "export ORB_VERSION=${ORB_VERSION}" >> "$BASH_ENV"
      - generate_mcp_server:
          name: my-orb
          version: "${ORB_VERSION}"
      - run:
          name: Prepare artifact
          command: |
            mkdir -p /tmp/artifacts
            cp /tmp/mcp-build/target/release/my_orb_mcp \
              /tmp/artifacts/my-orb-mcp-linux-x86_64
      - store_artifacts:
          path: /tmp/artifacts
          destination: mcp-server
      - upload_release_asset:
          asset_path: /tmp/artifacts/my-orb-mcp-linux-x86_64
```

### Workflow Integration

The MCP server generation job should:

- **Run only on release tags** - Use a filter like `tags: only: /^v[0-9]+\.[0-9]+\.[0-9]+$/`
- **Require all test jobs** - Don't generate from untested code
- **Run in parallel with publishing** - It does not need to block or depend on the publish step
- **Not block publishing** - If MCP generation fails, the orb release should still succeed

```yaml
workflows:
  release:
    jobs:
      # ... test jobs ...

      - generate-mcp-server:
          filters:
            branches:
              ignore: /.*/
            tags:
              only: /^v[0-9]+\.[0-9]+\.[0-9]+$/
          requires:
            - all-test-jobs
          context:
            - release  # provides GITHUB_TOKEN

      - publish:
          # Does NOT require generate-mcp-server
          requires:
            - all-test-jobs
          context:
            - publishing
```

## Binary Naming Convention

Generated binaries use platform-suffixed names for clarity:

```
<orb-name>-mcp-<platform>-<arch>
```

Example: `circleci-toolkit-mcp-linux-x86_64`

Currently only Linux x86_64 is supported (the standard CircleCI executor architecture).

## Prerequisites

- **GitHub token** - A token with `contents: write` permission on the repository, provided via a CircleCI context
- **GitHub release** - Must exist before the upload step runs. If your pipeline creates the release in a separate stage, ensure it completes first
- **jq** - Required for parsing GitHub API responses in the upload command
- **gen-orb-mcp** - Must be available in the executor (pre-installed or runtime-installed)

## Troubleshooting

### Binary compilation timeout

The generated MCP server compiles a Rust project, which can take 3-5 minutes. Set `no_output_timeout: 15m` on the generation step to prevent CircleCI from killing the job.

### GitHub release not found

The upload command looks up the release by `CIRCLE_TAG`. Ensure:
- The job runs on a tag-triggered pipeline
- The GitHub release has already been created for that tag
- `CIRCLE_TAG` is set (it won't be on branch builds)

### cargo binstall falls back to source compilation

If `cargo binstall gen-orb-mcp` compiles from source instead of downloading a binary, it means pre-built binaries are not yet available for gen-orb-mcp. This is expected for now and only affects Docker image build time, not CI pipeline execution time (since the tool is pre-installed in the image).

### Binary name mismatch

The generated binary name is derived from the orb name with hyphens converted to underscores (Rust convention). For example, `circleci-toolkit` produces `circleci_toolkit_mcp`. The rename to the platform-suffixed name happens in the "Prepare artifact" step.
