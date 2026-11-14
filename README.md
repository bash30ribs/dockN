# dockN — Pillar 1: The Subconscious

> **Personal Analytics with eBPF & High-Speed Adaptive Telemetry**  
> *"Most users interact with their system at a high level (`top`, `htop`). To stand out, you must understand your system at the kernel level. This pillar creates a rich, real-time data stream of your digital life—providing the raw material for true personalization."*

---

## 🏛️ Architecture Overview

```
                                      +-----------------------------------------+
                                      |             Host Environment            |
                                      +--------------------+--------------------+
                                                           |
                      +------------------------------------+-----------------------------------+
                      |                                                                        |
     [Linux with Kernel Privileges]                                        [Cross-Platform / Company / Mac / Windows]
                      |                                                                        |
                      v                                                                        v
         +--------------------------+                                             +--------------------------+
         |     Linux eBPF Probe     |                                             | Universal Adaptive Engine|
         | • sched:sched_process_exec|                                            | • Differential Proc Poll |
         | • sched:sched_process_exit|                                            | • Socket State Snooping  |
         | • Kernel Ring Buffer     |                                             | • Recursive FS Watcher   |
         +------------+-------------+                                             +------------+-------------+
                      |                                                                        |
                      +------------------------------------+-----------------------------------+
                                                           |
                                                           v
                                            +------------------------------+
                                            |   Context Enricher & Queue   |
                                            |  • Microsecond Timestamps    |
                                            |  • Dev/IDE/Terminal Tagging  |
                                            |  • Non-blocking Ring Channel |
                                            +--------------+---------------+
                                                           |
                                                           v
                                            +------------------------------+
                                            |    High-Speed SQLite (WAL)   |
                                            |  • 256MB Memory-Mapped I/O   |
                                            |  • Micro-batch Transactions  |
                                            |  • Local subconscious.db     |
                                            +--------------+---------------+
                                                           |
                                  +------------------------+------------------------+
                                  |                                                 |
                                  v                                                 v
                     +--------------------------+                      +--------------------------+
                     |    Instant CLI Report    |                      |      JSON API Stream     |
                     |  • Deep Work Score       |                      |  • LLM / Agent Pipeline  |
                     |  • Tool Latency Ranking  |                      |  • Personal Profiling    |
                     |  • File Churn Hotspots   |                      |  • Zero-Overhead Feeds   |
                     +--------------------------+                      +--------------------------+
```

---

## ⚡ Key Capabilities

1. **Dual-Engine Telemetry (Runs on Any OS & Company Setup):**
   - **Linux eBPF Mode:** Taps directly into kernel execution tracepoints (`sched_process_exec`, `sched_process_exit`, network sockets) with zero performance overhead.
   - **Universal Adaptive Mode:** Runs seamlessly without root privileges on macOS, Windows, Linux, and corporate laptops via high-frequency differential tracking and OS event notifications.
   - **Automatic Detection:** Automatically activates eBPF when available; gracefully falls back to the adaptive engine without crashing.

2. **User-Centric Intelligence (The "Subconscious" Metrics):**
   - **Deep Work Index:** Quantifies unbroken focus time vs. context switching fragmentation.
   - **Dev Tool Bottlenecks:** Measures P50, P95, and max execution latency for tools (`cargo`, `rustc`, `git`, `python`, `docker`, test suites).
   - **Workspace File Churn Hotspots:** Identifies files and directories undergoing high-velocity edits.
   - **Network Egress Footprint:** Tracks remote domains and socket connections created by local developer tools.

3. **New-Gen Local Storage Architecture:**
   - **Local SQLite Engine:** Strict personal privacy (data never leaves your machine).
   - **Tuned WAL Mode:** Zero write stalls, `synchronous = NORMAL`, and 256MB memory-mapped I/O (`PRAGMA mmap_size = 268435456;`).
   - **Micro-Batch Async Writer:** Buffers up to 256 events or flushes every 250ms within a single transaction.

---

## 🚀 CLI Commands

### 1. Start the Telemetry Daemon
Runs in the background, listening to kernel and system interactions:
```bash
dockn daemon
```
*Options:*
- `--workspace <PATH>`: Monitor a specific workspace (default: current directory).
- `--engine <auto|ebpf|universal>`: Force a specific telemetry engine.

### 2. Generate Personal Subconscious Report
Fetches the intelligence summary across your activity history in milliseconds:
```bash
# Pretty terminal report (default: past 24 hours)
dockn report

# Past 8 hours
dockn report --hours 8

# Machine-readable JSON output for AI / agent pipelines
dockn report --format json
```

### 3. Identify Execution Bottlenecks
Finds the slowest compilers, test runs, scripts, and build tasks:
```bash
dockn bottlenecks --min-ms 100
```

### 4. Live Telemetry Stream
Live terminal stream of events as they hit the kernel / system:
```bash
dockn watch
# Filter by process, network, or file events
dockn watch --filter process
```

### 5. Check Database Status
```bash
dockn status
```

---

## 🛠️ Build & Installation

Built with modern Rust:
```bash
# Debug build
cargo build

# Optimized release binary
cargo build --release

# Binary is located at
./target/release/dockN --help
```
