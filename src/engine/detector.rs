use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    LinuxEbpf,
    UniversalAdaptive,
}

impl EngineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LinuxEbpf => "Linux eBPF (Kernel-Level Telemetry)",
            Self::UniversalAdaptive => "Universal Adaptive (Cross-Platform High-Speed Telemetry)",
        }
    }
}

pub struct EngineDetector;

impl EngineDetector {
    pub fn detect_best_engine() -> EngineType {
        #[cfg(target_os = "linux")]
        {
            if Self::is_ebpf_capable() {
                info!("eBPF environment validated: Kernel BTF & tracepoints active.");
                return EngineType::LinuxEbpf;
            }
        }

        info!("Selecting Universal Adaptive Engine (compatible with all OSs, enterprise laptops, and zero-privilege containers).");
        EngineType::UniversalAdaptive
    }

    #[cfg(target_os = "linux")]
    pub fn is_ebpf_capable() -> bool {
        let has_btf = Path::new("/sys/kernel/btf/vmlinux").exists();
        let has_tracefs = Path::new("/sys/kernel/tracing").exists()
            || Path::new("/sys/kernel/debug/tracing").exists();
        let is_root = unsafe { libc::geteuid() == 0 };

        // Even if non-root, some systems have CAP_BPF or unprivileged tracepoints
        let tracefs_accessible = Path::new("/sys/kernel/tracing/events/sched").is_dir()
            || Path::new("/sys/kernel/debug/tracing/events/sched").is_dir();

        (has_btf || has_tracefs) && (is_root || tracefs_accessible)
    }

    #[cfg(not(target_os = "linux"))]
    pub fn is_ebpf_capable() -> bool {
        false
    }
}
