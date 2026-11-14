use anyhow::Result;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tokio::sync::{mpsc, watch};
use tracing::{error, info, warn};

use crate::model::*;
use crate::project::ProjectDetector;

pub struct UniversalEngine {
    workspace_path: PathBuf,
    project_detector: std::sync::Arc<ProjectDetector>,
}

impl UniversalEngine {
    pub fn new<P: AsRef<Path>>(workspace_path: P) -> Self {
        Self {
            workspace_path: workspace_path.as_ref().to_path_buf(),
            project_detector: ProjectDetector::get(),
        }
    }

    pub async fn run(
        &mut self,
        tx: mpsc::Sender<TelemetryEvent>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> Result<()> {
        info!("Starting Universal Telemetry Engine for workspace: {:?}", self.workspace_path);

        // 1. Start File System Watcher
        let (file_tx, mut file_rx) = mpsc::channel::<TelemetryEvent>(256);
        let _watcher = self.spawn_file_watcher(file_tx);

        // 2. Start Process and Network Polling Loop
        let mut sys = System::new()?;
        // pid -> (comm, app_name, package_name, start_time, tag, project_name)
        let mut tracked_processes: HashMap<u32, (String, String, String, Instant, Option<String>, Option<String>)> = HashMap::new();
        let mut poll_interval = tokio::time::interval(Duration::from_millis(500));

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("Universal Telemetry Engine shutting down.");
                        break;
                    }
                }
                Some(file_event) = file_rx.recv() => {
                    let _ = tx.send(file_event).await;
                }
                _ = poll_interval.tick() => {
                    // Refresh processes
                    sys.refresh_processes_specifics(
                        ProcessesToUpdate::All,
                        true,
                        ProcessRefreshKind::nothing()
                            .with_cpu()
                            .with_memory()
                            .with_exe(UpdateKind::OnlyIfNotSet)
                            .with_cmd(UpdateKind::OnlyIfNotSet)
                            .with_cwd(UpdateKind::OnlyIfNotSet),
                    );

                    let now_us = chrono::Utc::now().timestamp_micros();
                    let current_pids: HashMap<u32, ()> = sys.processes().iter().map(|(pid, _)| (pid.as_u32(), ())).collect();

                    // Detect exited processes
                    let exited_pids: Vec<u32> = tracked_processes
                        .keys()
                        .filter(|pid| !current_pids.contains_key(pid))
                        .copied()
                        .collect();

                    for pid in exited_pids {
                        if let Some((comm, app_name, package_name, start_time, tag, project)) = tracked_processes.remove(&pid) {
                            let duration_ms = start_time.elapsed().as_millis() as u64;
                            let exit_event = ProcessEvent {
                                timestamp_us: now_us,
                                event_type: ProcessEventType::Exit,
                                pid,
                                ppid: None,
                                comm,
                                app_name: Some(app_name),
                                package_name: Some(package_name),
                                cmdline: None,
                                cwd: None,
                                cpu_pct: 0.0,
                                memory_bytes: 0,
                                duration_ms: Some(duration_ms),
                                exit_code: Some(0),
                                context_tag: tag,
                                project_name: project,
                            };
                            let _ = tx.send(TelemetryEvent::Process(exit_event)).await;
                        }
                    }

                    // Detect new & active processes
                    for (pid_obj, proc) in sys.processes() {
                        let pid = pid_obj.as_u32();
                        let comm_raw = proc.name().to_string_lossy().to_string();

                        // Filter out kernel worker threads and low-value OS noise
                        if is_system_noise(&comm_raw) {
                            continue;
                        }

                        let exe_path = proc.exe();
                        let cmd_vec: Vec<String> = proc.cmd().iter().map(|s| s.to_string_lossy().to_string()).collect();
                        let app_info = crate::app_name::AppResolver::get().resolve(&comm_raw, exe_path, &cmd_vec);

                        let comm = app_info.package_name.clone();
                        let app_name = app_info.display_name.clone();
                        let package_name = app_info.package_name.clone();
                        let tag = app_info.category.clone();

                        let cmdline = if !proc.cmd().is_empty() {
                            Some(cmd_vec.join(" "))
                        } else {
                            None
                        };
                        let cwd = proc.cwd().map(|p| p.to_string_lossy().to_string());
                        let project = proc.cwd().and_then(|p| self.project_detector.resolve_project_from_path(p));

                        let cpu_pct = proc.cpu_usage();
                        let memory_bytes = proc.memory();

                        if !tracked_processes.contains_key(&pid) {
                            // New process spawn
                            tracked_processes.insert(
                                pid,
                                (comm.clone(), app_name.clone(), package_name.clone(), Instant::now(), Some(tag.clone()), project.clone()),
                            );

                            let spawn_event = ProcessEvent {
                                timestamp_us: now_us,
                                event_type: ProcessEventType::Spawn,
                                pid,
                                ppid: proc.parent().map(|p| p.as_u32()),
                                comm: comm.clone(),
                                app_name: Some(app_name.clone()),
                                package_name: Some(package_name.clone()),
                                cmdline: cmdline.clone(),
                                cwd: cwd.clone(),
                                cpu_pct,
                                memory_bytes,
                                duration_ms: None,
                                exit_code: None,
                                context_tag: Some(tag.clone()),
                                project_name: project.clone(),
                            };
                            let _ = tx.send(TelemetryEvent::Process(spawn_event)).await;
                        } else if cpu_pct > 1.0 {
                            // High-activity process sampling
                            let active_event = ProcessEvent {
                                timestamp_us: now_us,
                                event_type: ProcessEventType::Active,
                                pid,
                                ppid: proc.parent().map(|p| p.as_u32()),
                                comm: comm.clone(),
                                app_name: Some(app_name),
                                package_name: Some(package_name),
                                cmdline,
                                cwd,
                                cpu_pct,
                                memory_bytes,
                                duration_ms: None,
                                exit_code: None,
                                context_tag: Some(tag),
                                project_name: project,
                            };
                            let _ = tx.send(TelemetryEvent::Process(active_event)).await;
                        }
                    }

                    // Poll active network connections (on Linux or fallback)
                    #[cfg(target_os = "linux")]
                    {
                        if let Ok(net_events) = poll_linux_network(now_us) {
                            for net_ev in net_events {
                                let _ = tx.send(TelemetryEvent::Network(net_ev)).await;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn spawn_file_watcher(&self, tx: mpsc::Sender<TelemetryEvent>) -> Option<RecommendedWatcher> {
        let watch_dir = self.workspace_path.clone();
        if !watch_dir.exists() {
            warn!("Workspace path {:?} does not exist. Skipping file watcher.", watch_dir);
            return None;
        }

        let detector = self.project_detector.clone();

        let watcher_result = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    let now_us = chrono::Utc::now().timestamp_micros();
                    let op_type = match event.kind {
                        EventKind::Create(_) => FileOpType::Create,
                        EventKind::Modify(_) => FileOpType::Modify,
                        EventKind::Remove(_) => FileOpType::Delete,
                        EventKind::Access(_) => FileOpType::Access,
                        _ => return,
                    };

                    for path in event.paths {
                        let path_str = path.to_string_lossy().to_string();
                        // Ignore noise like .git objects, target directory build artifacts, node_modules, and dockN's own DB
                        if should_ignore_file_path(&path_str) {
                            continue;
                        }

                        let project = detector.resolve_project_from_path(&path);
                        let total_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                        let file_ev = FileEvent {
                            timestamp_us: now_us,
                            pid: std::process::id(),
                            comm: "workspace".to_string(),
                            file_path: path_str,
                            op_type,
                            total_bytes,
                            latency_us: 150, // Microsecond I/O approximation
                            is_workspace: true,
                            project_name: project,
                        };
                        let _ = tx.blocking_send(TelemetryEvent::File(file_ev));
                    }
                }
            },
            Config::default(),
        );

        match watcher_result {
            Ok(mut watcher) => {
                if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::Recursive) {
                    error!("Failed to watch workspace {:?}: {:?}", watch_dir, e);
                    None
                } else {
                    info!("File watcher actively monitoring {:?}", watch_dir);
                    Some(watcher)
                }
            }
            Err(e) => {
                error!("Failed to initialize recommended watcher: {:?}", e);
                None
            }
        }
    }
}

