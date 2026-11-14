use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, watch};
use tracing::{debug, info, warn};

use crate::model::*;

pub struct EbpfEngine {
    trace_path: Option<PathBuf>,
}

impl EbpfEngine {
    pub fn new() -> Self {
        let candidates = [
            PathBuf::from("/sys/kernel/tracing"),
            PathBuf::from("/sys/kernel/debug/tracing"),
        ];

        let trace_path = candidates.into_iter().find(|p| p.exists());

        Self { trace_path }
    }

    pub fn is_available(&self) -> bool {
        if let Some(ref path) = self.trace_path {
            let sched_exec = path.join("events/sched/sched_process_exec/enable");
            sched_exec.exists()
        } else {
            false
        }
    }

    pub async fn run(
        &mut self,
        tx: mpsc::Sender<TelemetryEvent>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> Result<()> {
        let trace_dir = match &self.trace_path {
            Some(p) => p.clone(),
            None => {
                anyhow::bail!("Kernel tracing directory not found. Please ensure tracefs or debugfs is mounted.");
            }
        };

        info!("Initializing Linux Kernel Tracepoint Subsystem at {:?}", trace_dir);

        // Try to enable sched_process_exec and sched_process_exit tracepoints
        let enable_exec = trace_dir.join("events/sched/sched_process_exec/enable");
        let enable_exit = trace_dir.join("events/sched/sched_process_exit/enable");
        let trace_pipe = trace_dir.join("trace_pipe");

        let has_write_access = match (std::fs::write(&enable_exec, "1"), std::fs::write(&enable_exit, "1")) {
            (Ok(_), Ok(_)) => true,
            _ => {
                warn!("Tracepoint write access restricted. Reading active kernel trace pipe if available.");
                false
            }
        };

        if !trace_pipe.exists() {
            anyhow::bail!("Kernel trace_pipe is not accessible ({:?})", trace_pipe);
        }

        // Open trace_pipe in a blocking thread and stream events to async channel
        let pipe_file = File::open(&trace_pipe)
            .with_context(|| format!("Failed to open trace_pipe at {:?}", trace_pipe))?;
        let reader = BufReader::new(pipe_file);

        let (pipe_tx, mut pipe_rx) = mpsc::channel::<TelemetryEvent>(512);

        // Blocking reader task
        std::thread::spawn(move || {
            for line_res in reader.lines() {
                if let Ok(line) = line_res {
                    if let Some(event) = parse_tracepoint_line(&line) {
                        if pipe_tx.blocking_send(event).is_err() {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
            debug!("eBPF trace_pipe worker thread exited.");
        });

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("eBPF Telemetry Engine shutting down.");
                        if has_write_access {
                            let _ = std::fs::write(&enable_exec, "0");
                            let _ = std::fs::write(&enable_exit, "0");
                        }
                        break;
                    }
                }
                Some(event) = pipe_rx.recv() => {
                    let _ = tx.send(event).await;
                }
            }
        }

        Ok(())
    }
}

/// Parses raw kernel tracepoint output from /sys/kernel/tracing/trace_pipe
/// Example format:
/// <...>-12345 [002] d... 12345.678901: sched_process_exec: filename=/usr/bin/cargo pid=12345 old_pid=12345
/// <...>-12345 [001] d... 12345.698901: sched_process_exit: comm=cargo pid=12345 prio=120
fn parse_tracepoint_line(line: &str) -> Option<TelemetryEvent> {
    let now_us = chrono::Utc::now().timestamp_micros();

    if line.contains("sched_process_exec:") {
        // Parse execution
        let parts: Vec<&str> = line.split("sched_process_exec:").collect();
        if parts.len() < 2 {
            return None;
        }
        let payload = parts[1];
        let mut filename = String::new();
        let mut pid: u32 = 0;

        for item in payload.split_whitespace() {
            if let Some(f) = item.strip_prefix("filename=") {
                filename = f.to_string();
            } else if let Some(p) = item.strip_prefix("pid=") {
                pid = p.parse().unwrap_or(0);
            }
        }

        let comm = Path::new(&filename)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| filename.clone());

        let app_info = crate::app_name::AppResolver::get().resolve(&comm, Some(Path::new(&filename)), &[]);

        Some(TelemetryEvent::Process(ProcessEvent {
            timestamp_us: now_us,
            event_type: ProcessEventType::Spawn,
            pid,
            ppid: None,
            comm: app_info.package_name.clone(),
            app_name: Some(app_info.display_name.clone()),
            package_name: Some(app_info.package_name.clone()),
            cmdline: Some(filename),
            cwd: None,
            cpu_pct: 0.0,
            memory_bytes: 0,
            duration_ms: None,
            exit_code: None,
            context_tag: Some(app_info.category),
            project_name: None,
        }))
    } else if line.contains("sched_process_exit:") {
        let parts: Vec<&str> = line.split("sched_process_exit:").collect();
        if parts.len() < 2 {
            return None;
        }
        let payload = parts[1];
        let mut comm = String::new();
        let mut pid: u32 = 0;

        for item in payload.split_whitespace() {
            if let Some(c) = item.strip_prefix("comm=") {
                comm = c.to_string();
            } else if let Some(p) = item.strip_prefix("pid=") {
                pid = p.parse().unwrap_or(0);
            }
        }

        let app_info = crate::app_name::AppResolver::get().resolve(&comm, None, &[]);

        Some(TelemetryEvent::Process(ProcessEvent {
            timestamp_us: now_us,
            event_type: ProcessEventType::Exit,
            pid,
            ppid: None,
            comm: app_info.package_name.clone(),
            app_name: Some(app_info.display_name.clone()),
            package_name: Some(app_info.package_name.clone()),
            cmdline: None,
            cwd: None,
            cpu_pct: 0.0,
            memory_bytes: 0,
            duration_ms: None,
            exit_code: Some(0),
            context_tag: Some(app_info.category),
            project_name: None,
        }))
    } else {
        None
    }
}
