# ParityClaw Sandbox for NVIDIA OpenShell
#
# BYOC (Bring Your Own Container) image that bundles:
#   - ParityClaw (OpenCrabs fork) v0.2.83
#   - guava-anti MCP server v5.1
#   - Eternal Memory (SQLite + Vector)
#
# Usage:
#   docker build -t parityclaw-sandbox .
#   openshell sandbox create --from ./parityclaw-sandbox -- parityclaw

# --- Stage 1: Build ParityClaw ---
FROM rust:1.84-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libsqlite3-dev \
    cmake git curl build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Clone and build ParityClaw
RUN git clone --depth 1 https://github.com/koatora20/parityclaw.git . \
    && cargo build --release \
    && strip target/release/opencrabs

# --- Stage 2: Build guava-anti ---
FROM rust:1.84-slim-bookworm AS guava-builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libsqlite3-dev \
    cmake git curl build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Build guava-anti from source
# TODO: Replace with actual source path or registry
COPY guava-anti-src/ .
RUN cargo build --release \
    && strip target/release/guava-anti

# --- Stage 3: Runtime ---
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 libsqlite3-0 \
    curl git jq \
    && rm -rf /var/lib/apt/lists/*

# Create sandbox user (matches OpenShell process policy)
RUN groupadd -g 1000 sandbox \
    && useradd -u 1000 -g sandbox -m -s /bin/bash sandbox

# Copy binaries
COPY --from=builder /build/target/release/opencrabs /usr/local/bin/parityclaw
COPY --from=guava-builder /build/target/release/guava-anti /usr/local/bin/guava-anti

# Create workspace structure
RUN mkdir -p /sandbox/.opencrabs/memory \
    && mkdir -p /sandbox/.openclaw/workspace/memory \
    && mkdir -p /sandbox/.openclaw/workspace/skills \
    && chown -R sandbox:sandbox /sandbox

# Default OpenCrabs config
COPY openclaw.json /sandbox/.opencrabs/openclaw.json

# GuavaBridge config
ENV GUAVA_WORKSPACE=/sandbox/.openclaw/workspace
ENV OPENCRABS_HOME=/sandbox/.opencrabs

USER sandbox
WORKDIR /sandbox

# Health check
HEALTHCHECK --interval=30s --timeout=3s \
    CMD guava-anti --version && parityclaw --version || exit 1

ENTRYPOINT ["parityclaw"]
CMD ["tui"]
