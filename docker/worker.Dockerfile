FROM rust:1.80-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p filebase-worker

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/filebase-worker /usr/local/bin/filebase-worker
CMD ["filebase-worker"]
