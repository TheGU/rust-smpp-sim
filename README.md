# Rust SMPP Simulator

![CI](https://github.com/TheGU/rust-smpp-sim/actions/workflows/ci.yml/badge.svg)
![Release](https://img.shields.io/github/v/release/TheGU/rust-smpp-sim)
[![GHCR](https://img.shields.io/badge/ghcr-latest-blue?logo=github)](https://github.com/TheGU/rust-smpp-sim/pkgs/container/rust-smpp-sim)

A high-performance, asynchronous SMPP 5.0 simulator written in Rust. Designed to replace legacy Java-based simulators with improved performance, stability, and modern features.

## Features

- **SMPP 5.0 Support**: Fully implements `BindTransmitter`, `BindReceiver`, `BindTransceiver`, `SubmitSm`, `EnquireLink`, and `Unbind`.
- **Lifecycle Simulation**: Configurable message states (`Delivered`, `Undeliverable`, `Accepted`, `Rejected`) with random transition probabilities and delays.
- **Delivery Receipts**: Automatically generates and sends `DeliverSm` receipts back to the client based on the simulated lifecycle.
- **MO Injection**: Periodic injection of Mobile Originated messages from CSV files or manual triggers.
- **Web Dashboard**: Real-time web interface to view :
  - Active Sessions
  - Message Queues (Outbound & Inbound)
  - System Statistics (Messages/sec, Total Processed)
  - Manual MO Injection
- **Multi-Account Support**: Configure multiple system IDs and passwords.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)

### Building

```sh
cargo build --release
```

### Running

```sh
# Run with default settings
cargo run

# Run with custom log level
RUST_LOG=debug cargo run
```

### Running with Docker

You can use the Docker image published to GHCR:

```sh
# Pull the latest image
docker pull ghcr.io/thegu/rust-smpp-sim:latest

# Run the container
docker run -p 2775:2775 -p 8080:8080 ghcr.io/thegu/rust-smpp-sim:latest
```

The server listens on **port 2775** for SMPP connections and **port 8080** for the Web Dashboard by default.

## Configuration

Configuration is managed via the `config` crate and supports environment variables and configuration files.

### Key Configuration Options

| Category      | Variable                        | Default   | Description                      |
| ------------- | ------------------------------- | --------- | -------------------------------- |
| **Server**    | `SERVER_HOST`                   | `0.0.0.0` | Binding IP address               |
|               | `SERVER_PORT`                   | `8080`    | Web Dashboard port               |
| **SMPP**      | `SMPP_PORT`                     | `2775`    | SMPP listening port              |
|               | `SMPP_SYSTEM_ID`                | `user`    | Default System ID                |
|               | `SMPP_PASSWORD`                 | `pass`    | Default Password                 |
| **Logging**   | `LOG_LEVEL`                     | `info`    | Log level (info, debug, trace)   |
| **Lifecycle** | `LIFECYCLE_MAX_TIME_ENROUTE_MS` | `5000`    | Max time before state transition |
|               | `LIFECYCLE_PERCENT_DELIVERED`   | `90`      | Probability of `DELIVRD` status  |

## Usage

1. **Connect**: Use any SMPP client (e.g., `smpp-cli`, Kannel, or custom code) to bind to `localhost:2775` with `user`/`pass`.
2. **Submit**: Send `SubmitSm` PDUs.
3. **Monitor**: Open http://localhost:8080 to see the message appear in the queue.
4. **Receipts**: Watch your client receive `DeliverSm` receipts after the simulated delay.

## Web Interface

Access the dashboard at `http://localhost:8080`.

- **Dashboard**: Overview of active sessions and message counts.
- **Logs**: Real-time server logs.
- **Injection**: Upload CSVs or manually inject MO messages.
