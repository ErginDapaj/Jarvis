# Build stage
FROM rust:1.82-bookworm AS builder

WORKDIR /app

# Install dependencies for sqlx
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src
COPY migrations ./migrations

# Build the application
# SQLx will connect to database during build to verify queries
# Make sure DATABASE_URL is available during build if not using offline mode
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/jarvis /app/jarvis

# Copy migrations for runtime migration support
COPY migrations ./migrations

ENV RUST_LOG=jarvis=info,serenity=warn

CMD ["./jarvis"]
