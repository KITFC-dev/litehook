# Build stage
FROM --platform=$BUILDPLATFORM rust:1.93.0-bookworm AS builder
WORKDIR /usr/src/litehook

ARG TARGETPLATFORM
ARG BUILDPLATFORM

RUN apt-get update && apt-get install -y \
    xz-utils \
    ca-certificates \
    llvm \
    && rm -rf /var/lib/apt/lists/*

# Install zig
RUN curl -fsSL https://ziglang.org/download/0.13.0/zig-linux-x86_64-0.13.0.tar.xz \
    | tar -xJ -C /usr/local && \
    ln -s /usr/local/zig-linux-x86_64-0.13.0/zig /usr/local/bin/zig
RUN cargo install cargo-zigbuild

RUN rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl

COPY . .

# Build
RUN case "$TARGETPLATFORM" in \
    # ARM
    "linux/arm64") TARGET="aarch64-unknown-linux-musl" ;; \
    # Anything else
    *) TARGET="x86_64-unknown-linux-musl" ;; \
    esac && \
    cargo zigbuild --release --target "$TARGET" && \
    llvm-strip "target/$TARGET/release/litehook" && \
    cp "target/$TARGET/release/litehook" /litehook-out

# Runtime stage
FROM cgr.dev/chainguard/static
COPY --from=builder /litehook-out /litehook
ENTRYPOINT ["/litehook"]
