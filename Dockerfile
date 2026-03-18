# Build stage
FROM --platform=$BUILDPLATFORM tonistiigi/xx AS xx
FROM --platform=$BUILDPLATFORM rust:1.93.0-bookworm AS builder
WORKDIR /usr/src/litehook

# Copy xx scripts
COPY --from=xx / /

ARG TARGETPLATFORM
ARG BUILDPLATFORM

# Install deps
RUN apt-get update && apt-get install -y \
    ca-certificates \
    llvm \
    clang \
    lld \
    gcc-aarch64-linux-gnu \
    && rm -rf /var/lib/apt/lists/*
RUN xx-apt-get install -y libc6-dev gcc-12 libgcc-12-dev

RUN rustup target add $(xx-cargo --print-target-triple)

COPY . .

# Build
RUN xx-cargo build --release --target-dir ./build && \
    xx-verify ./build/$(xx-cargo --print-target-triple)/release/litehook && \
    llvm-strip ./build/$(xx-cargo --print-target-triple)/release/litehook && \
    cp ./build/$(xx-cargo --print-target-triple)/release/litehook /litehook-out

# Runtime stage
FROM cgr.dev/chainguard/wolfi-base
RUN apk add --no-cache libssl3 libstdc++ zlib
COPY --from=builder /litehook-out /litehook
ENTRYPOINT ["/litehook"]
