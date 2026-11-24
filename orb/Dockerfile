FROM rust:1-slim-trixie@sha256:6f14b2b4b278ec0d50aa1d69541c304ce0f95e0f6268487aa9924af1c2e9aebe AS builder
ARG CRATE_VERSION
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    clang \
    cmake \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && { err=/tmp/cargo-install.err; n=0; \
       until cargo install gen-orb-mcp --locked \
             --version "${CRATE_VERSION}" 2>"$err"; do \
         cat "$err" >&2; \
         grep -q "failed to compile" "$err" \
           && echo "build failed, not an index delay" >&2 && exit 1; \
         n=$((n+1)); \
         [ "$n" -ge 40 ] \
           && echo "crates.io index never served ${CRATE_VERSION}" >&2 \
           && exit 1; \
         echo "waiting for crates.io index: ${CRATE_VERSION} (try $n)"; \
         rm -rf "${CARGO_HOME:-/usr/local/cargo}"/registry/index/*/.cache; \
         sleep 15; \
       done; }

FROM rust:1-slim-trixie@sha256:6f14b2b4b278ec0d50aa1d69541c304ce0f95e0f6268487aa9924af1c2e9aebe
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    clang \
    cmake \
    git \
    gnupg \
    libssl-dev \
    openssh-client \
    pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -ms /bin/bash circleci
COPY --from=builder /usr/local/cargo/bin/gen-orb-mcp /usr/local/bin/gen-orb-mcp
USER circleci
WORKDIR /home/circleci/project
