# MQTT Mass Test - Super Fast Rust MQTT Test Program

A high-performance MQTT test program written in Rust with an interactive console GUI (TUI). Designed to generate massive amounts of MQTT traffic for stress testing, benchmarking, and load testing MQTT brokers.

## Features

- **Interactive Console GUI** - Beautiful terminal UI built with Ratatui for easy configuration
- **Multiple Producer Threads** - Spawn hundreds of concurrent MQTT producers
- **Hierarchical Topic Structure** - Generate complex topic hierarchies with configurable depth
- **Real-time Metrics** - Live metrics display showing:
  - Total messages published (global and per-producer)
  - Values/second (v/s) rate calculation
  - Per-producer performance tracking
  - Uptime counter
- **Configurable Publishing**
  - Custom sleep between publishes (controls message rate)
  - QoS levels (0, 1, 2)
  - Retained flag support
  - ISO timestamp payloads with counter and random value
- **Config Management** - Save and load configurations as JSON
- **CLI Arguments** - Override broker host/port via command line

## Building

### Requirements
- Rust 1.70+ (we use edition 2021)
- macOS, Linux, or Windows

### Build
```bash
cargo build --release
```

The binary will be at `target/release/mqtt-test`

## Usage

### Interactive Mode (Default)
```bash
./target/release/mqtt-test
```

This launches the interactive TUI configuration screen where you can:
1. Configure all parameters (broker, producers, topics, sleep time, etc.)
2. Review configuration summary
3. Start the test and monitor live metrics

### Command Line Options
```bash
./target/release/mqtt-test --broker 192.168.1.100 --port 1883 --config config.json --auto-start
```

Options:
- `--broker HOST` - MQTT broker hostname (default: localhost)
- `--port PORT` - MQTT broker port (default: 1883)
- `--config FILE` - Load configuration from JSON file
- `--auto-start` - Skip UI and start immediately (requires config file)

## Configuration

### Interactive Configuration Fields

| Field | Description | Default |
|-------|-------------|---------|
| Broker Host | MQTT broker hostname/IP | localhost |
| Broker Port | MQTT broker port | 1883 |
| Producers | Number of concurrent producer threads | 10 |
| Num Topics | Number of base topics per producer | 100 |
| Topics per Node | Branching factor (children per node) | 10 |
| Max Depth | Tree depth (levels of subtopics) | 3 |
| Sleep (ms) | Milliseconds between publishes | 100 |
| QoS | MQTT Quality of Service (0, 1, 2) | 1 |
| Retained | Publish with retained flag | false |
| Topic Prefix | Base topic prefix | test |

### Topic Structure Example

With default settings (100 topics, 10 per node, depth 3):
```
test_00001/                              (root)
├── test_00001/01/                       (depth 1)
│   ├── test_00001/01/01/               (depth 2)
│   │   ├── test_00001/01/01/01         (depth 3)
│   │   ├── test_00001/01/01/02
│   │   └── ...
│   └── ...
├── test_00001/02/
└── ...
test_00002/
...
test_00100/
```

Each producer publishes to all topics in its hierarchy.

### Metrics Topic

Each producer publishes periodic metrics to:
```
test_XXXXX/metrics
```

With payload:
```json
{
  "ts": "2024-10-21T17:52:00.123456Z",
  "producer_id": 1,
  "total_published": 15234,
  "counter": 567
}
```

### Payload Format

Each regular message contains:
```json
{
  "ts": "2024-10-21T17:52:00.123456Z",
  "counter": 1234,
  "value": 0.8234
}
```

## Saving and Loading Configurations

### Save Configuration

After configuring in the UI, you can export settings. To save a configuration manually, use:

```bash
# The application will prompt to save
# Configuration is saved as JSON: config.json
```

### Load Configuration

```bash
./target/release/mqtt-test --config my-config.json --auto-start
```

Example config.json:
```json
{
  "broker_host": "localhost",
  "broker_port": 1883,
  "num_producers": 10,
  "num_topics": 100,
  "topics_per_node": 10,
  "max_depth": 3,
  "sleep_ms": 100,
  "qos": 1,
  "retained": false,
  "topic_prefix": "test"
}
```

## Performance Calculations

### Messages Per Second (v/s)

**Per Producer v/s:**
- Formula: `(messages_published_in_interval / time_interval_ms) * 1000`
- Updated every 1 second

**Global v/s:**
- Sum of all producer v/s values

### Example Calculation

With 10 producers, each publishing 100 messages/second:
```
Global v/s = 10 × 100 = 1,000 messages/second
```

If each producer publishes to 1,111 topics (100 topics × 10 per node, ~3 depth):
- With 100ms sleep between publishes:
- Each producer: 1 message per 100ms × 1,111 topics = ~111 messages/second
- Global: 10 × 111 = ~1,110 messages/second

