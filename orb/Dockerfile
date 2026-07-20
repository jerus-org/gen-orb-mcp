FROM rust:1-slim-trixie@sha256:5c6f46a6e4472ab1ca7ba7d494e6677f2f219ebc02f32025d3986f057635ec9c AS builder
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
    && { n=0; until cargo install gen-orb-mcp --version "${CRATE_VERSION}" --locked; do \
         n=$((n+1)); \
         [ "$n" -ge 20 ] && echo "crates.io index never served ${CRATE_VERSION}" >&2 && exit 1; \
         echo "waiting for crates.io index to propagate ${CRATE_VERSION} (attempt $n)"; \
         sleep 15; \
       done; }

FROM rust:1-slim-trixie@sha256:5c6f46a6e4472ab1ca7ba7d494e6677f2f219ebc02f32025d3986f057635ec9c
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
