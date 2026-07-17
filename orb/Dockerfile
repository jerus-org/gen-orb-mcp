FROM rust:1-slim-trixie@sha256:34fb2f168c432d421a09883c663b275b33cbb30f6b18642fbd09a684c6546d0e AS builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential ca-certificates clang cmake libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install gen-orb-mcp --locked

FROM rust:1-slim-trixie@sha256:34fb2f168c432d421a09883c663b275b33cbb30f6b18642fbd09a684c6546d0e
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential ca-certificates clang cmake git gnupg libssl-dev openssh-client pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -ms /bin/bash circleci
COPY --from=builder /usr/local/cargo/bin/gen-orb-mcp /usr/local/bin/gen-orb-mcp
USER circleci
WORKDIR /home/circleci/project
