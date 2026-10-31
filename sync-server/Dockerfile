# Build the sync server image. The build context is the repository root, because the Cargo
# workspace needs every member manifest, and the shared-core crate is compiled into the server:
#
#   docker build -f sync-server/Dockerfile .

ARG RUST_VERSION=1
FROM rust:${RUST_VERSION}-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Only the server's dependencies are cooked. The desktop app's GTK and WebKit build dependencies
# are never needed here.
RUN cargo chef cook --release --locked -p sync-server --recipe-path recipe.json
COPY . .
RUN cargo build --release --locked -p sync-server

FROM debian:bookworm-slim AS runtime
# curl is only for the health check.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system app \
    && useradd --system --gid app --no-create-home --shell /usr/sbin/nologin app
COPY --from=builder /app/target/release/sync-server /usr/local/bin/sync-server

USER app
# Migrations are embedded in the binary and run at startup. DATABASE_URL and JWT_SECRET come from
# the environment; see docs/deploy-dokploy.md.
ENV BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=sync_server=info
EXPOSE 8080
# The check assumes the default port. If BIND_ADDR changes, change this too.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["/usr/local/bin/sync-server"]
