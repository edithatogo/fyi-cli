# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release --locked --package fyi-mcp

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/fyi-mcp /usr/local/bin/fyi-mcp

ENV FYI_MCP_EPHEMERAL=1
ENV DATABASE_URL=sqlite::memory:

ENTRYPOINT ["/usr/local/bin/fyi-mcp"]