## Real-time Metrics Display

The live metrics screen shows:

```
Global Metrics
═══════════════════════════════════════════════════════════
Total Published: 50234  |  Global v/s: 1234.56
Uptime: 0:02:34  |  Active Producers: 10
═══════════════════════════════════════════════════════════

Per-Producer Metrics
Producer   1: Total=    5023  v/s= 123.45  Counter=    567
Producer   2: Total=    5034  v/s= 125.12  Counter=    568
Producer   3: Total=    5021  v/s= 122.89  Counter=    566
...
```

## Usage Scenarios

### Light Load Testing
```
Producers: 5
Topics: 50
Topics per Node: 10
Max Depth: 2
Sleep: 200ms
```

### Medium Load Testing
```
Producers: 20
Topics: 100
Topics per Node: 10
Max Depth: 3
Sleep: 50ms
```

### Heavy Stress Testing
```
Producers: 100
Topics: 1000
Topics per Node: 10
Max Depth: 3
Sleep: 10ms
```

## Controls

### Configuration Screen
- **↑/↓** - Navigate between fields
- **ENTER** - Edit currently selected field
- **SPACE** - Start the test immediately (no confirmation needed)
- **Q** - Quit without starting

### Metrics Screen (Runs Endlessly)
- **Q** - Stop test and quit
- **CTRL+C** - Force quit (if needed)

## Architecture

### Components

1. **Config Module** (`src/config.rs`)
   - Configuration structure with serialization/deserialization
   - Save/load to/from JSON
   - Default values

2. **Metrics Module** (`src/metrics.rs`)
   - Per-producer atomic metrics tracking
   - Global metrics aggregation
   - Real-time v/s calculation

3. **Topic Module** (`src/topic.rs`)
   - Topic generation with tree structure
   - Hierarchical naming with numerical sequences

4. **Producer Module** (`src/producer.rs`)
   - MQTT client connection per producer
   - Async publishing loop
   - Metrics publication

5. **UI Module** (`src/ui.rs`)
   - Ratatui-based console GUI
   - Three screens: config input, confirmation, live metrics
   - Keyboard event handling

6. **Main Module** (`src/main.rs`)
   - CLI argument parsing
   - Application orchestration
   - Producer spawning and monitoring

## Technical Details

### Dependencies
- **rumqttc** - Pure Rust MQTT client (no external C dependencies)
- **tokio** - Async runtime
- **ratatui** - Terminal UI framework
- **crossterm** - Terminal control
- **serde/serde_json** - Configuration serialization
- **chrono** - ISO timestamp generation
- **clap** - CLI argument parsing
- **fastrand** - Fast random number generation

### Performance Optimizations
- Atomic operations for metrics (no locks)
- Send-safe RNG (fastrand)
- Async/await with Tokio for concurrent producers
- Pre-generated topic list (computed once per producer)
- Minimal allocations in hot paths

### Async Architecture
- Each producer runs as an independent Tokio task
- One MQTT connection per producer
- UI runs in blocking mode on main thread
- Metrics shared via Arc<GlobalMetrics>

## Troubleshooting

### "Connection refused"
- Verify MQTT broker is running and accessible
- Check broker host and port are correct
- Test with `nc -zv broker_host broker_port`

### Low message rate
- Reduce `Sleep (ms)` value for faster publishing
- Increase `Producers` for more concurrent connections
- Check broker capacity and network latency

### High memory usage
- Reduce number of producers
- Reduce number of topics per producer
- Lower max depth to reduce topic count

### Terminal UI issues
- Ensure terminal supports ANSI colors and size 80x24 minimum
- Try resizing terminal window
- Run with `TERM=xterm-256color` if colors are wrong

## Building from Source

```bash
# Clone repository
cd rust-mass-test

# Build debug version
cargo build

# Build release (optimized)
cargo build --release

# Run tests
cargo test

# Run with verbose output
cargo build --verbose
```

## License

MIT

## Contributing

Contributions welcome! Areas for improvement:
- Connection pooling support
- WebSocket MQTT support
- Payload customization options
- Statistics export (CSV/JSON)
- Remote monitoring API
- Batch topic operations

## Performance Notes

This tool is designed for testing and can generate very high message rates. Use responsibly:

- Start with small producer counts and increase gradually
- Monitor broker CPU and memory usage
- Test on non-production systems first
- Ensure adequate network bandwidth
- Consider broker limits and configuration

Typical rates:
- Single producer: 500-2000 msg/s (depends on broker)
- 10 producers: 5,000-20,000 msg/s
- 100 producers: 50,000-200,000 msg/s

Actual rates depend on:
- Broker performance and configuration
- Network latency
- Topic count and tree depth
- Message payload size
- QoS level used
- System resources (CPU, memory, disk I/O)
