# Build stage
FROM rust:1.93.0-bookworm AS builder
WORKDIR /usr/src/litehook
RUN apt-get update && apt-get install -y musl-tools
RUN rustup target add x86_64-unknown-linux-musl
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN strip target/x86_64-unknown-linux-musl/release/litehook

# Runtime stage
FROM cgr.dev/chainguard/static
COPY --from=builder /usr/src/litehook/target/x86_64-unknown-linux-musl/release/litehook /litehook
ENTRYPOINT ["/litehook"]
