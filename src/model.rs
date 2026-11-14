use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessEventType {
    Spawn,
    Exit,
    Active,
}

#[allow(dead_code)]
impl ProcessEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Spawn => "SPAWN",
            Self::Exit => "EXIT",
            Self::Active => "ACTIVE",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "SPAWN" => Self::Spawn,
            "EXIT" => Self::Exit,
            _ => Self::Active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEvent {
    pub timestamp_us: i64,
    pub event_type: ProcessEventType,
    pub pid: u32,
    pub ppid: Option<u32>,
    pub comm: String,
    pub app_name: Option<String>,
    pub package_name: Option<String>,
    pub cmdline: Option<String>,
    pub cwd: Option<String>,
    pub cpu_pct: f32,
    pub memory_bytes: u64,
    pub duration_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub context_tag: Option<String>,
    pub project_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEvent {
    pub timestamp_us: i64,
    pub pid: u32,
    pub comm: String,
    pub protocol: String,
    pub src_addr: String,
    pub dst_addr: String,
    pub dst_port: u16,
    pub domain: Option<String>,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOpType {
    Create,
    Modify,
    Delete,
    Access,
}

#[allow(dead_code)]
impl FileOpType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "CREATE",
            Self::Modify => "MODIFY",
            Self::Delete => "DELETE",
            Self::Access => "ACCESS",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "CREATE" => Self::Create,
            "DELETE" => Self::Delete,
            "ACCESS" => Self::Access,
            _ => Self::Modify,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    pub timestamp_us: i64,
    pub pid: u32,
    pub comm: String,
    pub file_path: String,
    pub op_type: FileOpType,
    pub total_bytes: u64,
    pub latency_us: u64,
    pub is_workspace: bool,
    pub project_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusEvent {
    pub timestamp_us: i64,
    pub window_class: String,
    pub window_title: Option<String>,
    pub workspace: Option<String>,
    pub duration_ms: Option<u64>,
    pub project_name: Option<String>,
    pub is_deep_work: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum TelemetryEvent {
    Process(ProcessEvent),
    Network(NetworkEvent),
    File(FileEvent),
    Focus(FocusEvent),
}

#[allow(dead_code)]
impl TelemetryEvent {
    pub fn timestamp_us(&self) -> i64 {
        match self {
            Self::Process(e) => e.timestamp_us,
            Self::Network(e) => e.timestamp_us,
            Self::File(e) => e.timestamp_us,
            Self::Focus(e) => e.timestamp_us,
        }
    }
}

/// Analytics metrics for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetric {
    pub comm: String,
    pub app_name: String,
    pub package_name: String,
    pub invocations: u64,
    pub total_duration_ms: u64,
    pub avg_duration_ms: f64,
    pub max_duration_ms: u64,
    pub context_tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationBreakdown {
    pub display_name: String,
    pub package_name: String,
    pub category: String,
    pub active_seconds: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHotspot {
    pub file_path: String,
    pub change_count: u64,
    pub last_seen_us: i64,
    pub project_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDestination {
    pub destination: String,
    pub port: u16,
    pub connection_count: u64,
    pub comms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusBreakdown {
    pub category: String,
    pub active_seconds: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub name: String,
    pub total_seconds: u64,
    pub file_mutations: u64,
    pub process_invocations: u64,
    pub percentage: f64,
    pub top_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowSession {
    pub window_class: String,
    pub window_title: String,
    pub project_name: Option<String>,
    pub duration_sec: u64,
    pub is_deep_work: bool,
    pub start_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyItem {
    pub category: String,
    pub severity: String,
    pub target: String,
    pub metric: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyReport {
    pub generated_at_us: i64,
    pub anomalies: Vec<AnomalyItem>,
    pub system_health_score: f32, // 0 - 100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiContextSummary {
    pub timeframe_hours: u32,
    pub deep_work_score: f32,
    pub context_switches_per_hour: f32,
    pub active_projects: Vec<ProjectSummary>,
    pub modified_files: Vec<String>,
    pub top_tools_used: Vec<String>,
    pub bottlenecks: Vec<ToolMetric>,
    pub standup_markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubconsciousReport {
    pub generated_at_us: i64,
    pub timeframe_hours: u32,
    pub total_events: u64,
    pub deep_work_score: f32, // 0.0 to 100.0 score based on focused window persistence & low fragmentation
    pub context_switches_per_hour: f32,
    pub top_applications: Vec<ApplicationBreakdown>,
    pub focus_breakdown: Vec<FocusBreakdown>,
    pub project_breakdown: Vec<ProjectSummary>,
    pub deep_work_sessions: Vec<FlowSession>,
    pub top_tools: Vec<ToolMetric>,
    pub bottlenecks: Vec<ToolMetric>,
    pub file_hotspots: Vec<FileHotspot>,
    pub network_destinations: Vec<NetworkDestination>,
    pub anomalies_detected: usize,
}
