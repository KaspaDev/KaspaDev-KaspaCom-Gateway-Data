FROM rust:slim-bookworm AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/krcbot-kaspacom-gatewayapi /app/gatewayapi
COPY config.yaml /app/config.yaml

# Create non-root user for security
RUN groupadd -r krcbot && useradd -r -g krcbot -d /app -s /sbin/nologin krcbot && \
    chown -R krcbot:krcbot /app

USER krcbot

CMD ["./gatewayapi"]
