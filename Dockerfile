# Build stage
FROM rust:1-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev build-essential && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to cache dependencies - REMOVED FOR DEBUGGING
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src
COPY templates ./templates
COPY static ./static

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:trixie-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/rust-smpp-sim /app/rust-smpp-sim
COPY --from=builder /app/templates /app/templates
COPY --from=builder /app/static /app/static

# Environment variables
# Environment variables
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080
ENV SMPP_PORT=2775
ENV SMPP_SYSTEM_ID=smppclient1
# Password should be supplied at runtime for security
# ENV SMPP_PASSWORD=password 
ENV LOG_LEVEL=info

# Expose ports
EXPOSE 8080 2775

# Run the application
CMD ["/app/rust-smpp-sim"]
