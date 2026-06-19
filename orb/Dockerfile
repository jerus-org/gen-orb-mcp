FROM rust:1-slim-trixie@sha256:3b05f7c617a200c41c3506097f0d15fc193a1c93bfd8f141007b47cac8f95d3c AS builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install gen-orb-mcp

FROM rust:1-slim-trixie@sha256:3b05f7c617a200c41c3506097f0d15fc193a1c93bfd8f141007b47cac8f95d3c
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git gnupg libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -ms /bin/bash circleci
COPY --from=builder /usr/local/cargo/bin/gen-orb-mcp /usr/local/bin/gen-orb-mcp
USER circleci
WORKDIR /home/circleci/project
