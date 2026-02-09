# CI Integration Guide

Integrate gen-orb-mcp into a CI/CD pipeline to generate MCP server binaries on each orb release.

## Overview

gen-orb-mcp generates a companion MCP server binary when a CircleCI orb is released and attaches it to the GitHub release. AI coding assistants can then download the binary to access orb documentation and tooling.

Three components are involved:

1. **CI container** - Docker image with gen-orb-mcp pre-installed
2. **CI pipeline** - Job that generates the binary and uploads it to the release
3. **Release assets** - Binary attached to the GitHub release for download

## CI Tool Installation

Two approaches to making gen-orb-mcp available in CI:

### Pre-installed in container (recommended)

Install gen-orb-mcp in the CI Docker image at build time. This avoids tool installation overhead on every run.

```dockerfile
ENV GEN_ORB_MCP_VERSION=0.1.0

RUN cargo binstall gen-orb-mcp --version "${GEN_ORB_MCP_VERSION}" --no-confirm
```

**Advantages:**
- No install overhead at CI runtime
- Reproducible builds with exact version pinned in image

**Trade-off:**
- Container rebuild required for version updates

`cargo binstall` downloads pre-built binaries when available and falls back to compiling from source otherwise. Either way, the tool is baked into the image and available at CI runtime.

**Tip:** Dependency update tools can automate version bumps. For example, [Renovate](https://docs.renovatebot.com/) detects and updates crate versions with a comment above the `ENV` line:

```dockerfile
# renovate: datasource=crate depName=gen-orb-mcp packageName=gen-orb-mcp versioning=semver-coerced
ENV GEN_ORB_MCP_VERSION=0.1.0
```

### Runtime installation

Install gen-orb-mcp as a CI step. Simpler to set up but adds time to every run.

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
- Source compilation can take several minutes if no pre-built binary is available

### Future: Public CircleCI orb

A public CircleCI orb providing gen-orb-mcp as a reusable job is planned. This would let any orb project add MCP server generation by using the orb directly, without managing tool installation.

## CircleCI Pipeline Configuration

### Reusable Commands

#### generate_mcp_server

Wraps `gen-orb-mcp generate` to produce an MCP server from an orb definition.

```yaml
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
    orb_name:
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
            --name "<< parameters.orb_name >>" \
            --version "<< parameters.version >>"
```

Set `no_output_timeout: 15m` because binary compilation can take several minutes without producing output.

#### upload_release_asset

Uploads a file to the GitHub release matching the current tag. Requires `GITHUB_TOKEN` in the environment.

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

          REPO_SLUG="${CIRCLE_PROJECT_USERNAME}/${CIRCLE_PROJECT_REPONAME}"

          RELEASE_RESPONSE=$(curl -s -w "\n%{http_code}" \
            -H "Authorization: Bearer ${TOKEN}" \
            "https://api.github.com/repos/${REPO_SLUG}/releases/tags/${CIRCLE_TAG}")

          HTTP_CODE=$(echo "${RELEASE_RESPONSE}" | tail -1)
          RELEASE_BODY=$(echo "${RELEASE_RESPONSE}" | sed '$d')

          if [[ "${HTTP_CODE}" != "200" ]]; then
            echo "ERROR: GitHub API returned HTTP ${HTTP_CODE}" >&2
            echo "${RELEASE_BODY}" | jq -r '.message // .' >&2
            exit 1
          fi

          RELEASE_ID=$(echo "${RELEASE_BODY}" | jq -r '.id')

          curl -s -w "\n%{http_code}" -X POST \
            -H "Authorization: Bearer ${TOKEN}" \
            -H "Content-Type: application/octet-stream" \
            "https://uploads.github.com/repos/${REPO_SLUG}/releases/${RELEASE_ID}/assets?name=${ASSET_NAME}" \
            --data-binary "@${ASSET_PATH}"
```

Uses `curl` and `jq` to interact with the GitHub Releases API. Capture HTTP status codes explicitly rather than using `curl -sf`, which suppresses error details and makes failures difficult to diagnose.

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
          orb_name: my-orb
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

This follows the standard CircleCI orb template pattern used by `circleci-toolkit` and `circleci/orb-tools`: the `test-deploy.yml` pipeline runs tests on all branches and tags, but release jobs run only on semver tags.

The MCP generation job should:

- **Run only on release tags** - Filter to semver tags (e.g., `v1.2.3`)
- **Require all test jobs** - Do not generate from untested code
- **Run in parallel with publishing** - Independent of the orb publish step
- **Not block publishing** - Orb release succeeds even if MCP generation fails

```yaml
release-filters: &release-filters
  branches:
    ignore: /.*/
  tags:
    only: /^v[0-9]+\.[0-9]+\.[0-9]+$/

workflows:
  test-deploy:
    jobs:
      # ... test jobs run on all branches and tags ...

      - generate-mcp-server:
          filters: *release-filters
          requires:
            - all-test-jobs
          context:
            - your-github-context  # must provide GITHUB_TOKEN with write access

      - orb-tools/publish:
          # Does NOT require generate-mcp-server
          filters: *release-filters
          requires:
            - orb-tools/pack
            - all-test-jobs
          context:
            - orb-publishing
```

## Binary Naming Convention

Generated binaries use platform-suffixed names:

```
<orb-name>-mcp-<platform>-<arch>
```

Example: `circleci-toolkit-mcp-linux-x86_64`

Currently only Linux x86_64 is supported (the standard CircleCI executor architecture).

## Prerequisites

- **GitHub token** - A `GITHUB_TOKEN` with write access to the GitHub release, provided via a CircleCI context. The upload step creates release assets, which requires `contents: write` permission.
- **GitHub release** - Must exist before the upload step runs
- **jq** - Required for parsing GitHub API responses in the upload command
- **gen-orb-mcp** - Must be available in the executor

## Troubleshooting

### Binary compilation timeout

The generated MCP server compiles a Rust project (3-5 minutes). Set `no_output_timeout: 15m` on the generation step to prevent CircleCI from killing the job.

### GitHub release not found

The upload command looks up the release by `CIRCLE_TAG`. Check that:
- The job runs on a tag-triggered pipeline
- The GitHub release exists for that tag
- `CIRCLE_TAG` is set (not available on branch builds)

### Upload fails with HTTP 401 or empty token

The upload step requires a `GITHUB_TOKEN` with write access to the release. Verify that:
- The CircleCI context attached to the job provides `GITHUB_TOKEN`
- The token has `contents: write` permission on the repository
- The correct context name is listed in the workflow entry

If the token is missing or lacks write access, the GitHub API returns `401 Unauthorized` or `403 Forbidden`. The upload command logs the HTTP status code and error message to help diagnose the issue.

### cargo binstall falls back to source compilation

This means pre-built binaries are not yet published for that version of gen-orb-mcp. This only affects Docker image build time, not CI pipeline execution (the tool is pre-installed in the image).

### Binary name mismatch

The generated binary name converts hyphens to underscores (Rust convention). For example, `circleci-toolkit` produces `circleci_toolkit_mcp`. The rename to the platform-suffixed name happens in the "Prepare artifact" step.
