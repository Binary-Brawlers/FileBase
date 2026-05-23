FROM rust:1.80-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p filebase-api

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/filebase-api /usr/local/bin/filebase-api
EXPOSE 8080
CMD ["filebase-api"]
