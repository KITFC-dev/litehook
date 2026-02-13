# Build stage
FROM rust:1.93.0-bookworm AS builder
WORKDIR /usr/src/litehook
COPY . .
RUN cargo build --release && strip target/release/litehook

# Runtime stage
FROM gcr.io/distroless/cc-debian12
COPY --from=builder /usr/src/litehook/target/release/litehook /litehook
ENTRYPOINT ["/litehook"]
