FROM rust:slim@sha256:e14e87345b4d5964ddcc3491d27ee046a0f23820f340c3c1e24da6880141f7c0
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git gnupg openssh-client \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -ms /bin/bash circleci
COPY gen-orb-mcp /usr/local/bin/gen-orb-mcp
RUN chmod +x /usr/local/bin/gen-orb-mcp
USER circleci
WORKDIR /home/circleci/project
