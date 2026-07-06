FROM rust:1-bookworm AS builder

WORKDIR /build
COPY apps/server/Cargo.toml apps/server/Cargo.lock apps/server/
COPY apps/server/migrations apps/server/migrations
COPY apps/server/src apps/server/src
WORKDIR /build/apps/server
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN useradd --system --create-home --home-dir /home/focusnook focusnook \
  && apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

USER focusnook
WORKDIR /app
COPY --from=builder /build/apps/server/target/release/focusnook-sync-server /usr/local/bin/focusnook-sync-server

EXPOSE 8080
CMD ["focusnook-sync-server"]
