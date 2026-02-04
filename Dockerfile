# syntax=docker/dockerfile:1.7

############################
# 1) Build
############################
FROM rust:1-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Cache deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Code
COPY . .
RUN cargo build --release

############################
# 2) Runtime
############################
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libsqlite3-0 \
    wget \
    && rm -rf /var/lib/apt/lists/*

# user non-root
RUN useradd -m -u 10001 linx

COPY --from=builder /app/target/release/linx /usr/local/bin/linx
COPY --from=builder --chown=linx:linx /app/public /app/public

RUN mkdir -p /data && chown -R linx:linx /data

USER linx

ENV RUST_LOG=info
ENV DATABASE_URL=sqlite:///data/linx.db
ENV LINX_URL=http://127.0.0.1:3000

HEALTHCHECK --interval=10s --timeout=2s --start-period=10s --retries=3 \
    CMD wget -qO- http://127.0.0.1:3000/api/health >/dev/null || exit 1

EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/linx"]
