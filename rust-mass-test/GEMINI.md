# Project Summary: MQTT Mass Test

This project is a high-performance MQTT test program written in Rust, designed to generate significant MQTT traffic for stress testing, benchmarking, and load testing MQTT brokers. It features an interactive console GUI (TUI) for configuration and real-time metrics display.

## Key Features:
*   **Interactive Console GUI (TUI):** Built with Ratatui for easy configuration and live monitoring.
*   **Multiple Producer Threads:** Supports spawning hundreds of concurrent MQTT producers.
*   **Hierarchical Topic Structure:** Generates complex topic hierarchies with configurable depth.
*   **Real-time Metrics:** Displays live metrics including total messages published, values/second (v/s) rate, per-producer performance, and uptime.
*   **Configurable Publishing:** Allows customization of sleep intervals, QoS levels, retained flag, and payload format (ISO timestamp with counter and random value).
*   **Configuration Management:** Supports saving and loading configurations as JSON files.
*   **CLI Arguments:** Overrides broker host/port and enables auto-start with a configuration file.

## Technologies Used:
*   **Language:** Rust (Edition 2021)
*   **MQTT Client:** `rumqttc` (pure Rust)
*   **Asynchronous Runtime:** `tokio`
*   **Terminal UI:** `ratatui`, `crossterm`
*   **Serialization:** `serde`, `serde_json`
*   **Date/Time:** `chrono`
*   **CLI Argument Parsing:** `clap`
*   **Random Number Generation:** `fastrand`

## Architecture Overview:
The application is structured into several modules:
*   `config.rs`: Handles configuration structure, serialization/deserialization, and default values.
*   `metrics.rs`: Manages per-producer atomic metrics tracking and global aggregation.
*   `topic.rs`: Responsible for generating hierarchical topic structures.
*   `producer.rs`: Implements MQTT client connections, asynchronous publishing loops, and metrics publication for each producer.
*   `ui.rs`: Provides the Ratatui-based console GUI with configuration input, confirmation, and live metrics screens.
*   `main.rs`: Orchestrates the application, parses CLI arguments, and manages producer spawning and monitoring.

## Building and Running:

### Requirements:
*   Rust 1.70+ (Edition 2021)
*   macOS, Linux, or Windows

### Build Commands:
To build the project in release mode (optimized):
```bash
cargo build --release
```
The executable will be located at `target/release/mqtt-test`.

### Running Commands:
**Interactive Mode (Default):**
```bash
./target/release/mqtt-test
```
This launches the interactive TUI for configuration and monitoring.

**Command Line Options (Non-interactive):**
```bash
./target/release/mqtt-test --broker 192.168.1.100 --port 1883 --config config.json --auto-start
```
This allows overriding broker details and starting the test immediately with a specified configuration file.

### Testing:
To run the project's tests:
```bash
cargo test
```

## Development Conventions:
*   **Code Style:** Adheres to standard Rust formatting and best practices.
*   **Concurrency:** Utilizes `tokio` for asynchronous operations and `Arc<GlobalMetrics>` for shared state management.
*   **Modularity:** Code is organized into distinct modules based on functionality.
*   **Performance:** Employs atomic operations, send-safe RNG, and minimal allocations in hot paths for high performance.
