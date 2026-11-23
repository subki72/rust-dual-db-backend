# Stage 1: Rust Release Builder
FROM rust:1.100-slim AS builder

WORKDIR /build

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY migrations/ ./migrations/

RUN cargo build --release --locked --bin braincode-be --bin ingest_csv

# Stage 2: Hardened Non-Root Runtime
FROM debian:bookworm-slim AS runtime

ENV RUST_LOG="braincode_be=info,tower_http=info" \
    PORT="3000"

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 appgroup \
    && useradd --uid 10001 --gid appgroup --shell /sbin/nologin --create-home appuser

WORKDIR /app

COPY --from=builder --chown=appuser:appgroup /build/target/release/braincode-be /app/braincode-be
COPY --from=builder --chown=appuser:appgroup /build/target/release/ingest_csv /app/ingest_csv
COPY --chown=appuser:appgroup migrations/ /app/migrations/

USER appuser

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["curl", "-fsS", "http://127.0.0.1:3000/health"]

CMD ["/app/braincode-be"]
