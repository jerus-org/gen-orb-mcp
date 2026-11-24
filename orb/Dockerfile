FROM rust:1-slim-trixie@sha256:1a939748a7cbab172de91a014e3804a97af1394a8c844faa67fce9401c4e40fb AS builder
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
    && { err=/tmp/cargo-install.err; n=0; s=15; \
       until cargo install gen-orb-mcp --locked \
             --version "${CRATE_VERSION}" 2>"$err"; do \
         cat "$err" >&2; \
         grep -q "failed to compile" "$err" \
           && echo "build failed, not an index delay" >&2 && exit 1; \
         n=$((n+1)); \
         if [ "$n" -ge 16 ]; then \
           echo "crates.io index never served ${CRATE_VERSION} after 16 tries (~57m)" >&2; \
           echo "polled: https://index.crates.io/ge/n-/gen-orb-mcp" >&2; \
           echo "The crate is published, but this container was not built" >&2; \
           echo "and the orb was not published." >&2; \
           echo "Once the index serves it, re-run the tag's workflow from the failed job." >&2; \
           echo "The wait is capped at an hour in total; the waits are set by [orb] crate_wait_*." >&2; \
           exit 1; \
         fi; \
         j=$(od -An -N1 -tu1 /dev/urandom 2>/dev/null); \
         w=$((s + ${j:-0} % (s / 5 + 1))); \
         echo "waiting for crates.io index: ${CRATE_VERSION} (try $n, next in ${w}s)"; \
         rm -rf "${CARGO_HOME:-/usr/local/cargo}"/registry/index/*/.cache; \
         sleep "$w"; \
         s=$((s * 2)); [ "$s" -le 300 ] || s=300; \
       done; }

FROM rust:1-slim-trixie@sha256:1a939748a7cbab172de91a014e3804a97af1394a8c844faa67fce9401c4e40fb
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
