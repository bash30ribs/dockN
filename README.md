# dockN — Pillar 1: The Subconscious

> **Personal Analytics with eBPF, Real-Time Focus Tracking & High-Speed Adaptive Telemetry**  
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
                                            |   Desktop & Focus Tracking   |
                                            |  • Hyprland UNIX IPC Socket  |
                                            |  • Active Window & Workspace |
                                            |  • Dwell Time & Deep Work    |
                                            +--------------+---------------+
                                                           |
                                                           v
                                            +------------------------------+
                                            |  Project Attribution Engine  |
                                            |  • Automatic Git Root Walk   |
                                            |  • Cargo / Node / Py Project |
                                            |  • Zero-Overhead LRU Caching |
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
                     |    Instant CLI Report    |                      |    LLM Context & Standup |
                     |  • Deep Work Flow Score  |                      |  • dockn context         |
                     |  • Project Attribution   |                      |  • dockn audit (Thrash)  |
                     |  • Tool Latency Ranking  |                      |  • dockn live (Radar)    |
                     |  • File Churn Hotspots   |                      |  • dockn timeline        |
                     +--------------------------+                      +--------------------------+
```

---

## ⚡ Key Capabilities

1. **Dual-Engine Telemetry (Runs on Any OS & Company Setup):**
   - **Linux eBPF Mode:** Taps directly into kernel execution tracepoints (`sched_process_exec`, `sched_process_exit`, network sockets) with zero performance overhead.
   - **Universal Adaptive Mode:** Runs seamlessly without root privileges on macOS, Windows, Linux, and corporate laptops via high-frequency differential tracking and OS event notifications.
   - **Automatic Detection:** Automatically activates eBPF when available; gracefully falls back to the adaptive engine without crashing.

2. **Compositor & Active Focus Attribution (Hyprland / Desktop IPC):**
   - Connects directly to the compositor's UNIX domain socket (`.socket2.sock`) to track which window is actively receiving user input.
   - Differentiates background daemons from true human cognitive attention.
   - Accurately computes **Deep Work Index** based on uninterrupted dwell time ($\ge$ 15 minutes) vs. context-switch fragmentation.

3. **Automatic Project & Git Repository Attribution:**
   - Dynamically walks parent directory trees to group file mutations, compiler invocations, and focus sessions by repository (`dockN`, `dotfiles`, etc.).
   - Fast, thread-safe LRU caching guarantees zero disk thrashing.

4. **AI Subconscious Synthesis (`dockn context`):**
   - Generates an instant, token-dense Markdown summary of what you worked on, modified files, tool bottlenecks, and flow score—ready to feed into LLMs or pipe directly (`dockn context | agy`).

5. **Subconscious Health & Thrashing Anomaly Detector (`dockn audit`):**
   - Detects hidden background disk thrashing (>500 writes/hr), runaway status bar scripts looping tightly, and network reconnect storms.

6. **Interactive Terminal Live Radar (`dockn live`):**
   - Clean, lightweight ANSI terminal dashboard showing real-time focused app, active project, event velocity, and live event ticker.

---

## 🚀 CLI Commands

### 1. Generate Personal Subconscious Report
Fetches the intelligence summary across your activity history in milliseconds:
```bash
dockn report

# Past 8 hours
dockn report --hours 8

# Machine-readable JSON output for AI pipelines
dockn report --format json
```

### 2. Generate LLM Context / Daily Standup Briefing
Export an AI-ready briefing of what you accomplished:
```bash
dockn context --hours 4

# Structured JSON format
dockn context --hours 4 --json
```

### 3. Subconscious Health & Thrashing Audit
Scan the subconscious database for loop thrashing, rogue scripts, and socket loops:
```bash
dockn audit
```

### 4. Project Effort & Churn Breakdown
See exactly how much time and churn went into each project:
```bash
dockn projects
```

### 5. Chronological Event Replay Timeline
Replay recent window switches and process executions:
```bash
dockn timeline --limit 25
```

### 6. Interactive Subconscious Live Radar
Live terminal radar of what the subconscious is sensing right now:
```bash
dockn live
```

### 7. Identify Execution Bottlenecks
Find the slowest compilers, test runs, scripts, and build tasks:
```bash
dockn bottlenecks --min-ms 100
```

### 8. Live Event Stream
Stream raw microsecond events as they hit the kernel / compositor:
```bash
dockn watch

# Filter by focus, process, network, or file events
dockn watch --filter focus
```

### 9. Check Database Status
```bash
dockn status
```

---

## 🛠️ Build & Installation

```bash
# Compile optimized release binary
cargo build --release

# Install to user binary path
rm -f ~/.local/bin/dockn && cp target/release/dockN ~/.local/bin/dockn

# Restart background systemd user service
systemctl --user restart dockn
```
