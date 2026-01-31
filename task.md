# Rust SMPP Simulator Reimplementation

- [ ] **Preparation**
  - [/] Analyze existing Java codebase
  - [x] Initialize Rust project structure (`cargo new`)
  - [x] Set up dependencies (`rusmpp`, `tokio`, `actix-web`)
  - [x] Create `implementation_plan.md`

- [ ] **Core Implementation**
  - [x] Implement Configuration Loader (Properties/Env)
  - [x] Implement Logging & Tracing
- [ ] **Core Implementation**
  - [x] Implement Configuration Loader (Properties/Env)
  - [x] Implement Logging & Tracing
  - [x] Create Async TCP Server Stub
  - [x] Implement SMPP Protocol Handling (Bind, Unbind)
  - [x] Implement Session Management
  - [x] Implement Message Queues (Inbound/Outbound)
  - [x] Implement SubmitSM & DeliverSM Logic
  - [x] Implement EnquireLink & KeepAlive

- [x] **Web Interface**
  - [x] Create HTTP Server (Actix/Axum)
  - [x] Implement Dashboard (HTML/CSS) forStatus
  - [x] Implement Configuration View & Edit API
  - [x] Implement Real-time Log View (WebSocket/SSE)
  - [x] Implement MO Message Injection UI

- [ ] **Verification & Testing**
  - [x] Unit Tests for Protocol logic
  - [x] Integration Tests with `smpp-client`
  - [x] Verify Docker Build

- [ ] **Deployment**
  - [x] Create Optimization-focused Dockerfile
  - [ ] Create CI/CD Pipeline (GitHub Actions)
  - [ ] Create Release script/docs
  - [ ] Create publish image on ghcr.io
