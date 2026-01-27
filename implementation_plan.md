# Rewrite SMPP Simulator in Rust

## Goal Description
Reimplement the existing Java-based SMPP Simulator in Rust to create a modern, high-performance, and resource-efficient SMPP testing tool.

**Key Value Propositions:**
1.  **Ease of Use**: A "batteries-included" simulator that requires no low-level SMPP knowledge to set up.
2.  **Web Interface**: A rich web dashboard to:
    -   Configure the simulator (ports, system IDs, behavior) dynamically.
    -   View real-time logs and sessions.
    -   Monitor server status.
    -   Inject MO (Mobile Originated) messages for testing.
3.  **Docker Native**: A single, small-footprint binary packaged in a minimal Docker image for easy integration into CI/CD pipelines.

## User Review Required
> [!NOTE]
> We will use `rusmpp` for the underlying SMPP v5 implementation. Our focus is building the *management and simulation layer* on top of it.

> [!IMPORTANT]
> The Web UI will allow modifying configuration at runtime where possible. Persistent configuration will also be supported.

## Proposed Changes

### Project Structure & Dependencies
#### [NEW] [Cargo.toml](file:///d:/Project/rust-smpp-sim/Cargo.toml)
- Use `tokio` (full features) as the async runtime.
- Use `rusmpp` for SMPP parsing/encoding.
- Use `actix-web` or `axum` for the HTTP management interface.
- Use `tracing` and `tracing-subscriber` for logging.
- Use `config` crate or `dotenvy` for configuration.
- Use `serde` for serialization.

### Core Implementation
#### [NEW] [src/main.rs](file:///d:/Project/rust-smpp-sim/src/main.rs)
- Initialize logging (`tracing`).
- Load configuration.
- Start the SMPP server (TCP listener).
- Start the HTTP server (web interface).

#### [NEW] [src/smpp_server.rs](file:///d:/Project/rust-smpp-sim/src/smpp_server.rs)
- Implement `bind_transmitter`, `bind_receiver`, `bind_transceiver` handling.
- Handle `submit_sm` (store message/queue).
- Handle `enquire_link`.
- Manage active sessions.

#### [NEW] [src/session.rs](file:///d:/Project/rust-smpp-sim/src/session.rs)
- Manage state of connected clients (ESMEs).

#### [NEW] [src/web_interface.rs](file:///d:/Project/rust-smpp-sim/src/web_interface.rs)
- **Dashboard**: HTML/CSS interface (using Askama templates or similar for SSR, or static assets + API).
- **Configuration API**: Endpoints to read/update settings.
- **Log Stream**: WebSocket or SSE endpoint for real-time logs.
- **MO Injection**: Form/API to simulate incoming messages.

### Testing & Deployment
#### [NEW] [Dockerfile](file:///d:/Project/rust-smpp-sim/Dockerfile)
- Multi-stage build.
- `builder` stage: Compiles the Rust binary.
- `runtime` stage: `gcr.io/distroless/cc` or `alpine` (if musl linked) to keep image size minimal (<50MB).

#### [NEW] [.github/workflows/ci.yml](file:///d:/Project/rust-smpp-sim/.github/workflows/ci.yml)
- Build and test on push/PR.
- Build Docker image and push to registry (if configured).

## Verification Plan

### Automated Tests
- **Unit Tests**: Test PDU parsing and logic functions in Rust.
  `cargo test`
- **Integration Tests**: Spin up the server and use a Rust SMPP client (test harness) to connect, bind, and send/receive messages.

### Manual Verification
- Run the docker container:
  ```sh
  docker build -t rust-smpp .
  docker run -p 2775:2775 -p 8080:8080 rust-smpp
  ```
- Use an external SMPP client (like `smpp-load` or the original Java `SMPPSim` client if compatible) to verify connectivity.
- Check HTTP interface at `http://localhost:8080/`.
