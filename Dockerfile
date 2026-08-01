FROM rust:1.97.1-bookworm@sha256:77fac8b98f9f46062bb680b6d25d5bcaabfc400143952ebc572e924bcbedc3fa AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --locked

FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818

RUN apt-get update \
    && apt-get install --no-install-recommends -y ca-certificates socat \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 kopalace

COPY --from=builder /app/target/release/k-o-palace /usr/local/bin/k-o-palace

USER kopalace
EXPOSE 3001
ENV PALACE_BIND=127.0.0.1:3001

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/k-o-palace", "healthcheck"]

ENTRYPOINT ["/usr/local/bin/k-o-palace"]