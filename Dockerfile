FROM rust:1.85-bookworm@sha256:e51d0265072d2d9d5d320f6a44dde6b9ef13653b035098febd68cce8fa7c0bc4 AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --locked

FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818

RUN useradd --system --create-home --uid 10001 kopalace

COPY --from=builder /app/target/release/k-o-palace /usr/local/bin/k-o-palace

USER kopalace
EXPOSE 3001
ENV PALACE_BIND=0.0.0.0:3001

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/k-o-palace", "healthcheck"]

ENTRYPOINT ["/usr/local/bin/k-o-palace"]