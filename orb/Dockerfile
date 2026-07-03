FROM rust:1-slim-trixie AS builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential ca-certificates clang cmake libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install gen-orb-mcp --locked

FROM rust:1-slim-trixie
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential ca-certificates clang cmake git gnupg libssl-dev openssh-client pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -ms /bin/bash circleci
COPY --from=builder /usr/local/cargo/bin/gen-orb-mcp /usr/local/bin/gen-orb-mcp
USER circleci
WORKDIR /home/circleci/project
