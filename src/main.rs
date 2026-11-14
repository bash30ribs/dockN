mod analytics;
mod app_name;
mod cli;
mod demo_seed;
mod engine;
mod focus;
mod live;
mod model;
mod project;
mod report;
mod storage;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::analytics::AnalyticsEngine;
use crate::cli::{Cli, Commands, EngineChoice, OutputFormat};
use crate::engine::{EngineManager, EngineType};
use crate::model::*;
use crate::report::ReportPrinter;
use crate::storage::Storage;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize structured logging for daemon/watch
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let db_path = cli.db.unwrap_or_else(Storage::default_db_path);

    match cli.command {
        Commands::Daemon { workspace, engine } => {
            println!(
                "{}",
                "🧠 dockN: Subconscious Daemon Initializing...".bold().bright_cyan()
            );
            println!("  Database target: {:?}", db_path);
            println!("  Workspace:       {:?}", workspace);

            let storage = Storage::open(&db_path)?;
            let (tx, rx) = tokio::sync::mpsc::channel::<TelemetryEvent>(1024);
            let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

            // Spawn high-performance WAL micro-batch writer
            let writer_handle = storage.spawn_writer(rx, shutdown_rx.clone());

            let preferred_engine = match engine {
                EngineChoice::Auto => None,
                EngineChoice::Ebpf => Some(EngineType::LinuxEbpf),
                EngineChoice::Universal => Some(EngineType::UniversalAdaptive),
            };

            let engine_mgr = EngineManager::new(preferred_engine, workspace);

            // Run until SIGINT (Ctrl+C)
            let engine_handle = tokio::spawn(async move {
                let _ = engine_mgr.start(tx, shutdown_rx).await;
            });

            println!(
                "{}",
                "✔ Subconscious Telemetry Daemon is running. Press Ctrl+C to stop."
                    .bright_green()
            );

            tokio::signal::ctrl_c().await?;
            println!("\nShutdown signal received. Stopping probes and flushing SQLite transactions...");

            let _ = shutdown_tx.send(true);
            let _ = engine_handle.await;
            let _ = writer_handle.await;

            println!("{}", "✔ dockN daemon stopped cleanly.".bright_green());
        }

        Commands::Report { hours, format } => {
            let storage = Storage::open(&db_path)?;
            let report = AnalyticsEngine::compute_report(&storage, hours)?;

            match format {
                OutputFormat::Text => ReportPrinter::print_terminal_report(&report),
                OutputFormat::Json => ReportPrinter::print_json_report(&report)?,
            }
        }

        Commands::Context { hours, json } => {
            let storage = Storage::open(&db_path)?;
            let ai_context = AnalyticsEngine::compute_ai_context(&storage, hours)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&ai_context)?);
            } else {
                println!("{}", ai_context.standup_markdown);
            }
        }

        Commands::Audit { hours } => {
            let storage = Storage::open(&db_path)?;
            let now_us = chrono::Utc::now().timestamp_micros();
            let since_us = if hours > 0 {
                now_us - (hours as i64 * 3600 * 1_000_000)
            } else {
                0
            };

            let audit = storage.get_anomalies(since_us)?;

            println!();
            println!(
                "{}",
                "🔍 dockN: Subconscious System Health & Thrashing Audit"
                    .bold()
                    .bright_cyan()
            );
            println!(
                "{}",
                "────────────────────────────────────────────────────────────────────────────"
                    .bright_black()
            );

            let score_color = if audit.system_health_score >= 85.0 {
                format!("{:.0}% (HEALTHY)", audit.system_health_score).bright_green()
            } else if audit.system_health_score >= 60.0 {
                format!("{:.0}% (DEGRADED)", audit.system_health_score).bright_yellow()
            } else {
                format!("{:.0}% (THRASHING DETECTED)", audit.system_health_score).bright_red()
            };

            println!("  Overall System Health: {}", score_color.bold());
            println!("  Anomalies Flagged:     {}", audit.anomalies.len().to_string().bold());
            println!();

            if audit.anomalies.is_empty() {
                println!(
                    "  {}",
                    "✔ Zero file thrashing, loop spin, or network reconnect storms detected."
                        .bright_green()
                );
            } else {
                for (idx, anomaly) in audit.anomalies.iter().enumerate() {
                    let sev_badge = match anomaly.severity.as_str() {
                        "HIGH" => "[HIGH]".bold().bright_red(),
                        "MEDIUM" => "[MED]".bold().bright_yellow(),
                        _ => "[LOW]".bright_blue(),
                    };

                    println!(
                        "  {}. {} {:<26} {}",
                        idx + 1,
                        sev_badge,
                        anomaly.category.bright_white().bold(),
                        anomaly.metric.bright_yellow()
                    );
                    println!("     Target: {}", anomaly.target.bright_cyan());
                    println!("     Action: {}", anomaly.recommendation.bright_white());
                    println!();
                }
            }
            println!(
                "{}",
                "────────────────────────────────────────────────────────────────────────────"
                    .bright_black()
            );
            println!();
        }

        Commands::Projects { hours } => {
            let storage = Storage::open(&db_path)?;
            let now_us = chrono::Utc::now().timestamp_micros();
            let since_us = if hours > 0 {
                now_us - (hours as i64 * 3600 * 1_000_000)
            } else {
                0
            };

            let projects = storage.get_project_summaries(since_us)?;

            println!();
            println!(
                "{}",
                "📂 dockN: Developer Project Attribution & Time Breakdown"
                    .bold()
                    .bright_cyan()
            );
            println!(
                "{}",
                "────────────────────────────────────────────────────────────────────────────"
                    .bright_black()
            );

            if projects.is_empty() {
                println!("  {}", "No project telemetry recorded in this timeframe.".bright_black());
            } else {
                println!(
                    "  {:<24}  {:<12}  {:<8}  {:<14}  {:<14}",
                    "PROJECT NAME".bright_black(),
                    "ACTIVE TIME".bright_black(),
                    "SHARE".bright_black(),
                    "FILE WRITES".bright_black(),
                    "PROCESS CALLS".bright_black()
                );
                for p in projects {
                    let hrs = p.total_seconds as f64 / 3600.0;
                    let time_str = if hrs >= 1.0 {
                        format!("{:.1} hours", hrs)
                    } else {
                        format!("{} mins", p.total_seconds / 60)
                    };

                    println!(
                        "  {:<24}  {:<12}  {:>5.1}%   {:<14}  {:<14}",
                        p.name.bright_white().bold(),
                        time_str.bright_yellow(),
                        p.percentage,
                        p.file_mutations,
                        p.process_invocations
                    );
                }
            }
            println!(
                "{}",
                "────────────────────────────────────────────────────────────────────────────"
                    .bright_black()
            );
            println!();
        }

        Commands::Timeline { limit } => {
            let storage = Storage::open(&db_path)?;
            let entries = storage.get_timeline(limit)?;

            println!();
            println!(
                "{}",
                "⏱️ dockN: Subconscious Chronological Event Replay"
                    .bold()
                    .bright_cyan()
            );
            println!(
                "{}",
                "────────────────────────────────────────────────────────────────────────────"
                    .bright_black()
            );

            if entries.is_empty() {
                println!("  {}", "No recent events recorded in database.".bright_black());
            } else {
                for e in entries {
                    println!("  {}", e.bright_white());
                }
            }
            println!(
                "{}",
                "────────────────────────────────────────────────────────────────────────────"
                    .bright_black()
            );
            println!();
        }

        Commands::Live => {
            let storage = Storage::open(&db_path)?;
            live::run_live_radar(storage).await?;
        }

        Commands::Bottlenecks { min_ms, limit } => {
            let storage = Storage::open(&db_path)?;
            let bottlenecks = storage.get_bottlenecks(min_ms, limit)?;

            println!();
            println!(
                "{}",
                "⚡ dockN: Top Tool Execution Bottlenecks"
                    .bold()
                    .bright_yellow()
            );
            println!(
                "{}",
                "────────────────────────────────────────────────────────────────────────────"
                    .bright_black()
            );

            if bottlenecks.is_empty() {
                println!(
                    "  {}",
                    format!("No tools with duration >= {}ms found in database.", min_ms).bright_green()
                );
            } else {
                println!(
                    "  {:<26}  {:<16}  {:<8}  {:<12}  {:<12}  {:<10}",
                    "APPLICATION / COMMAND".bright_black(),
                    "PACKAGE".bright_black(),
                    "CALLS".bright_black(),
                    "AVG DURATION".bright_black(),
                    "MAX DURATION".bright_black(),
                    "CATEGORY".bright_black()
                );
                for b in bottlenecks {
                    let max_str = if b.max_duration_ms > 1000 {
                        format!("{:.2}s", b.max_duration_ms as f64 / 1000.0).bright_red()
                    } else {
                        format!("{}ms", b.max_duration_ms).bright_yellow()
                    };

                    println!(
                        "  {:<26}  {:<16}  {:<8}  {:<12.1}ms {:<12}  {:<10}",
                        b.app_name.bright_cyan().bold(),
                        b.package_name.bright_black(),
                        b.invocations,
                        b.avg_duration_ms,
                        max_str,
                        b.context_tag.bright_black()
                    );
                }
            }
            println!();
        }

        Commands::Watch { workspace, filter } => {
            println!(
                "{}",
                "👀 Streaming live subconscious events (Ctrl+C to quit)..."
                    .bold()
                    .bright_cyan()
            );

            let (tx, mut rx) = tokio::sync::mpsc::channel::<TelemetryEvent>(256);
            let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

            let engine_mgr = EngineManager::new(None, workspace);
            tokio::spawn(async move {
                let _ = engine_mgr.start(tx, shutdown_rx).await;
            });

            tokio::spawn(async move {
                let _ = tokio::signal::ctrl_c().await;
                let _ = shutdown_tx.send(true);
            });

            let filter_lower = filter.to_lowercase();

            while let Some(event) = rx.recv().await {
                let ts = chrono::Utc::now().format("%H:%M:%S%.3f");
                match event {
                    TelemetryEvent::Process(p) => {
                        if filter_lower == "all" || filter_lower == "process" {
                            let tag = match p.event_type {
                                ProcessEventType::Spawn => "[SPAWN]".bright_green(),
                                ProcessEventType::Exit => "[EXIT]".bright_red(),
                                ProcessEventType::Active => "[ACTIVE]".bright_blue(),
                            };
                            let dur_str = p
                                .duration_ms
                                .map(|d| format!(" ({}ms)", d))
                                .unwrap_or_default();
                            let display_app = p.app_name.as_deref().unwrap_or(&p.comm);
                            let pkg = p.package_name.as_deref().unwrap_or(&p.comm);
                            let proj_badge = p.project_name.as_deref().map(|pr| format!("[{}] ", pr)).unwrap_or_default();
                            println!(
                                "{} {} {}{:<24} [{}] pid={:<6} {}{}",
                                ts.to_string().bright_black(),
                                tag,
                                proj_badge.bright_cyan(),
                                display_app.bright_white().bold(),
                                pkg.bright_black(),
                                p.pid,
                                p.cmdline.unwrap_or_default().bright_black(),
                                dur_str.bright_yellow()
                            );
                        }
                    }
                    TelemetryEvent::Network(n) => {
                        if filter_lower == "all" || filter_lower == "network" {
                            println!(
                                "{} {} {:<14} -> {}:{} ({})",
                                ts.to_string().bright_black(),
                                "[NET]".bright_magenta(),
                                n.comm.bright_white(),
                                n.dst_addr.bright_cyan(),
                                n.dst_port,
                                n.protocol
                            );
                        }
                    }
                    TelemetryEvent::File(f) => {
                        if filter_lower == "all" || filter_lower == "file" {
                            let proj_badge = f.project_name.as_deref().map(|pr| format!("[{}] ", pr)).unwrap_or_default();
                            println!(
                                "{} {} {:<7} {}{}",
                                ts.to_string().bright_black(),
                                "[FILE]".bright_yellow(),
                                f.op_type.as_str().bright_cyan(),
                                proj_badge.bright_cyan(),
                                f.file_path.bright_white()
                            );
                        }
                    }
                    TelemetryEvent::Focus(fc) => {
                        if filter_lower == "all" || filter_lower == "focus" {
                            let proj_badge = fc.project_name.as_deref().map(|pr| format!("[{}] ", pr)).unwrap_or_default();
                            let dur_str = fc.duration_ms.map(|d| format!(" ({:.1}s)", d as f64 / 1000.0)).unwrap_or_default();
                            println!(
                                "{} {} {}{:<20} | {}{}",
                                ts.to_string().bright_black(),
                                "[FOCUS]".bold().bright_green(),
                                proj_badge.bright_cyan(),
                                fc.window_class.bright_white().bold(),
                                fc.window_title.unwrap_or_default().bright_black(),
                                dur_str.bright_yellow()
                            );
                        }
                    }
                }
            }
        }

        Commands::Status => {
            let storage = Storage::open(&db_path)?;
            let status = storage.get_status()?;

            println!();
            println!(
                "{}",
                "📊 dockN: Digital Subconscious Database Status"
                    .bold()
                    .bright_cyan()
            );
            println!(
                "{}",
                "────────────────────────────────────────────────────────────────────────────"
                    .bright_black()
            );
            println!("  Path:                 {:?}", status.db_path);
            println!(
                "  Size:                 {:.2} MB",
                status.file_size_bytes as f64 / (1024.0 * 1024.0)
            );
            println!(
                "  Process Events:       {}",
                status.total_process_events.to_string().bright_green()
            );
            println!(
                "  Network Events:       {}",
                status.total_network_events.to_string().bright_magenta()
            );
            println!(
                "  File Events:          {}",
                status.total_file_events.to_string().bright_yellow()
            );
            println!(
                "  Focus Events:         {}",
                status.total_focus_events.to_string().bright_cyan()
            );
            println!(
                "  Total Telemetry:      {}",
                (status.total_process_events + status.total_network_events + status.total_file_events + status.total_focus_events)
                    .to_string()
                    .bold()
                    .bright_white()
            );

            if let (Some(first), Some(last)) = (status.first_event_us, status.last_event_us) {
                let first_dt = chrono::DateTime::from_timestamp_micros(first)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                let last_dt = chrono::DateTime::from_timestamp_micros(last)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                println!("  First Captured Event: {}", first_dt.bright_black());
                println!("  Latest Captured Event:{}", last_dt.bright_black());
            }
            println!();
        }

        Commands::SeedDemo => {
            let storage = Storage::open(&db_path)?;
            demo_seed::seed_sample_data(&storage).await?;
            println!(
                "{}",
                "✔ Seeded realistic subconscious telemetry data into SQLite successfully!"
                    .bright_green()
            );
            println!(
                "  Run `{}` to view the full analytics report.",
                "dockn report".bold().bright_yellow()
            );
            println!(
                "  Run `{}` to check latency rankings.",
                "dockn bottlenecks".bold().bright_yellow()
            );
        }
    }

    Ok(())
}
