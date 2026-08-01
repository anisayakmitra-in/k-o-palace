FROM rust:1.85-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --locked

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install --no-install-recommends -y ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 kopalace

COPY --from=builder /app/target/release/k-o-palace /usr/local/bin/k-o-palace

USER kopalace
EXPOSE 3001
ENV PALACE_BIND=0.0.0.0:3001

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD /usr/bin/curl --fail http://127.0.0.1:3001/health || exit 1

ENTRYPOINT ["/usr/local/bin/k-o-palace"]