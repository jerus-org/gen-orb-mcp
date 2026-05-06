FROM rust:1-slim-bookworm AS builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install gen-orb-mcp

FROM debian:12-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -ms /bin/bash circleci
COPY --from=builder /usr/local/cargo/bin/gen-orb-mcp /usr/local/bin/gen-orb-mcp
USER circleci
WORKDIR /home/circleci/project
