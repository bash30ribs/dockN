use colored::Colorize;
use crate::model::*;

pub struct ReportPrinter;

impl ReportPrinter {
    pub fn print_terminal_report(report: &SubconsciousReport) {
        println!();
        println!(
            "{}",
            "╔════════════════════════════════════════════════════════════════════════════════╗"
                .bright_blue()
        );
        println!(
            "{}  {}  {}",
            "║".bright_blue(),
            "🧠 dockN — Pillar 1: The Subconscious (Personal Analytics Engine)"
                .bold()
                .bright_white(),
            "║".bright_blue()
        );
        println!(
            "{}",
            "╚════════════════════════════════════════════════════════════════════════════════╝"
                .bright_blue()
        );
        println!();

        // 1. Meta telemetry info
        let timeframe_str = if report.timeframe_hours == 0 {
            "All-Time History".to_string()
        } else {
            format!("Past {} Hours", report.timeframe_hours)
        };

        println!(
            " {}  {}  |  {}  {}  |  {}  {}",
            "Timeframe:".bright_black(),
            timeframe_str.bright_cyan().bold(),
            "Events Captured:".bright_black(),
            format!("{}", report.total_events).bright_green().bold(),
            "Context Switch Rate:".bright_black(),
            format!("{:.1}/hr", report.context_switches_per_hour).bright_yellow().bold()
        );
        println!();

        // 2. Deep Work Score & Progress Bar
        let score = report.deep_work_score;
        let score_label = if score >= 80.0 {
            "Deep Flow State (Minimal Interruption)".bright_green().bold()
        } else if score >= 60.0 {
            "Sustained Productive Focus".green()
        } else if score >= 40.0 {
            "Moderate Context Switching".yellow()
        } else {
            "High Fragmentation (Frequent Distractions)".red()
        };

        let bar_width: usize = 30;
        let filled = ((score / 100.0) * bar_width as f32).round() as usize;
        let unfilled = bar_width.saturating_sub(filled);
        let bar_str = format!(
            "{}{}",
            "█".repeat(filled).bright_green(),
            "░".repeat(unfilled).bright_black()
        );

        println!("{}", "── Deep Work & Cognitive Flow Index ───────────────────────────────────────".bright_black());
        println!(
            "  [{}] {:>5.1}%  ({})",
            bar_str,
            score,
            score_label
        );
        println!();

        // 3. Project Attribution Breakdown
        if !report.project_breakdown.is_empty() {
            println!("{}", "── Project & Workspace Attribution ────────────────────────────────────────".bright_black());
            for proj in &report.project_breakdown {
                let bar_len = ((proj.percentage / 100.0) * 18.0).round() as usize;
                let mini_bar = format!(
                    "{}{}",
                    "■".repeat(bar_len).bright_cyan(),
                    " ".repeat(18_usize.saturating_sub(bar_len))
                );
                let hrs = proj.total_seconds as f64 / 3600.0;
                let time_str = if hrs >= 1.0 {
                    format!("{:.1}h", hrs)
                } else {
                    format!("{}m", proj.total_seconds / 60)
                };
                println!(
                    "  {:<24}  {}  {:>5.1}%  ({:>5})  [{} writes, {} procs]",
                    proj.name.bright_white().bold(),
                    mini_bar,
                    proj.percentage,
                    time_str.bright_yellow(),
                    proj.file_mutations,
                    proj.process_invocations
                );
            }
            println!();
        }

        // 4. Top Running Applications & Packages (User-centric)
        println!("{}", "── Top Running Applications & Packages ──────────────────────────────────────".bright_black());
        if report.top_applications.is_empty() {
            println!("  {}", "No applications recorded yet.".bright_black());
        } else {
            for app in &report.top_applications {
                let app_bar_len = ((app.percentage / 100.0) * 18.0).round() as usize;
                let mini_bar = format!(
                    "{}{}",
                    "■".repeat(app_bar_len).bright_cyan(),
                    " ".repeat(18_usize.saturating_sub(app_bar_len))
                );
                let title = format!("{} ({})", app.display_name, app.package_name);
                println!(
                    "  {:<36}  {}  {:>5.1}%  ({:>5}s)  [{}]",
                    title.bright_white().bold(),
                    mini_bar,
                    app.percentage,
                    app.active_seconds,
                    app.category.bright_black()
                );
            }
        }
        println!();

        // 5. Focus Category Breakdown
        println!("{}", "── Activity & Tool Categorization ──────────────────────────────────────────".bright_black());
        if report.focus_breakdown.is_empty() {
            println!("  {}", "No active categories recorded yet.".bright_black());
        } else {
            for item in &report.focus_breakdown {
                let cat_bar_len = ((item.percentage / 100.0) * 18.0).round() as usize;
                let mini_bar = format!(
                    "{}{}",
                    "■".repeat(cat_bar_len).bright_green(),
                    " ".repeat(18_usize.saturating_sub(cat_bar_len))
                );
                println!(
                    "  {:<16}  {}  {:>5.1}%  ({}s)",
                    item.category.bright_white(),
                    mini_bar,
                    item.percentage,
                    item.active_seconds
                );
            }
        }
        println!();

        // 6. Developer Tool Bottlenecks & Latency
        println!("{}", "── Developer Tool Latencies & Bottlenecks ─────────────────────────────────".bright_black());
        if report.bottlenecks.is_empty() {
            println!("  {}", "No slow commands (>100ms) recorded in this timeframe.".bright_green());
        } else {
            println!(
                "  {:<26}  {:<16}  {:<8}  {:<12}  {:<12}  {:<10}",
                "APPLICATION / COMMAND".bright_black(),
                "PACKAGE".bright_black(),
                "CALLS".bright_black(),
                "AVG DURATION".bright_black(),
                "MAX DURATION".bright_black(),
                "TOTAL TIME".bright_black()
            );
            for tool in &report.bottlenecks {
                let max_str = if tool.max_duration_ms > 1000 {
                    format!("{:.2}s", tool.max_duration_ms as f64 / 1000.0).bright_red()
                } else {
                    format!("{}ms", tool.max_duration_ms).bright_yellow()
                };

                println!(
                    "  {:<26}  {:<16}  {:<8}  {:<12.1}ms {:<12}  {:.1}s",
                    tool.app_name.bright_cyan().bold(),
                    tool.package_name.bright_black(),
                    tool.invocations,
                    tool.avg_duration_ms,
                    max_str,
                    tool.total_duration_ms as f64 / 1000.0
                );
            }
        }
        println!();

        // 7. File Churn & Hotspots
        println!("{}", "── Workspace File Churn Hotspots ──────────────────────────────────────────".bright_black());
        if report.file_hotspots.is_empty() {
            println!("  {}", "No active workspace file churn detected.".bright_black());
        } else {
            for (idx, hotspot) in report.file_hotspots.iter().take(6).enumerate() {
                let proj_badge = hotspot.project_name.as_deref().map(|p| format!("[{}] ", p)).unwrap_or_default();
                println!(
                    "  {:>2}. {}{:<44} ({} mutations)",
                    idx + 1,
                    proj_badge.bright_cyan(),
                    hotspot.file_path.bright_white(),
                    format!("{}", hotspot.change_count).bright_yellow()
                );
            }
        }
        println!();

        // 8. Network Destinations
        println!("{}", "── Active Outbound Network Destinations ───────────────────────────────────".bright_black());
        if report.network_destinations.is_empty() {
            println!("  {}", "No outbound connections captured.".bright_black());
        } else {
            for dest in &report.network_destinations {
                let comms_str = if dest.comms.is_empty() {
                    "unknown".to_string()
                } else {
                    dest.comms.join(", ")
                };
                println!(
                    "  {:<28} : {:<5} ({} connections) [{}]",
                    dest.destination.bright_cyan(),
                    dest.port,
                    dest.connection_count,
                    comms_str.bright_black()
                );
            }
        }
        println!();

        if report.anomalies_detected > 0 {
            println!("{}", "── Subconscious Health Alert ──────────────────────────────────────────────".bright_yellow());
            println!(
                "  {} {} anomalies or potential thrash loops detected. Run `{}` for diagnostics.",
                "⚠️".bright_yellow(),
                report.anomalies_detected.to_string().bold().bright_red(),
                "dockn audit".bold().bright_yellow()
            );
            println!();
        }

        println!(
            "{}",
            "────────────────────────────────────────────────────────────────────────────"
                .bright_black()
        );
        println!(
            " {} {}",
            "💡 Insight:".bright_yellow().bold(),
            "This subconscious dataset reflects your deterministic machine behavior.".bright_white()
        );
        println!();
    }

    pub fn print_json_report(report: &SubconsciousReport) -> anyhow::Result<()> {
        let json_str = serde_json::to_string_pretty(report)?;
        println!("{}", json_str);
        Ok(())
    }
}
