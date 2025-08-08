# Multi-stage build for optimized image size
# Stage 1: Builder
FROM rust:1.75-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
    libasound2-dev \
    libpq-dev \
    cmake \
    clang \
    llvm \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/sam

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs for dependency compilation
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/sam.rs

# Build dependencies
RUN cargo build --release && \
    rm -rf src

# Copy actual source code
COPY . .

# Touch main.rs to ensure it's newer than the cached deps
RUN touch src/main.rs src/sam.rs

# Build the application
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libasound2 \
    libpq5 \
    ffmpeg \
    redis-server \
    postgresql-client \
    curl \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash sam && \
    mkdir -p /opt/sam /var/log/sam /var/lib/sam && \
    chown -R sam:sam /opt/sam /var/log/sam /var/lib/sam

# Copy binary from builder
COPY --from=builder /usr/src/sam/target/release/sam /opt/sam/sam

# Copy configuration files
COPY --from=builder /usr/src/sam/cfg /opt/sam/cfg
COPY --from=builder /usr/src/sam/data /opt/sam/data
COPY --from=builder /usr/src/sam/packages /opt/sam/packages

# Copy entrypoint script
COPY docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

# Set environment variables
ENV SAM_HOME=/opt/sam \
    SAM_CONFIG=/opt/sam/config.json \
    SAM_DATA=/var/lib/sam \
    SAM_LOGS=/var/log/sam \
    RUST_LOG=info \
    RUST_BACKTRACE=1

# Expose ports
EXPOSE 8000 8080 8443

# Volume for persistent data
VOLUME ["/var/lib/sam", "/var/log/sam", "/opt/sam/config"]

# Switch to non-root user
USER sam

# Set working directory
WORKDIR /opt/sam

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Entrypoint
ENTRYPOINT ["/docker-entrypoint.sh"]

# Default command
CMD ["serve"]