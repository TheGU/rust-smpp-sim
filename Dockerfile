# Build stage
FROM rust:1.85-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src
COPY templates ./templates

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/rust-smpp-sim /app/rust-smpp-sim
COPY --from=builder /app/templates /app/templates

# Environment variables
ENV SMPP__SERVER__HOST=0.0.0.0
ENV SMPP__SERVER__PORT=8080
ENV SMPP__SMPP__PORT=2775
ENV SMPP__SMPP__SYSTEM_ID=smppclient1
ENV SMPP__SMPP__PASSWORD=password
ENV SMPP__LOG__LEVEL=info

# Expose ports
EXPOSE 8080 2775

# Run the application
CMD ["/app/rust-smpp-sim"]
