# Build stage
FROM --platform=$BUILDPLATFORM tonistiigi/xx AS xx
FROM --platform=$BUILDPLATFORM rust:1.93.0-bookworm AS builder
WORKDIR /usr/src/litehook

# Copy xx scripts
COPY --from=xx / /

ARG TARGETPLATFORM
ARG BUILDPLATFORM
# xx should use musl
ENV XX_LIBC=musl
# fix sqlite linker error with musl,
# this is temporary, as this disables large file support
ENV CFLAGS="-DSQLITE_DISABLE_LFS"

# Install deps
RUN apt-get update && apt-get install -y \
    ca-certificates \
    llvm \
    clang \
    lld \
    && rm -rf /var/lib/apt/lists/*
RUN xx-apt-get install -y musl-dev musl-tools zlib1g-dev

RUN rustup target add $(xx-cargo --print-target-triple)

COPY . .

# Build
RUN xx-cargo build --release --target-dir ./build && \
    xx-verify ./build/$(xx-cargo --print-target-triple)/release/litehook && \
    llvm-strip ./build/$(xx-cargo --print-target-triple)/release/litehook && \
    cp ./build/$(xx-cargo --print-target-triple)/release/litehook /litehook-out

# Runtime stage
FROM cgr.dev/chainguard/static
COPY --from=builder /litehook-out /litehook
ENTRYPOINT ["/litehook"]
