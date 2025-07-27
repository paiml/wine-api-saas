# Multi-stage build for optimized Rust container
FROM rust:1.88-slim AS builder

WORKDIR /app

# Install required dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy the actual source code
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage with minimal base image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder stage
COPY --from=builder /app/target/release/wine-api /app/wine-api

# Create directory for database volume
RUN mkdir -p /app/data

# Create a non-root user
RUN useradd -r -s /bin/false appuser
RUN chown -R appuser:appuser /app
USER appuser

# Expose port
EXPOSE 3000

# Create volume for database
VOLUME ["/app/data"]

# Set environment variables
ENV RUST_LOG=info
ENV DATABASE_URL=sqlite:/app/data/wine_ratings.db

# Run the application
CMD ["./wine-api"]
