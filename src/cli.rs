use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "dockn",
    author = "dockN Engineering",
    version = "0.2.0",
    about = "🧠 Pillar 1: The Subconscious — Personal Analytics with Kernel & System Telemetry",
    long_about = "Capture process executions, network connections, file access patterns, and latency with zero-overhead modern telemetry stored in local SQLite."
)]
pub struct Cli {
    #[arg(long, global = true, help = "Path to local SQLite subconscious database")]
    pub db: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Start the subconscious background telemetry daemon")]
    Daemon {
        #[arg(short, long, default_value = ".", help = "Workspace path to monitor")]
        workspace: PathBuf,

        #[arg(long, value_enum, default_value_t = EngineChoice::Auto, help = "Telemetry engine to use")]
        engine: EngineChoice,
    },

    #[command(about = "Generate a personal analytics report from your digital subconscious")]
    Report {
        #[arg(long, default_value_t = 24, help = "Time window in hours (0 for all-time)")]
        hours: u32,

        #[arg(short, long, value_enum, default_value_t = OutputFormat::Text, help = "Report format")]
        format: OutputFormat,
    },

    #[command(about = "Generate LLM-ready developer context & daily standup briefing")]
    Context {
        #[arg(long, default_value_t = 4, help = "Time window in hours")]
        hours: u32,

        #[arg(long, help = "Output structured JSON instead of Markdown")]
        json: bool,
    },

    #[command(about = "Audit subconscious health, file thrashing, loop spin, and socket storms")]
    Audit {
        #[arg(long, default_value_t = 24, help = "Time window in hours")]
        hours: u32,
    },

    #[command(about = "Analyze per-application CPU load and peak memory consumption footprints")]
    Profile {
        #[arg(long, default_value_t = 24, help = "Time window in hours")]
        hours: u32,
    },

    #[command(about = "Prune old telemetry and vacuum database to reclaim disk space")]
    Prune {
        #[arg(long, default_value_t = 7, help = "Prune events older than this many days")]
        days: u32,

        #[arg(long, help = "Execute full SQLite vacuum to shrink database file size on disk")]
        vacuum: bool,
    },

    #[command(about = "Export subconscious telemetry to CSV or JSON Lines for external data science")]
    Export {
        #[arg(long, default_value = "process_events", help = "Table to export (process_events, network_events, file_events, focus_events)")]
        table: String,

        #[arg(long, value_enum, default_value_t = ExportFormat::Csv, help = "Export format")]
        format: ExportFormat,

        #[arg(short, long, help = "File path to save export to (defaults to stdout)")]
        output: Option<PathBuf>,
    },

    #[command(about = "Generate shell integration hook (eval \"$(dockn hook zsh)\")")]
    Hook {
        #[arg(value_enum, help = "Shell to generate integration script for")]
        shell: ShellType,
    },

    #[command(about = "Record a single command execution directly (called by shell hook)")]
    RecordCmd {
        #[arg(long)]
        comm: String,

        #[arg(long)]
        cmdline: String,

        #[arg(long)]
        cwd: String,

        #[arg(long, default_value_t = 0)]
        duration_ms: u64,

        #[arg(long, default_value_t = 0)]
        exit_code: i32,
    },

    #[command(about = "Deep dive into developer time and file churn grouped by project")]
    Projects {
        #[arg(long, default_value_t = 24, help = "Time window in hours")]
        hours: u32,
    },

    #[command(about = "Replay chronological timeline of recent user actions and focus transitions")]
    Timeline {
        #[arg(short, long, default_value_t = 25, help = "Number of events to display")]
        limit: usize,
    },

    #[command(about = "Launch real-time interactive Subconscious Radar in terminal")]
    Live,

    #[command(about = "List system and tool bottlenecks ranked by execution latency")]
    Bottlenecks {
        #[arg(long, default_value_t = 100, help = "Minimum latency in milliseconds")]
        min_ms: u64,

        #[arg(short, long, default_value_t = 15, help = "Number of bottlenecks to list")]
        limit: usize,
    },

    #[command(about = "Stream live microsecond events from kernel & system directly to terminal")]
    Watch {
        #[arg(short, long, default_value = ".", help = "Workspace path to monitor")]
        workspace: PathBuf,

        #[arg(long, default_value = "all", help = "Filter events (process, network, file, focus, all)")]
        filter: String,
    },

    #[command(about = "Display subconscious database status, size, and telemetry count")]
    Status,

    #[command(about = "Seed realistic sample telemetry to test reports and analytical queries instantly")]
    SeedDemo,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineChoice {
    Auto,
    Ebpf,
    Universal,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Jsonl,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellType {
    Zsh,
    Bash,
}
