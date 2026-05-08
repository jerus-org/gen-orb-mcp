# CI Integration Guide

Integrate gen-orb-mcp into a CircleCI pipeline to run MCP server generation as part of your orb's build and release workflow.

## Overview

gen-orb-mcp is available as a **CircleCI orb** (`jerus-org/gen-orb-mcp`) that exposes each subcommand as a reusable job. The orb handles tool installation and execution; you choose how the outputs are built, stored, and distributed.

The orb is a set of building blocks, not a prescribed workflow. This guide shows how to assemble them.

## The gen-orb-mcp Orb

### Add to your config

```yaml
orbs:
  gen-orb-mcp: jerus-org/gen-orb-mcp@0.1
```

### Available jobs

| Job | Required parameters | Description |
|-----|---------------------|-------------|
| `generate` | `orb_path` | Generate MCP server source from an orb YAML |
| `validate` | `orb_path` | Validate an orb definition |
| `diff` | `current`, `previous`, `since_version` | Compute conformance rules between two orb versions |
| `migrate` | `orb`, `rules` | Apply migration rules to a consumer `.circleci/` directory |
| `prime` | — | Populate `prior-versions/` and `migrations/` from git history |

Each job runs inside the pre-built Docker image with gen-orb-mcp pre-installed — no manual installation required.

## Approach 1: Orb jobs as drop-in building blocks

The orb jobs produce output in the workspace. What you do with that output — compile it, archive it, upload it to a release, commit it back — is your choice.

### Regenerate on every build (validate only)

The simplest use is running `generate` on every build to confirm the generated orb source is consistent and passes `orb-tools/validate`:

```yaml
orbs:
  gen-orb-mcp: jerus-org/gen-orb-mcp@0.1
  orb-tools: circleci/orb-tools@12

workflows:
  build:
    jobs:
      - gen-orb-mcp/generate:
          orb_path: src/@orb.yml
          output: /tmp/mcp-build
          version: "0.0.0"
```

No build step, no upload — just confirm generation succeeds.

### Generate source and compile to binary

To compile the generated source to a binary, add a build step after `generate`:

```yaml
workflows:
  build:
    jobs:
      - gen-orb-mcp/generate:
          orb_path: src/@orb.yml
          output: /tmp/mcp-build
          version: "${CIRCLE_TAG#v}"

      - compile-mcp-server:
          requires: [gen-orb-mcp/generate]
          steps:
            - attach_workspace:
                at: /tmp/mcp-build
            - run:
                name: Compile MCP server
                no_output_timeout: 15m
                command: |
                  cd /tmp/mcp-build
                  cargo build --release
```

`no_output_timeout: 15m` prevents CircleCI from killing a silent Rust compilation.

### Use `prime` before `generate` to embed history

`prime` discovers prior orb versions from git tags and writes snapshot files. Pass those files to `generate`:

```yaml
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

When `ephemeral: true`, `prime` writes to `/tmp/gen-orb-mcp-prime-<pid>/` and prints the
`PRIME_PV_DIR` and `PRIME_MIG_DIR` env vars to `$BASH_ENV` for use in subsequent jobs.

## Approach 2: Runtime installation without the orb

If you prefer not to use the orb, install gen-orb-mcp at runtime:

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
          name: Generate MCP server source
          command: |
            gen-orb-mcp generate \
              --orb-path src/@orb.yml \
              --output /tmp/mcp-build \
              --version "${CIRCLE_TAG#v}"
```

**Trade-off**: Adds install time to every run. `cargo binstall` downloads a pre-built binary when available; falls back to source compilation (3-5 minutes) otherwise.

## Approach 3: Pre-installed in a custom container

Install gen-orb-mcp in your CI Docker image at build time:

```dockerfile
# renovate: datasource=crate depName=gen-orb-mcp versioning=semver-coerced
ENV GEN_ORB_MCP_VERSION=0.1.0
RUN cargo binstall gen-orb-mcp --version "${GEN_ORB_MCP_VERSION}" --no-confirm
```

The `renovate:` comment lets Renovate track the version automatically.

**Advantage:** no install overhead at runtime, exact version pinned in image.
**Trade-off:** container rebuild required for version updates.

## Storing and Distributing the Generated Binary

Where the binary ends up is entirely your choice. Common options:

### CircleCI artifacts (build-time inspection)

```yaml
- store_artifacts:
    path: /tmp/mcp-build/target/release/my_orb_mcp
    destination: mcp-server/my-orb-mcp-linux-x86_64
```

The binary is accessible in the CircleCI UI. Useful for inspection and manual download.

### GitHub release asset

Upload to the GitHub release using the Releases API. `curl` and `jq` are available in most CircleCI convenience images:

