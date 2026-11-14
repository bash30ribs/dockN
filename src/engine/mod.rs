pub mod detector;
pub mod ebpf;
pub mod universal;

use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, watch};
use tracing::{info, warn};

pub use detector::{EngineDetector, EngineType};
pub use ebpf::EbpfEngine;
pub use universal::UniversalEngine;

use crate::focus::FocusTracker;
use crate::model::TelemetryEvent;

pub struct EngineManager {
    preferred_type: Option<EngineType>,
    workspace_path: PathBuf,
}

impl EngineManager {
    pub fn new<P: AsRef<Path>>(preferred_type: Option<EngineType>, workspace_path: P) -> Self {
        Self {
            preferred_type,
            workspace_path: workspace_path.as_ref().to_path_buf(),
        }
    }

    pub async fn start(
        &self,
        tx: mpsc::Sender<TelemetryEvent>,
        shutdown: watch::Receiver<bool>,
    ) -> Result<()> {
        let engine_type = self
            .preferred_type
            .unwrap_or_else(EngineDetector::detect_best_engine);

        info!("Activating Telemetry Engine: {}", engine_type.as_str());

        // Spawn Desktop/Window Focus Tracker concurrently
        let focus_tx = tx.clone();
        let focus_shutdown = shutdown.clone();
        tokio::spawn(async move {
            let tracker = FocusTracker::new();
            if let Err(e) = tracker.run(focus_tx, focus_shutdown).await {
                warn!("Focus tracker stopped: {:?}", e);
            }
        });

        match engine_type {
            EngineType::LinuxEbpf => {
                let mut ebpf = EbpfEngine::new();
                if ebpf.is_available() {
                    if let Err(e) = ebpf.run(tx.clone(), shutdown.clone()).await {
                        warn!("eBPF engine encountered an error ({:?}). Falling back to Universal Adaptive Engine.", e);
                        let mut universal = UniversalEngine::new(&self.workspace_path);
                        return universal.run(tx, shutdown).await;
                    }
                } else {
                    info!("Linux kernel tracepoints not directly accessible without root; activating Universal Adaptive Engine.");
                    let mut universal = UniversalEngine::new(&self.workspace_path);
                    return universal.run(tx, shutdown).await;
                }
            }
            EngineType::UniversalAdaptive => {
                let mut universal = UniversalEngine::new(&self.workspace_path);
                return universal.run(tx, shutdown).await;
            }
        }

        Ok(())
    }
}
