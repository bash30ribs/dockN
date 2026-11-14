use anyhow::Result;
use crate::model::*;
use crate::storage::Storage;

pub async fn seed_sample_data(storage: &Storage) -> Result<()> {
    let now_us = chrono::Utc::now().timestamp_micros();
    let one_hour_us = 3600 * 1_000_000;

    let sample_events: Vec<TelemetryEvent> = vec![
        // 1. Rust build execution
        TelemetryEvent::Process(ProcessEvent {
            timestamp_us: now_us - (2 * one_hour_us),
            event_type: ProcessEventType::Spawn,
            pid: 14210,
            ppid: Some(1012),
            comm: "cargo".to_string(),
            app_name: Some("Cargo (Rust Build Tool)".to_string()),
            package_name: Some("cargo".to_string()),
            cmdline: Some("cargo check --all-targets".to_string()),
            cwd: Some("/home/ribs/For Ide/dockN".to_string()),
            cpu_pct: 78.5,
            memory_bytes: 420_000_000,
            duration_ms: None,
            exit_code: None,
            context_tag: Some("development".to_string()),
            project_name: Some("dockN".to_string()),
        }),
        TelemetryEvent::Process(ProcessEvent {
            timestamp_us: now_us - (2 * one_hour_us) + 4_800_000,
            event_type: ProcessEventType::Exit,
            pid: 14210,
            ppid: Some(1012),
            comm: "cargo".to_string(),
            app_name: Some("Cargo (Rust Build Tool)".to_string()),
            package_name: Some("cargo".to_string()),
            cmdline: None,
            cwd: None,
            cpu_pct: 0.0,
            memory_bytes: 0,
            duration_ms: Some(4800), // 4.8s
            exit_code: Some(0),
            context_tag: Some("development".to_string()),
            project_name: Some("dockN".to_string()),
        }),

        // 2. Heavy rustc compile step
        TelemetryEvent::Process(ProcessEvent {
            timestamp_us: now_us - (one_hour_us) - 15 * 60 * 1_000_000,
            event_type: ProcessEventType::Exit,
            pid: 14280,
            ppid: Some(14210),
            comm: "rustc".to_string(),
            app_name: Some("Rust Compiler".to_string()),
            package_name: Some("rustc".to_string()),
            cmdline: Some("rustc --crate-name dockn src/main.rs".to_string()),
            cwd: Some("/home/ribs/For Ide/dockN".to_string()),
            cpu_pct: 95.0,
            memory_bytes: 850_000_000,
            duration_ms: Some(12450), // 12.4s (Bottleneck!)
            exit_code: Some(0),
            context_tag: Some("development".to_string()),
            project_name: Some("dockN".to_string()),
        }),

        // 3. Git commit & push
        TelemetryEvent::Process(ProcessEvent {
            timestamp_us: now_us - 45 * 60 * 1_000_000,
            event_type: ProcessEventType::Exit,
            pid: 15301,
            ppid: Some(1012),
            comm: "git".to_string(),
            app_name: Some("Git".to_string()),
            package_name: Some("git".to_string()),
            cmdline: Some("git status".to_string()),
            cwd: Some("/home/ribs/For Ide/dockN".to_string()),
            cpu_pct: 12.0,
            memory_bytes: 35_000_000,
            duration_ms: Some(85),
            exit_code: Some(0),
            context_tag: Some("development".to_string()),
            project_name: Some("dockN".to_string()),
        }),

        // 4. Editor / IDE session
        TelemetryEvent::Process(ProcessEvent {
            timestamp_us: now_us - 30 * 60 * 1_000_000,
            event_type: ProcessEventType::Active,
            pid: 8820,
            ppid: Some(1),
            comm: "nvim".to_string(),
            app_name: Some("Neovim".to_string()),
            package_name: Some("neovim".to_string()),
            cmdline: Some("nvim src/storage.rs".to_string()),
            cwd: Some("/home/ribs/For Ide/dockN".to_string()),
            cpu_pct: 4.2,
            memory_bytes: 120_000_000,
            duration_ms: Some(5400 * 1000), // 1.5 hours in editor
            exit_code: None,
            context_tag: Some("ide".to_string()),
            project_name: Some("dockN".to_string()),
        }),

        // 5. Terminal shell
        TelemetryEvent::Process(ProcessEvent {
            timestamp_us: now_us - 25 * 60 * 1_000_000,
            event_type: ProcessEventType::Active,
            pid: 1012,
            ppid: Some(1),
            comm: "zsh".to_string(),
            app_name: Some("Zsh Shell".to_string()),
            package_name: Some("zsh".to_string()),
            cmdline: Some("zsh".to_string()),
            cwd: Some("/home/ribs/For Ide/dockN".to_string()),
            cpu_pct: 1.0,
            memory_bytes: 45_000_000,
            duration_ms: Some(7200 * 1000),
            exit_code: None,
            context_tag: Some("terminal".to_string()),
            project_name: Some("dockN".to_string()),
        }),

        // 6. Network events (crates.io and github)
        TelemetryEvent::Network(NetworkEvent {
            timestamp_us: now_us - (2 * one_hour_us) + 200_000,
            pid: 14210,
            comm: "cargo".to_string(),
            protocol: "TCP".to_string(),
            src_addr: "192.168.1.15".to_string(),
            dst_addr: "151.101.1.6".to_string(),
            dst_port: 443,
            domain: Some("crates.io".to_string()),
            bytes_sent: 1840,
            bytes_recv: 65420,
            duration_ms: Some(120),
        }),
        TelemetryEvent::Network(NetworkEvent {
            timestamp_us: now_us - 45 * 60 * 1_000_000,
            pid: 15301,
            comm: "git".to_string(),
            protocol: "TCP".to_string(),
            src_addr: "192.168.1.15".to_string(),
            dst_addr: "140.82.121.4".to_string(),
            dst_port: 443,
            domain: Some("github.com".to_string()),
            bytes_sent: 34500,
            bytes_recv: 12200,
            duration_ms: Some(340),
        }),

        // 7. File events
        TelemetryEvent::File(FileEvent {
            timestamp_us: now_us - 15 * 60 * 1_000_000,
            pid: 8820,
            comm: "nvim".to_string(),
            file_path: "/home/ribs/For Ide/dockN/src/storage.rs".to_string(),
            op_type: FileOpType::Modify,
            total_bytes: 8400,
            latency_us: 120,
            is_workspace: true,
            project_name: Some("dockN".to_string()),
        }),
        TelemetryEvent::File(FileEvent {
            timestamp_us: now_us - 12 * 60 * 1_000_000,
            pid: 8820,
            comm: "nvim".to_string(),
            file_path: "/home/ribs/For Ide/dockN/src/storage.rs".to_string(),
            op_type: FileOpType::Modify,
            total_bytes: 8520,
            latency_us: 95,
            is_workspace: true,
            project_name: Some("dockN".to_string()),
        }),
        TelemetryEvent::File(FileEvent {
            timestamp_us: now_us - 10 * 60 * 1_000_000,
            pid: 8820,
            comm: "nvim".to_string(),
            file_path: "/home/ribs/For Ide/dockN/src/analytics.rs".to_string(),
            op_type: FileOpType::Create,
            total_bytes: 5120,
            latency_us: 140,
            is_workspace: true,
            project_name: Some("dockN".to_string()),
        }),

        // 8. Focus Events (Real Deep Work Flow)
        TelemetryEvent::Focus(FocusEvent {
            timestamp_us: now_us - 90 * 60 * 1_000_000,
            window_class: "antigravity-ide".to_string(),
            window_title: Some("src/storage.rs — dockN — Antigravity IDE".to_string()),
            workspace: Some("1".to_string()),
            duration_ms: Some(35 * 60 * 1000), // 35 minutes uninterrupted
            project_name: Some("dockN".to_string()),
            is_deep_work: true,
        }),
        TelemetryEvent::Focus(FocusEvent {
            timestamp_us: now_us - 50 * 60 * 1_000_000,
            window_class: "kitty".to_string(),
            window_title: Some("dockN terminal (cargo test)".to_string()),
            workspace: Some("1".to_string()),
            duration_ms: Some(18 * 60 * 1000), // 18 minutes uninterrupted
            project_name: Some("dockN".to_string()),
            is_deep_work: true,
        }),
    ];

    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let writer = storage.spawn_writer(rx, shutdown_rx);

    for ev in sample_events {
        let _ = tx.send(ev).await;
    }
    drop(tx);
    let _ = shutdown_tx.send(true);
    let _ = writer.await;

    Ok(())
}