fn is_system_noise(comm: &str) -> bool {
    let lower = comm.to_lowercase();
    lower.starts_with("kworker")
        || lower.starts_with("systemd")
        || lower.contains("dbus")
        || lower.contains("pipewire")
        || lower.contains("pulseaudio")
        || lower.contains("journald")
        || lower.contains("udevd")
        || lower.contains("polkit")
        || lower.contains("irq/")
}

fn should_ignore_file_path(path: &str) -> bool {
    path.contains("dockn")
        || path.contains("subconscious")
        || path.contains(".db")
        || path.contains(".db-wal")
        || path.contains(".db-shm")
        || path.contains(".gemini")
        || path.contains("/.git/")
        || path.contains("/target/")
        || path.contains("/node_modules/")
        || path.contains("/.cache/")
        || path.ends_with(".tmp")
        || path.ends_with('~')
}

#[cfg(target_os = "linux")]
fn poll_linux_network(now_us: i64) -> Result<Vec<NetworkEvent>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let mut events = Vec::new();
    let paths = ["/proc/net/tcp", "/proc/net/tcp6"];

    for p in paths {
        if let Ok(file) = File::open(p) {
            let reader = BufReader::new(file);
            for line in reader.lines().skip(1).take(20) {
                if let Ok(l) = line {
                    let parts: Vec<&str> = l.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let state = parts[3];
                        // 01 is TCP_ESTABLISHED
                        if state == "01" {
                            let rem_addr_hex = parts[2];
                            if let Some((ip, port)) = parse_hex_socket_addr(rem_addr_hex) {
                                if !ip.starts_with("127.0.0.1") && !ip.starts_with("0.0.0.0") {
                                    events.push(NetworkEvent {
                                        timestamp_us: now_us,
                                        pid: 0,
                                        comm: "network-socket".to_string(),
                                        protocol: "TCP".to_string(),
                                        src_addr: "local".to_string(),
                                        dst_addr: ip,
                                        dst_port: port,
                                        domain: None,
                                        bytes_sent: 0,
                                        bytes_recv: 0,
                                        duration_ms: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(events)
}

#[cfg(target_os = "linux")]
fn parse_hex_socket_addr(hex: &str) -> Option<(String, u16)> {
    let mut split = hex.split(':');
    let ip_hex = split.next()?;
    let port_hex = split.next()?;

    let port = u16::from_str_radix(port_hex, 16).ok()?;
    if ip_hex.len() == 8 {
        // IPv4 in little endian
        let num = u32::from_str_radix(ip_hex, 16).ok()?;
        let b1 = (num & 0xFF) as u8;
        let b2 = ((num >> 8) & 0xFF) as u8;
        let b3 = ((num >> 16) & 0xFF) as u8;
        let b4 = ((num >> 24) & 0xFF) as u8;
        Some((format!("{}.{}.{}.{}", b1, b2, b3, b4), port))
    } else {
        Some((ip_hex.to_string(), port))
    }
}
