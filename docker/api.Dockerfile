FROM rust:1.80-slim AS chef
WORKDIR /app
RUN cargo install cargo-chef --locked --version 0.1.68
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libssl-dev \
 && rm -rf /var/lib/apt/lists/*

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Cached dependency build - only rebuilds when Cargo.toml/Cargo.lock change
RUN cargo chef cook --release --recipe-path recipe.json -p filebase-api
COPY . .
RUN cargo build --release -p filebase-api

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl libssl3 \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 1001 --user-group filebase
COPY --from=builder /app/target/release/filebase-api /usr/local/bin/filebase-api

USER filebase
ENV BIND_ADDRESS=0.0.0.0:8080
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/health/live || exit 1

CMD ["filebase-api"]
