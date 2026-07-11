# syntax=docker/dockerfile:1.6

# Build stage — compile release binary dengan sqlx offline (.sqlx sudah di-commit)
FROM rust:1-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx

ENV SQLX_OFFLINE=true
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked && \
    cp target/release/kewarasan /usr/local/bin/kewarasan

# Runtime stage — debian slim, cukup ca-certificates + libssl3 (untuk teloxide/reqwest)
FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/kewarasan /usr/local/bin/kewarasan

ENV WEB_PORT=8775
EXPOSE 8775

CMD ["kewarasan"]
