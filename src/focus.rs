use anyhow::Result;
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{mpsc, watch};
use tracing::{info, warn};

use crate::model::*;
use crate::project::ProjectDetector;

pub struct FocusTracker {
    project_detector: std::sync::Arc<ProjectDetector>,
}

impl FocusTracker {
    pub fn new() -> Self {
        Self {
            project_detector: ProjectDetector::get(),
        }
    }

    pub async fn run(
        &self,
        tx: mpsc::Sender<TelemetryEvent>,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Result<()> {
        let hypr_sig = env::var("HYPRLAND_INSTANCE_SIGNATURE");
        let xdg_runtime = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::getuid() }));

        if let Ok(sig) = hypr_sig {
            let socket_path = PathBuf::from(&xdg_runtime)
                .join("hypr")
                .join(&sig)
                .join(".socket2.sock");

            if socket_path.exists() {
                info!("Hyprland compositor detected. Attaching real-time focus probe to {:?}", socket_path);
                if let Err(e) = self.stream_hyprland_events(&socket_path, tx.clone(), shutdown_rx.clone()).await {
                    warn!("Hyprland focus socket closed or error: {:?}. Falling back to polling mode.", e);
                } else {
                    return Ok(());
                }
            }
        }

        // Generic fallback polling loop for other environments or if socket failed
        info!("Starting generic active window polling probe.");
        self.run_fallback_polling(tx, shutdown_rx).await
    }

    async fn stream_hyprland_events(
        &self,
        socket_path: &PathBuf,
        tx: mpsc::Sender<TelemetryEvent>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> Result<()> {
        let stream = UnixStream::connect(socket_path).await?;
        let reader = BufReader::new(stream);
        let mut lines = reader.lines();

        let mut current_class = String::new();
        let mut current_title = String::new();
        let mut current_workspace = String::from("1");
        let mut focus_start = Instant::now();

        // Get initial window state
        if let Some((cls, title, ws)) = query_hyprctl_activewindow() {
            current_class = cls;
            current_title = title;
            current_workspace = ws;
            focus_start = Instant::now();
        }

        let mut heartbeat = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        // Flush final window event
                        let dur_ms = focus_start.elapsed().as_millis() as u64;
                        if dur_ms >= 500 && !current_class.is_empty() {
                            let now_us = chrono::Utc::now().timestamp_micros();
                            let project = self.project_detector.infer_from_window_title(&current_title);
                            let is_deep = dur_ms >= (15 * 60 * 1000);
                            let ev = FocusEvent {
                                timestamp_us: now_us,
                                window_class: current_class.clone(),
                                window_title: Some(current_title.clone()),
                                workspace: Some(current_workspace.clone()),
                                duration_ms: Some(dur_ms),
                                project_name: project,
                                is_deep_work: is_deep,
                            };
                            let _ = tx.send(TelemetryEvent::Focus(ev)).await;
                        }
                        break;
                    }
                }
                _ = heartbeat.tick() => {
                    // Periodic heartbeat for sustained focus without window switching
                    let dur_ms = focus_start.elapsed().as_millis() as u64;
                    if dur_ms >= 30_000 && !current_class.is_empty() {
                        let now_us = chrono::Utc::now().timestamp_micros();
                        let project = self.project_detector.infer_from_window_title(&current_title);
                        let is_deep = dur_ms >= (15 * 60 * 1000);
                        let ev = FocusEvent {
                            timestamp_us: now_us,
                            window_class: current_class.clone(),
                            window_title: Some(current_title.clone()),
                            workspace: Some(current_workspace.clone()),
                            duration_ms: Some(dur_ms),
                            project_name: project,
                            is_deep_work: is_deep,
                        };
                        let _ = tx.send(TelemetryEvent::Focus(ev)).await;
                    }
                }
                line_res = lines.next_line() => {
                    match line_res {
                        Ok(Some(line)) => {
                            if line.starts_with("activewindow>>") {
                                let payload = &line["activewindow>>".len()..];
                                let mut parts = payload.splitn(2, ',');
                                let new_class = parts.next().unwrap_or("").trim().to_string();
                                let new_title = parts.next().unwrap_or("").trim().to_string();

                                if new_class != current_class || new_title != current_title {
                                    let dur_ms = focus_start.elapsed().as_millis() as u64;
                                    if dur_ms >= 500 && !current_class.is_empty() {
                                        let now_us = chrono::Utc::now().timestamp_micros();
                                        let project = self.project_detector.infer_from_window_title(&current_title);
                                        let is_deep = dur_ms >= (15 * 60 * 1000);
                                        let ev = FocusEvent {
                                            timestamp_us: now_us,
                                            window_class: current_class.clone(),
                                            window_title: Some(current_title.clone()),
                                            workspace: Some(current_workspace.clone()),
                                            duration_ms: Some(dur_ms),
                                            project_name: project,
                                            is_deep_work: is_deep,
                                        };
                                        let _ = tx.send(TelemetryEvent::Focus(ev)).await;
                                    }

                                    current_class = new_class;
                                    current_title = new_title;
                                    focus_start = Instant::now();
                                }
                            } else if line.starts_with("workspace>>") {
                                let ws = line["workspace>>".len()..].trim().to_string();
                                current_workspace = ws;
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            warn!("Error reading Hyprland focus stream: {:?}", e);
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn run_fallback_polling(
        &self,
        tx: mpsc::Sender<TelemetryEvent>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let mut last_class = String::new();
        let mut focus_start = Instant::now();

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                }
                _ = interval.tick() => {
                    if let Some((cls, title, ws)) = query_hyprctl_activewindow() {
                        if cls != last_class {
                            let dur_ms = focus_start.elapsed().as_millis() as u64;
                            if dur_ms >= 500 && !last_class.is_empty() {
                                let now_us = chrono::Utc::now().timestamp_micros();
                                let project = self.project_detector.infer_from_window_title(&title);
                                let is_deep = dur_ms >= (15 * 60 * 1000);
                                let ev = FocusEvent {
                                    timestamp_us: now_us,
                                    window_class: last_class.clone(),
                                    window_title: Some(title.clone()),
                                    workspace: Some(ws.clone()),
                                    duration_ms: Some(dur_ms),
                                    project_name: project,
                                    is_deep_work: is_deep,
                                };
                                let _ = tx.send(TelemetryEvent::Focus(ev)).await;
                            }
                            last_class = cls;
                            focus_start = Instant::now();
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

fn query_hyprctl_activewindow() -> Option<(String, String, String)> {
    let output = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let val: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let class = val.get("class")?.as_str()?.to_string();
    let title = val.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string();
    let ws = val.get("workspace")
        .and_then(|w| w.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("1")
        .to_string();

    Some((class, title, ws))
}
