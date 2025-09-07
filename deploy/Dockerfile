# Multi-stage build optimized for CapRover deployment
# Stage 1: Builder
FROM rustlang/rust:nightly-bookworm AS builder

# Install build dependencies including additional tools for SAM
RUN apt-get update && apt-get install -y \
    build-essential \
    libssl-dev \
    pkg-config \
    libasound2-dev \
    libpq-dev \
    libsqlite3-dev \
    cmake \
    clang \
    llvm \
    libclang-dev \
    git \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Set environment variables for build optimization
ENV CARGO_NET_RETRY=10
ENV CARGO_IO_TIMEOUT=600
ENV RUST_BACKTRACE=1

# Create app directory
WORKDIR /usr/src/sam

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock build.rs ./

# Cache bust to force rebuild with Rust nightly for edition2024 support
RUN echo "Cache bust: 2025-09-06-145.41"

# Create dummy source structure for dependency compilation
RUN mkdir -p src/bin src/lib benches && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/installer.rs && \
    echo "pub mod sam;" > src/lib.rs && \
    echo "pub fn hello() {}" > src/sam.rs && \
    mkdir -p src/sam && \
    echo "pub fn placeholder() {}" > src/sam/mod.rs && \
    echo "pub fn placeholder() {}" > src/lib/mod.rs && \
    echo "fn main() {}" > benches/service_benchmarks.rs

# Build dependencies only
RUN cargo build --release && \
    rm -rf src target/release/deps/sam-* target/release/deps/installer-*

# Copy actual source code
COPY src ./src/
COPY benches ./benches/
COPY cfg ./cfg/
COPY data ./data/
COPY scripts ./scripts/
COPY packages ./packages/
COPY www ./www/

# Build the main application
RUN cargo build --release --bin sam

# Stage 2: Runtime optimized for CapRover
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libasound2 \
    libpq5 \
    libsqlite3-0 \
    sqlite3 \
    ffmpeg \
    redis-server \
    postgresql-client-15 \
    curl \
    wget \
    tzdata \
    dumb-init \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get autoremove -y \
    && apt-get clean

# Create non-root user for security
RUN useradd -m -u 1001 -s /bin/bash sam && \
    mkdir -p /app /var/log/sam /var/lib/sam /tmp/sam && \
    chown -R sam:sam /app /var/log/sam /var/lib/sam /tmp/sam

# Copy binary from builder
COPY --from=builder /usr/src/sam/target/release/sam /app/sam
RUN chmod +x /app/sam

# Copy configuration files and assets
COPY --from=builder /usr/src/sam/cfg /app/cfg
COPY --from=builder /usr/src/sam/data /app/data
COPY --from=builder /usr/src/sam/packages /app/packages
COPY --from=builder /usr/src/sam/www /app/www

# Copy and setup entrypoint script
COPY deploy/docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

# Set environment variables for CapRover
ENV SAM_HOME=/app \
    SAM_CONFIG=/app/cfg/config.json \
    SAM_DATA=/var/lib/sam \
    SAM_LOGS=/var/log/sam \
    RUST_LOG=info \
    RUST_BACKTRACE=1 \
    DATABASE_ENGINE=sqlite \
    SQLITE_DATABASE_PATH=/var/lib/sam/sam.db \
    PORT=8000

# Expose port (CapRover will map this to 80/443)
EXPOSE 8000

# Create volumes for persistent data
VOLUME ["/var/lib/sam", "/var/log/sam"]

# Switch to non-root user
USER sam

# Set working directory
WORKDIR /app

# Health check for CapRover
HEALTHCHECK --interval=30s --timeout=10s --start-period=120s --retries=3 \
    CMD curl -f http://localhost:8000/health/live || exit 1

# Use dumb-init as PID 1 to handle signals properly
ENTRYPOINT ["/usr/bin/dumb-init", "--"]

# Default command
CMD ["/app/docker-entrypoint.sh", "serve"]