use anyhow::Result;
use colored::Colorize;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

use crate::project::ProjectDetector;
use crate::storage::Storage;

pub async fn run_live_radar(storage: Storage) -> Result<()> {
    println!("{}", "Initializing Subconscious Live Radar...".bright_cyan());
    let detector = ProjectDetector::get();

    // Loop until Ctrl+C
    loop {
        // Query active window via hyprctl
        let (window_class, window_title, workspace) = query_active_window();
        let project = if !window_title.is_empty() {
            detector.infer_from_window_title(&window_title).unwrap_or_else(|| "General System".to_string())
        } else {
            "General System".to_string()
        };

        let status = storage.get_status().ok();
        let total_telemetry = status.as_ref().map(|s| {
            s.total_process_events + s.total_network_events + s.total_file_events + s.total_focus_events
        }).unwrap_or(0);
        let db_size_mb = status.as_ref().map(|s| s.file_size_bytes as f64 / (1024.0 * 1024.0)).unwrap_or(0.0);

        let timeline = storage.get_timeline(8).unwrap_or_default();

        // Clear terminal screen and reset cursor
        print!("\x1B[2J\x1B[1;1H");

        println!(
            "{}",
            "╔════════════════════════════════════════════════════════════════════════════════╗"
                .bright_cyan()
        );
        println!(
            "{}   {}    {}",
            "║".bright_cyan(),
            "🧠 dockN: Subconscious Live Radar — Kernel & Focus Telemetry".bold().bright_white(),
            "║".bright_cyan()
        );
        println!(
            "{}",
            "╚════════════════════════════════════════════════════════════════════════════════╝"
                .bright_cyan()
        );
        println!("  Press {} to exit live view.", "Ctrl+C".bold().bright_yellow());
        println!();

        // Active Focus & Context Section
        println!("{}", "── Active Foreground Attention ────────────────────────────────────────────".bright_black());
        println!(
            "  {:<18} {}",
            "Application:".bright_black(),
            window_class.bold().bright_green()
        );
        println!(
            "  {:<18} {}",
            "Active Window:".bright_black(),
            window_title.bright_white()
        );
        println!(
            "  {:<18} {}  (Workspace: {})",
            "Project Root:".bright_black(),
            project.bold().bright_yellow(),
            workspace.bright_cyan()
        );
        println!();

        // Subconscious Telemetry Metrics
        println!("{}", "── Subconscious Telemetry Stream ──────────────────────────────────────────".bright_black());
        println!(
            "  {:<18} {} MB",
            "Database Size:".bright_black(),
            format!("{:.2}", db_size_mb).bright_magenta()
        );
        println!(
            "  {:<18} {} events captured",
            "Total Ingested:".bright_black(),
            format!("{}", total_telemetry).bright_green().bold()
        );
        println!();

        // Live Event Ticker
        println!("{}", "── Real-Time Subconscious Event Ticker ────────────────────────────────────".bright_black());
        if timeline.is_empty() {
            println!("  {}", "Waiting for incoming kernel & process events...".bright_black());
        } else {
            for entry in timeline {
                println!("  {}", entry.bright_white());
            }
        }

        println!();
        println!(
            "{}",
            "────────────────────────────────────────────────────────────────────────────"
                .bright_black()
        );

        sleep(Duration::from_millis(600)).await;
    }
}

fn query_active_window() -> (String, String, String) {
    if let Ok(output) = Command::new("hyprctl").args(["activewindow", "-j"]).output() {
        if output.status.success() {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                let class = val.get("class").and_then(|c| c.as_str()).unwrap_or("unknown").to_string();
                let title = val.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string();
                let ws = val.get("workspace")
                    .and_then(|w| w.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("1")
                    .to_string();
                return (class, title, ws);
            }
        }
    }
    ("idle".to_string(), "Desktop Idle".to_string(), "1".to_string())
}
