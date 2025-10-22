# Project Summary: MQTT Mass Test

This project provides two Rust binaries: `mqtt-publish` (publisher) and `mqtt-subscribe` (subscriber). Both are designed for high-performance MQTT traffic generation and analysis, featuring an interactive console GUI (TUI) for configuration and real-time metrics display.

## Key Features:
*   **Interactive Console GUI (TUI):** Built with Ratatui for easy configuration and live monitoring.
*   **Multiple Client Threads:** Supports spawning hundreds of concurrent MQTT clients (producers or subscribers).
*   **Hierarchical Topic Structure:** Generates complex topic hierarchies with configurable depth.
*   **Real-time Metrics:** Displays live metrics including total messages published/received, values/second (v/s) rate, per-client performance, and uptime.
*   **Configurable Publishing (mqtt-publish):** Allows customization of sleep intervals, QoS levels, retained flag, and payload format (ISO timestamp with counter and random value). **Metric topic publishing has been removed.**
*   **Configurable Subscribing (mqtt-subscribe):** Allows subscribing to a configurable percentage of generated topics.
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
*   `config.rs`: Handles configuration structure, serialization/deserialization, and default values. Now includes `subscribe_percentage`.
*   `metrics.rs`: Manages per-client atomic metrics tracking (published and received) and global aggregation. `ProducerMetrics` was refactored to `ClientMetrics`.
*   `topic.rs`: Responsible for generating hierarchical topic structures.
*   `producer.rs`: Implements MQTT client connections, asynchronous publishing loops, and metrics publication for each producer.
*   `subscriber.rs`: Implements MQTT client connections, subscription to a percentage of topics, and received message tracking for each subscriber.
*   `publisher_main.rs`: Entry point for the `mqtt-publish` (publisher) binary.
*   `subscriber_main.rs`: Entry point for the `mqtt-subscribe` (subscriber) binary.
*   `ui.rs`: Provides the Ratatui-based console GUI with configuration input, confirmation, and live metrics screens. Now uses "Clients" instead of "Producers" in the UI.

## Building and Running:

### Requirements:
*   Rust 1.70+ (Edition 2021)
*   macOS, Linux, or Windows

### Build Commands:
To build both binaries in release mode (optimized):
```bash
cargo build --release
```
The executables will be located at `target/release/mqtt-publish` and `target/release/mqtt-subscribe`.

### Running Commands:
**MQTT Publisher (mqtt-publish):**
**Interactive Mode (Default):**
```bash
./target/release/mqtt-publish
```
This launches the interactive TUI for configuration and monitoring of publishing.

**Command Line Options (Non-interactive):**
```bash
./target/release/mqtt-publish --broker 192.168.1.100 --port 1883 --config config.json --auto-start
```

**MQTT Subscriber (mqtt-subscribe):**
**Interactive Mode (Default):**
```bash
./target/release/mqtt-subscribe
```
This launches the interactive TUI for configuration and monitoring of subscribing.

**Command Line Options (Non-interactive):**
```bash
./target/release/mqtt-subscribe --broker 192.168.1.100 --port 1883 --config example-subscribe-config.json --auto-start --subscribe-percentage 50
```

### Testing:
To run the project's tests:
```bash
cargo test
```

## Development Conventions:
*   **Code Style:** Adheres to standard Rust formatting and best practices.
*   **Concurrency:** Utilizes `tokio` for asynchronous operations and `Arc<Mutex<GlobalMetrics>>` for shared state management.
*   **Modularity:** Code is organized into distinct modules based on functionality.
*   **Performance:** Employs atomic operations, `fastrand` for RNG, and minimal allocations in hot paths for high performance.