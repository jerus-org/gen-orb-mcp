FROM debian:12-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -ms /bin/bash circleci
COPY gen-orb-mcp /usr/local/bin/gen-orb-mcp
RUN chmod +x /usr/local/bin/gen-orb-mcp
USER circleci
WORKDIR /home/circleci/project
