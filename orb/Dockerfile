FROM rust:1-slim-trixie@sha256:4ddd2ebb0043566905e1e5e82ba7c66c03444b32ef16caa6bf5a20155d6ffb15 AS builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install gen-orb-mcp

FROM rust:1-slim-trixie@sha256:4ddd2ebb0043566905e1e5e82ba7c66c03444b32ef16caa6bf5a20155d6ffb15
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git gnupg libssl-dev openssh-client pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -ms /bin/bash circleci
COPY --from=builder /usr/local/cargo/bin/gen-orb-mcp /usr/local/bin/gen-orb-mcp
USER circleci
WORKDIR /home/circleci/project
