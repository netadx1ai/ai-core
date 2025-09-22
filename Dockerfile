# Multi-stage build for AI-CORE
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/ai-core

# Copy Cargo files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source files to build dependencies
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/api-gateway.rs && \
    echo "fn main() {}" > src/bin/mcp-manager.rs && \
    echo "fn main() {}" > src/bin/intent-parser.rs && \
    mkdir -p src/lib && \
    echo "" > src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && \
    rm -rf src/

# Copy actual source code
COPY src/ ./src/

# Build the actual application
RUN touch src/lib.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN groupadd -r aicore && useradd -r -g aicore aicore

# Set working directory
WORKDIR /app

# Copy binaries from builder stage
COPY --from=builder /usr/src/ai-core/target/release/api-gateway /app/
COPY --from=builder /usr/src/ai-core/target/release/mcp-manager /app/
COPY --from=builder /usr/src/ai-core/target/release/intent-parser /app/

# Copy configuration files
COPY config/ ./config/

# Create necessary directories
RUN mkdir -p logs data && \
    chown -R aicore:aicore /app

# Switch to app user
USER aicore

# Expose ports
EXPOSE 8080 8081 8082

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Default command (can be overridden)
CMD ["./api-gateway"]

# Labels for metadata
LABEL org.opencontainers.image.title="AI-CORE"
LABEL org.opencontainers.image.description="AI-assisted software development environment"
LABEL org.opencontainers.image.vendor="NetADX.ai"
LABEL org.opencontainers.image.source="https://github.com/netadx1ai/ai-core"
LABEL org.opencontainers.image.documentation="https://docs.netadx.ai"
LABEL org.opencontainers.image.licenses="MIT"
