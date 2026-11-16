# syntax=docker/dockerfile:1

FROM rust:1-bookworm@sha256:ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea AS builder

WORKDIR /app
COPY . .
RUN cargo build --release --locked --package fyi-mcp

FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/fyi-mcp /usr/local/bin/fyi-mcp

ENV FYI_MCP_EPHEMERAL=1
ENV DATABASE_URL=sqlite::memory:

ENTRYPOINT ["/usr/local/bin/fyi-mcp"]
