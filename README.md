# dockN — Pillar 1: The Subconscious

[![dockN CI](https://github.com/bash30ribs/dockN/actions/workflows/ci.yml/badge.svg)](https://github.com/bash30ribs/dockN/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust: 2024](https://img.shields.io/badge/Rust-2024%20Edition-orange.svg)](https://www.rust-lang.org)

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
                     |  • File Churn Hotspots   |                      |  • dockn profile         |
                     +--------------------------+                      +--------------------------+
```

---

## ⚡ Key Capabilities

1. **Dual-Engine Telemetry (Runs on Any OS & Enterprise Setup):**
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

4. **Zero-Latency Shell Integration (ZSH & Bash):**
   - Non-blocking `preexec` and `precmd` hooks record exact terminal commands, exit codes, and millisecond execution durations into SQLite asynchronously.

5. **Resource & System Footprint Profiling (`dockn profile`):**
   - Tracks peak and average memory footprint and CPU utilization curves per application to expose memory leaks and heavy processes.

6. **Subconscious AI Synthesis (`dockn context`):**
   - Generates an instant, token-dense Markdown summary of what you worked on, modified files, tool bottlenecks, and flow score—ready to feed into LLMs or pipe directly (`dockn context | agy`).

7. **Subconscious Health & Thrashing Anomaly Detector (`dockn audit`):**
   - Detects hidden background disk thrashing (>500 writes/hr), runaway status bar scripts looping tightly, and network reconnect storms.

8. **Interactive Terminal Live Radar (`dockn live`):**
   - Clean, lightweight ANSI terminal dashboard showing real-time focused app, active project, event velocity, and live event ticker.

9. **Smart Data Retention & Compaction (`dockn prune`):**
   - Automated aging of high-frequency events with SQLite WAL compaction and disk space reclamation.

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

### 3. Resource & System Footprint Profiling
Inspect peak memory footprints and CPU spikes across applications:
```bash
dockn profile --hours 24
```

### 4. Zero-Latency Shell Integration
Add exact command execution telemetry to your shell prompt:
```bash
# For ZSH (in ~/.zshrc)
eval "$(dockn hook zsh)"

# For BASH (in ~/.bashrc)
eval "$(dockn hook bash)"
```

### 5. Export Subconscious Telemetry (CSV & JSON Lines)
Export your subconscious dataset for Pandas, DuckDB, Polars, or Jupyter:
```bash
# Export process telemetry as CSV
dockn export --table process_events --format csv --output telemetry.csv

# Export focus timeline as JSONL
dockn export --table focus_events --format jsonl --output focus.jsonl
```

### 6. Subconscious Health & Thrashing Audit
Scan the subconscious database for loop thrashing, rogue scripts, and socket loops:
```bash
dockn audit
```

### 7. Project Effort & Churn Breakdown
See exactly how much time and churn went into each project:
```bash
dockn projects
```

### 8. Chronological Event Replay Timeline
Replay recent window switches and process executions:
```bash
dockn timeline --limit 25
```

### 9. Interactive Subconscious Live Radar
Live terminal radar of what the subconscious is sensing right now:
```bash
dockn live
```

### 10. Database Pruning & Compaction
Prune events older than $N$ days and reclaim physical disk space:
```bash
dockn prune --days 14 --vacuum
```

---

## 🛠️ Build & Installation

```bash
# Compile optimized release binary
cargo build --release

# Run test suite
cargo test

# Install to user binary path
rm -f ~/.local/bin/dockn && cp target/release/dockN ~/.local/bin/dockn

# Restart background systemd user service
systemctl --user restart dockn
```
