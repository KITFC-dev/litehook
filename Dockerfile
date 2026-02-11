FROM rust:1.93.0-bookworm AS builder
WORKDIR /usr/src/litehook
COPY . .
RUN cargo install --path .

# Build final image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/litehook /usr/local/bin/litehook
WORKDIR /app

CMD ["litehook"]
