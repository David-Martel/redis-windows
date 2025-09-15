# Multi-stage Dockerfile for agent-sync Rust project
# Supports Linux builds with static linking and multi-arch support

# Build stage - Use official Rust image with MUSL support
FROM rust:1.75-bullseye as builder

# Install build dependencies for cross-compilation
RUN apt-get update && apt-get install -y \
    musl-tools \
    musl-dev \
    gcc-musl \
    pkg-config \
    libssl-dev \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

# Add required Rust targets
RUN rustup target add x86_64-unknown-linux-musl
RUN rustup target add x86_64-unknown-linux-gnu
RUN rustup target add aarch64-unknown-linux-gnu

# Set up cross-compilation environment
ENV CC_x86_64_unknown_linux_musl=musl-gcc
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc
ENV PKG_CONFIG_ALLOW_CROSS=1

# Create app directory
WORKDIR /usr/src/agent-sync

# Copy configuration files first for better layer caching
COPY .cargo/ ./.cargo/
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies (this will be cached unless Cargo.toml changes)
ARG TARGET=x86_64-unknown-linux-musl
RUN cargo build --release --target=$TARGET
RUN rm src/main.rs

# Copy source code
COPY src/ ./src/
COPY benches/ ./benches/

# Build the actual application
RUN cargo build --release --target=$TARGET

# Create final image with minimal footprint
FROM scratch as runtime

# Copy CA certificates for TLS support
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Set build argument for target architecture
ARG TARGET=x86_64-unknown-linux-musl

# Copy the binary from builder stage
COPY --from=builder /usr/src/agent-sync/target/$TARGET/release/libagent_sync.so /usr/local/lib/ 2>/dev/null || true
COPY --from=builder /usr/src/agent-sync/target/$TARGET/release/libagent_sync.a /usr/local/lib/ 2>/dev/null || true

# Alternative runtime image for development/debugging
FROM debian:bullseye-slim as runtime-debug

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

ARG TARGET=x86_64-unknown-linux-gnu
COPY --from=builder /usr/src/agent-sync/target/$TARGET/release/libagent_sync.so /usr/local/lib/ 2>/dev/null || true
COPY --from=builder /usr/src/agent-sync/target/$TARGET/release/libagent_sync.a /usr/local/lib/ 2>/dev/null || true

# Set library path
ENV LD_LIBRARY_PATH=/usr/local/lib

# Test stage for running tests in Docker
FROM builder as test

# Copy test files and run tests
COPY tests/ ./tests/ 2>/dev/null || true
RUN cargo test --release --target=$TARGET

# Benchmark stage
FROM builder as bench

RUN cargo bench --target=$TARGET

# Multi-arch build support
FROM builder as build-aarch64
RUN rustup target add aarch64-unknown-linux-gnu
RUN apt-get update && apt-get install -y gcc-aarch64-linux-gnu
ENV CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
RUN cargo build --release --target=aarch64-unknown-linux-gnu

# Default to minimal runtime
FROM runtime

# Metadata
LABEL org.opencontainers.image.title="agent-sync"
LABEL org.opencontainers.image.description="High-performance Redis agent synchronization library"
LABEL org.opencontainers.image.version="0.1.0"
LABEL org.opencontainers.image.source="https://github.com/anthropic/redis-windows"
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"

# Health check (if applicable)
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["true"] # Replace with actual health check if needed

# Default command (for library, this might not be applicable)
CMD ["true"]