```bash
ASSET_PATH="/tmp/mcp-build/target/release/my_orb_mcp"
ASSET_NAME="my-orb-mcp-linux-x86_64"
REPO_SLUG="${CIRCLE_PROJECT_USERNAME}/${CIRCLE_PROJECT_REPONAME}"
TOKEN="${GITHUB_TOKEN}"

# Get the release ID for this tag
RELEASE_ID=$(curl -sf \
  -H "Authorization: Bearer ${TOKEN}" \
  "https://api.github.com/repos/${REPO_SLUG}/releases/tags/${CIRCLE_TAG}" \
  | jq -r '.id')

# Upload the asset
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/octet-stream" \
  "https://uploads.github.com/repos/${REPO_SLUG}/releases/${RELEASE_ID}/assets?name=${ASSET_NAME}" \
  --data-binary "@${ASSET_PATH}")

echo "Upload HTTP status: ${HTTP_CODE}"
[ "${HTTP_CODE}" = "201" ] || exit 1
```

This requires a CircleCI context providing `GITHUB_TOKEN` with `contents: write` permission. The GitHub release must exist before the upload runs.

### Committed back to the repository

If you want the generated source committed back (so it is version-controlled alongside the orb):

```yaml
- run:
    name: Commit generated MCP server source
    command: |
      git config user.email "ci@example.com"
      git config user.name "CI"
      git add mcp-server/
      git diff --cached --quiet || git commit -m "chore: regenerate MCP server source"
      git push origin main
```

This requires a deploy key or token with push access.

### Persist as workspace artifact for a downstream job

```yaml
- persist_to_workspace:
    root: /tmp
    paths:
      - mcp-build/target/release/my_orb_mcp
```

## Complete Example: Release workflow

This example generates the MCP server source and uploads a compiled binary to the GitHub release. It uses the orb for generation and standard `curl` for the upload — no `gh` CLI required.

```yaml
orbs:
  gen-orb-mcp: jerus-org/gen-orb-mcp@0.1

release-filters: &release-filters
  branches:
    ignore: /.*/
  tags:
    only: /^v[0-9]+\.[0-9]+\.[0-9]+$/

workflows:
  release:
    jobs:
      - gen-orb-mcp/prime:
          filters: *release-filters
          orb_path: src/@orb.yml
          earliest_version: "1.0.0"
          ephemeral: true

      - gen-orb-mcp/generate:
          filters: *release-filters
          requires: [gen-orb-mcp/prime]
          orb_path: src/@orb.yml
          output: /tmp/mcp-build
          version: "${CIRCLE_TAG#v}"
          migrations: /tmp/gen-orb-mcp-prime/migrations
          prior_versions: /tmp/gen-orb-mcp-prime/prior-versions

      - compile-and-upload:
          filters: *release-filters
          requires: [gen-orb-mcp/generate]
          context: github-release
          docker:
            - image: cimg/rust:1.85
          steps:
            - attach_workspace:
                at: /tmp/mcp-build
            - run:
                name: Compile
                no_output_timeout: 15m
                command: cd /tmp/mcp-build && cargo build --release
            - run:
                name: Upload to GitHub release
                command: |
                  ASSET_PATH="/tmp/mcp-build/target/release/my_orb_mcp"
                  ASSET_NAME="my-orb-mcp-linux-x86_64"
                  REPO_SLUG="${CIRCLE_PROJECT_USERNAME}/${CIRCLE_PROJECT_REPONAME}"
                  TOKEN="${GITHUB_TOKEN}"

                  RELEASE_ID=$(curl -sf \
                    -H "Authorization: Bearer ${TOKEN}" \
                    "https://api.github.com/repos/${REPO_SLUG}/releases/tags/${CIRCLE_TAG}" \
                    | jq -r '.id')

                  HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
                    -H "Authorization: Bearer ${TOKEN}" \
                    -H "Content-Type: application/octet-stream" \
                    "https://uploads.github.com/repos/${REPO_SLUG}/releases/${RELEASE_ID}/assets?name=${ASSET_NAME}" \
                    --data-binary "@${ASSET_PATH}")

                  echo "Upload HTTP status: ${HTTP_CODE}"
                  [ "${HTTP_CODE}" = "201" ] || exit 1
```

## Binary Naming Convention

The Rust compiler converts hyphens to underscores in binary names. For example, an orb named `my-orb` produces a binary named `my_orb_mcp`. Rename it before uploading if you want the hyphenated name:

```bash
mv /tmp/mcp-build/target/release/my_orb_mcp \
   /tmp/artifacts/my-orb-mcp-linux-x86_64
```

## Troubleshooting

### Binary compilation timeout

Rust compilation can take 3-5 minutes without producing output. Set `no_output_timeout: 15m` on any step that compiles.

### GitHub release not found

The upload command looks up the release by `CIRCLE_TAG`. The job must run on a tag-triggered pipeline, and the GitHub release must exist for that tag before the upload step runs.

### Upload fails with HTTP 401 or 403

The `GITHUB_TOKEN` must have `contents: write` permission on the repository. Verify that the correct CircleCI context is attached to the job and that the token has the required scope.

### prime writes to a temporary directory that is not shared

When using `ephemeral: true`, the path written to `$BASH_ENV` by `prime` is only available within the same job. Pass the path explicitly as `migrations` and `prior_versions` parameters to `generate` or use workspace persistence between jobs.
