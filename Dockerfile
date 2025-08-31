# Build stage
FROM rust:1.82 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY static ./static
COPY config.toml ./

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

# Create app directory
RUN mkdir -p /app/static && chown -R appuser:appuser /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/img-server-rs /usr/local/bin/img-server-rs

# Copy static files
COPY --from=builder /app/static /app/static

# Change ownership
RUN chown appuser:appuser /usr/local/bin/img-server-rs

# Switch to app user
USER appuser

# Set working directory
WORKDIR /app

# Expose port (更新为正确的端口)
EXPOSE 3030

# Health check (更新为正确的端口和路径)
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3030/ || exit 1

# Run the application
CMD ["img-server-rs"]
