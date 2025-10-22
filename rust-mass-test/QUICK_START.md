# Quick Start Guide

## 🚀 Get Running in 30 Seconds

### 1. Build
```bash
cargo build --release
```
Binary: `target/release/mqtt-test` (2.1 MB)

### 2. Run
```bash
./target/release/mqtt-test
```

### 3. Configure in GUI
```
↑/↓     Navigate fields
ENTER   Edit field
SPACE   Start test immediately
Q       Quit
```

### 4. Watch Metrics (Runs Endlessly)
```
Press Q to stop and exit
```

---

## 💡 Common Use Cases

### Test Local Broker (Default)
```bash
./target/release/mqtt-test
# Uses localhost:1883
```

### Remote Broker
```bash
./target/release/mqtt-test --broker 192.168.1.100 --port 1883
```

### Auto-start (Script-Friendly)
```bash
./target/release/mqtt-test --config config.json --auto-start
```

### Save Configuration
1. Run interactive mode
2. Configure settings
3. Settings auto-save to `config.json`

### Load Saved Config
```bash
./target/release/mqtt-test --config config.json
```

---

## 📊 Preset Configurations

### Light Load
Edit these fields in GUI:
- Producers: 5
- Num Topics: 50
- Topics/Node: 10
- Max Depth: 2
- Sleep: 200ms

### Medium Load
- Producers: 20
- Num Topics: 100
- Topics/Node: 10
- Max Depth: 3
- Sleep: 50ms

### Heavy Load
- Producers: 100
- Num Topics: 1000
- Topics/Node: 10
- Max Depth: 3
- Sleep: 10ms

---

## 🔧 Configuration Parameters

| Field | What It Does | Range | Default |
|-------|-------------|-------|---------|
| **Producers** | Number of concurrent workers | 1-1000 | 10 |
| **Num Topics** | Total topics per producer | 1-10000 | 100 |
| **Topics/Node** | Branching factor | 1-10 | 10 |
| **Max Depth** | Topic tree depth | 1-10 | 3 |
| **Sleep (ms)** | Pause between publishes | 1-10000 | 100 |
| **QoS** | Message quality | 0, 1, 2 | 1 |
| **Retained** | Keep last message | true/false | false |
| **Broker** | Server address | any IP/host | localhost |
| **Port** | Server port | 1-65535 | 1883 |
| **Prefix** | Topic name prefix | any string | test |

---

## 📈 Understanding the Metrics

### Global Metrics
```
Total Published: 50,234   ← Total messages sent so far
Global v/s: 1,234.56      ← Messages per second (all producers)
Uptime: 0:02:34          ← How long test has been running
Active Producers: 10      ← Number of producing threads
```

### Per-Producer Metrics
```
Producer   1: Total=5023  v/s=123.45  Counter=567
             ↑           ↑             ↑
             ID      Cumulative    Per-Cycle
                     Total         Counter
```

### Calculating Message Rate
**Formula:**
```
Topics per Producer = Num Topics × (Topics/Node ^ Max Depth)
Msg/Sec per Producer = 1000 / Sleep(ms)
Global Msg/Sec = Producers × Msg/Sec per Producer
```

**Example:**
```
Config: 10 producers, 100 topics, 10 per node, depth 3, sleep 100ms
Topics per producer = 100 × 10 × 10 × 10 = 100,000
Msg/sec per producer = 1000 / 100 = 10 msg/sec
Global = 10 producers × 10 = 100 msg/sec
```

---

## 🎯 Topic Structure

### How Topics Are Generated

With default settings:
```
test_00001          ← Level 0 (root)
├─ test_00001/01    ← Level 1
│  ├─ test_00001/01/01  ← Level 2
│  │  ├─ test_00001/01/01/01  ← Level 3
│  │  ├─ test_00001/01/01/02
│  │  └─ ...
│  └─ test_00001/01/02
├─ test_00001/02
└─ test_00001/03    ... to /10
```

Each producer gets its own base topic (test_00001, test_00002, etc.)

### Metrics Topic
```
test_00001/metrics  ← Special topic for metrics publication
```

---

## 📝 Example Configuration

Save as `heavy-load.json`:
```json
{
  "broker_host": "192.168.1.100",
  "broker_port": 1883,
  "num_producers": 50,
  "num_topics": 500,
  "topics_per_node": 10,
  "max_depth": 3,
  "sleep_ms": 20,
  "qos": 1,
  "retained": false,
  "topic_prefix": "stress-test"
}
```

Use it:
```bash
./target/release/mqtt-test --config heavy-load.json --auto-start
```

---

## ⚡ Performance Tips

### To Increase Message Rate
1. Lower `Sleep (ms)` - More messages per second per producer
2. Increase `Producers` - More concurrent connections
3. Reduce `Max Depth` - Fewer total topics

### To Reduce Load
1. Raise `Sleep (ms)` - Wait longer between messages
2. Lower `Producers` - Fewer concurrent connections
3. Lower `Num Topics` - Publish to fewer topics

### To Reduce Memory Usage
1. Decrease `Producers` - Fewer connections
2. Decrease `Num Topics` - Fewer topics to track
3. Lower `Max Depth` - Smaller topic tree

---

## 🐛 Troubleshooting

| Problem | Solution |
|---------|----------|
| Connection refused | Check broker is running, host/port correct |
| Very slow | Increase sleep value, check broker logs |
| High CPU | Reduce producers or increase sleep |
| High Memory | Reduce topics or producers |
| Terminal garbled | Resize window, check TERM=xterm-256color |
| GUI laggy | Reduce producer count for faster updates |

---

## 💾 Files Created

```
mqtt-test (binary)              ← Run this!
README.md                       ← Full documentation
PROJECT_SUMMARY.md             ← Technical details
QUICK_START.md                 ← This file
example-config.json            ← Example config
.gitignore                      ← Git ignore rules
src/                            ← Source code
  main.rs, config.rs, etc.
```

---

## 🎓 Example Session

```bash
# 1. Build
$ cargo build --release
   Compiling mqtt-test v0.1.0
   Finished release [optimized] target(s) in 13.94s

# 2. Run
$ ./target/release/mqtt-test

# 3. Configure using ↑/↓ and ENTER to edit fields
   ┌──────────────────────────────────┐
   │ Broker Host      localhost        │
   │ Broker Port      1883             │
   │ Producers        10               │  ← Edit this to 20
   │ ...                               │
   └──────────────────────────────────┘

# 4. Press SPACE to start immediately (no confirmation needed!)

# 5. Metrics display starts - runs forever
   Total Published: 1,234  |  Global v/s: 456.78
   Producer   1: Total=123  v/s=45.67  Counter=12
   Producer   2: Total=124  v/s=46.12  Counter=13
   ...

# 6. Press Q to stop
```

---

## 📞 Need Help?

- **README.md** - Full documentation
- **PROJECT_SUMMARY.md** - Architecture & technical details
- **example-config.json** - Config file template

---

**Happy Testing! 🚀**
