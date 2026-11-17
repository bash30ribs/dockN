use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info};

use crate::model::*;

pub struct DatabaseStatus {
    pub db_path: PathBuf,
    pub file_size_bytes: u64,
    pub total_process_events: u64,
    pub total_network_events: u64,
    pub total_file_events: u64,
    pub total_focus_events: u64,
    pub first_event_us: Option<i64>,
    pub last_event_us: Option<i64>,
}

#[derive(Clone)]
pub struct Storage {
    pub(crate) conn_pool: Arc<Mutex<Connection>>,
    pub(crate) db_path: PathBuf,
}

impl Storage {
    pub fn get_raw_connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn_pool.lock().unwrap()
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        if let Some(parent) = path_buf.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory for {:?}", path_buf))?;
        }

        let conn = Connection::open(&path_buf)
            .with_context(|| format!("Failed to open SQLite database at {:?}", path_buf))?;

        // Tune SQLite for modern high-throughput streaming and instant analytical fetches
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA mmap_size = 268435456;
            PRAGMA cache_size = -64000;
            PRAGMA temp_store = MEMORY;
            PRAGMA auto_vacuum = INCREMENTAL;
            "#,
        )?;

        let storage = Self {
            conn_pool: Arc::new(Mutex::new(conn)),
            db_path: path_buf,
        };

        storage.init_schema()?;
        Ok(storage)
    }

    pub fn default_db_path() -> PathBuf {
        let base_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("./.dockn_data"))
            .join("dockn");
        base_dir.join("subconscious.db")
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn_pool.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS process_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_us INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                pid INTEGER NOT NULL,
                ppid INTEGER,
                comm TEXT NOT NULL,
                app_name TEXT,
                package_name TEXT,
                cmdline TEXT,
                cwd TEXT,
                cpu_pct REAL DEFAULT 0.0,
                memory_bytes INTEGER DEFAULT 0,
                duration_ms INTEGER,
                exit_code INTEGER,
                context_tag TEXT,
                project_name TEXT
            );

            CREATE TABLE IF NOT EXISTS network_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_us INTEGER NOT NULL,
                pid INTEGER NOT NULL,
                comm TEXT NOT NULL,
                protocol TEXT NOT NULL,
                src_addr TEXT NOT NULL,
                dst_addr TEXT NOT NULL,
                dst_port INTEGER NOT NULL,
                domain TEXT,
                bytes_sent INTEGER DEFAULT 0,
                bytes_recv INTEGER DEFAULT 0,
                duration_ms INTEGER
            );

            CREATE TABLE IF NOT EXISTS file_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_us INTEGER NOT NULL,
                pid INTEGER NOT NULL,
                comm TEXT NOT NULL,
                file_path TEXT NOT NULL,
                op_type TEXT NOT NULL,
                total_bytes INTEGER DEFAULT 0,
                latency_us INTEGER DEFAULT 0,
                is_workspace INTEGER DEFAULT 0,
                project_name TEXT
            );

            CREATE TABLE IF NOT EXISTS focus_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_us INTEGER NOT NULL,
                window_class TEXT NOT NULL,
                window_title TEXT,
                workspace TEXT,
                duration_ms INTEGER,
                project_name TEXT,
                is_deep_work INTEGER DEFAULT 0
            );
            "#,
        )?;

        // Ensure columns exist on older databases before creating indices
        let _ = conn.execute("ALTER TABLE process_events ADD COLUMN app_name TEXT;", []);
        let _ = conn.execute("ALTER TABLE process_events ADD COLUMN package_name TEXT;", []);
        let _ = conn.execute("ALTER TABLE process_events ADD COLUMN project_name TEXT;", []);
        let _ = conn.execute("ALTER TABLE file_events ADD COLUMN project_name TEXT;", []);

        conn.execute_batch(
            r#"
            -- Indices for microsecond analytical rollups
            CREATE INDEX IF NOT EXISTS idx_proc_time ON process_events(timestamp_us);
            CREATE INDEX IF NOT EXISTS idx_proc_comm ON process_events(comm);
            CREATE INDEX IF NOT EXISTS idx_proc_app ON process_events(app_name);
            CREATE INDEX IF NOT EXISTS idx_proc_tag ON process_events(context_tag);
            CREATE INDEX IF NOT EXISTS idx_proc_proj ON process_events(project_name);
            CREATE INDEX IF NOT EXISTS idx_proc_dur ON process_events(duration_ms) WHERE duration_ms IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_net_time ON network_events(timestamp_us);
            CREATE INDEX IF NOT EXISTS idx_net_dst ON network_events(dst_addr);
            CREATE INDEX IF NOT EXISTS idx_file_time ON file_events(timestamp_us);
            CREATE INDEX IF NOT EXISTS idx_file_path ON file_events(file_path);
            CREATE INDEX IF NOT EXISTS idx_file_proj ON file_events(project_name);
            CREATE INDEX IF NOT EXISTS idx_focus_time ON focus_events(timestamp_us);
            CREATE INDEX IF NOT EXISTS idx_focus_class ON focus_events(window_class);
            CREATE INDEX IF NOT EXISTS idx_focus_proj ON focus_events(project_name);
            "#,
        )?;

        Ok(())
    }

    /// Spawns an asynchronous batch writer loop
    pub fn spawn_writer(
        &self,
        mut rx: mpsc::Receiver<TelemetryEvent>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> tokio::task::JoinHandle<()> {
        let storage = self.clone();
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(256);
            let mut interval = tokio::time::interval(Duration::from_millis(250));

            loop {
                tokio::select! {
                    event_opt = rx.recv() => {
                        match event_opt {
                            Some(event) => {
                                batch.push(event);
                                if batch.len() >= 256 {
                                    if let Err(e) = storage.flush_batch(&batch).await {
                                        error!("Failed to flush batch of {} events: {:?}", batch.len(), e);
                                    }
                                    batch.clear();
                                }
                            }
                            None => break,
                        }
                    }
                    _ = interval.tick() => {
                        if !batch.is_empty() {
                            if let Err(e) = storage.flush_batch(&batch).await {
                                error!("Failed to periodically flush {} events: {:?}", batch.len(), e);
                            }
                            batch.clear();
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            info!("Writer received shutdown signal. Flushing remaining queue...");
                            break;
                        }
                    }
                }
            }

            // Drain remaining events in channel
            while let Ok(event) = rx.try_recv() {
                batch.push(event);
            }
            if !batch.is_empty() {
                let _ = storage.flush_batch(&batch).await;
            }
            info!("Writer shutdown complete.");
        })
    }

    async fn flush_batch(&self, batch: &[TelemetryEvent]) -> Result<()> {
        let mut conn = self.conn_pool.lock().unwrap();
        let tx = conn.transaction()?;

        {
            let mut proc_stmt = tx.prepare_cached(
                r#"
                INSERT INTO process_events (
                    timestamp_us, event_type, pid, ppid, comm, app_name, package_name,
                    cmdline, cwd, cpu_pct, memory_bytes, duration_ms, exit_code, context_tag, project_name
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                "#,
            )?;

            let mut net_stmt = tx.prepare_cached(
                r#"
                INSERT INTO network_events (
                    timestamp_us, pid, comm, protocol, src_addr, dst_addr,
                    dst_port, domain, bytes_sent, bytes_recv, duration_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
            )?;

            let mut file_stmt = tx.prepare_cached(
                r#"
                INSERT INTO file_events (
                    timestamp_us, pid, comm, file_path, op_type,
                    total_bytes, latency_us, is_workspace, project_name
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
            )?;

            let mut focus_stmt = tx.prepare_cached(
                r#"
                INSERT INTO focus_events (
                    timestamp_us, window_class, window_title, workspace,
                    duration_ms, project_name, is_deep_work
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
            )?;

            for event in batch {
                match event {
                    TelemetryEvent::Process(p) => {
                        proc_stmt.execute(params![
                            p.timestamp_us,
                            p.event_type.as_str(),
                            p.pid as i64,
                            p.ppid.map(|pp| pp as i64),
                            p.comm,
                            p.app_name,
                            p.package_name,
                            p.cmdline,
                            p.cwd,
                            p.cpu_pct,
                            p.memory_bytes as i64,
                            p.duration_ms.map(|d| d as i64),
                            p.exit_code,
                            p.context_tag,
                            p.project_name,
                        ])?;
                    }
                    TelemetryEvent::Network(n) => {
                        net_stmt.execute(params![
                            n.timestamp_us,
                            n.pid as i64,
                            n.comm,
                            n.protocol,
                            n.src_addr,
                            n.dst_addr,
                            n.dst_port as i64,
                            n.domain,
                            n.bytes_sent as i64,
                            n.bytes_recv as i64,
                            n.duration_ms.map(|d| d as i64),
                        ])?;
                    }
                    TelemetryEvent::File(f) => {
                        file_stmt.execute(params![
                            f.timestamp_us,
                            f.pid as i64,
                            f.comm,
                            f.file_path,
                            f.op_type.as_str(),
                            f.total_bytes as i64,
                            f.latency_us as i64,
                            if f.is_workspace { 1 } else { 0 },
                            f.project_name,
                        ])?;
                    }
                    TelemetryEvent::Focus(fc) => {
                        focus_stmt.execute(params![
                            fc.timestamp_us,
                            fc.window_class,
                            fc.window_title,
                            fc.workspace,
                            fc.duration_ms.map(|d| d as i64),
                            fc.project_name,
                            if fc.is_deep_work { 1 } else { 0 },
                        ])?;
                    }
                }
            }
        }

        tx.commit()?;
        debug!("Flushed {} events into SQLite successfully", batch.len());
        Ok(())
    }

    pub fn get_status(&self) -> Result<DatabaseStatus> {
        let conn = self.conn_pool.lock().unwrap();
        let file_size = std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let proc_count: i64 = conn.query_row(
            "SELECT count(*) FROM process_events",
            [],
            |r| r.get(0),
        ).unwrap_or(0);

        let net_count: i64 = conn.query_row(
            "SELECT count(*) FROM network_events",
            [],
            |r| r.get(0),
        ).unwrap_or(0);

        let file_count: i64 = conn.query_row(
            "SELECT count(*) FROM file_events",
            [],
            |r| r.get(0),
        ).unwrap_or(0);

        let focus_count: i64 = conn.query_row(
            "SELECT count(*) FROM focus_events",
            [],
            |r| r.get(0),
        ).unwrap_or(0);

        let first_us: Option<i64> = conn.query_row(
            "SELECT min(timestamp_us) FROM process_events",
            [],
            |r| r.get(0),
        ).ok();

        let last_us: Option<i64> = conn.query_row(
            "SELECT max(timestamp_us) FROM process_events",
            [],
            |r| r.get(0),
        ).ok();

        Ok(DatabaseStatus {
            db_path: self.db_path.clone(),
            file_size_bytes: file_size,
            total_process_events: proc_count as u64,
            total_network_events: net_count as u64,
            total_file_events: file_count as u64,
            total_focus_events: focus_count as u64,
            first_event_us: first_us,
            last_event_us: last_us,
        })
    }

    pub fn get_bottlenecks(&self, min_duration_ms: u64, limit: usize) -> Result<Vec<ToolMetric>> {
        let conn = self.conn_pool.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT coalesce(app_name, comm) as app,
                   coalesce(package_name, comm) as pkg,
                   comm,
                   count(*) as invocations,
                   sum(duration_ms) as total_dur,
                   avg(duration_ms) as avg_dur,
                   max(duration_ms) as max_dur,
                   coalesce(context_tag, 'dev-tool') as tag
            FROM process_events
            WHERE duration_ms >= ?1
              AND comm NOT IN ('tokio-rt-worker', 'notify-rs inoti', 'DedicatedWorker', 'QSGRenderThread', 'qs:traceq0', 'qs:gdrv0', 'ThreadPoolForeg', 'Chrome_IOThread', 'AudioService', 'QQmlThread', 'QQuickPixmapRea', 'gmain', 'Thread (pooled)')
              AND comm NOT LIKE 'qs:%'
              AND comm NOT LIKE 'pool-%'
              AND comm NOT LIKE '%worker%'
              AND comm NOT LIKE 'Q%'
              AND coalesce(app_name, comm) NOT LIKE '%thread%'
              AND coalesce(app_name, comm) NOT LIKE '%pooled%'
            GROUP BY app
            ORDER BY max_dur DESC
            LIMIT ?2
            "#,
        )?;

        let rows = stmt.query_map(params![min_duration_ms as i64, limit as i64], |row| {
            let app_name: String = row.get(0)?;
            let package_name: String = row.get(1)?;
            let comm: String = row.get(2)?;
            let invocations: i64 = row.get(3)?;
            let total_dur: i64 = row.get(4)?;
            let max_dur: i64 = row.get(6)?;
            Ok(ToolMetric {
                comm,
                app_name,
                package_name,
                invocations: invocations as u64,
                total_duration_ms: total_dur as u64,
                avg_duration_ms: row.get(5)?,
                max_duration_ms: max_dur as u64,
                context_tag: row.get(7)?,
            })
        })?;

        let mut metrics = Vec::new();
        for r in rows {
            metrics.push(r?);
        }
        Ok(metrics)
    }

    pub fn get_project_summaries(&self, since_us: i64) -> Result<Vec<ProjectSummary>> {
        let conn = self.conn_pool.lock().unwrap();

        // 1. Project activity from focus events (highest accuracy)
        let mut proj_map: HashMap<String, (u64, u64, u64, Vec<String>)> = HashMap::new();

        {
            let mut stmt = conn.prepare(
                r#"
                SELECT project_name, sum(coalesce(duration_ms, 1000)) / 1000 as total_sec
                FROM focus_events
                WHERE timestamp_us >= ?1 AND project_name IS NOT NULL
                GROUP BY project_name
                ORDER BY total_sec DESC
                "#,
            )?;
            let rows = stmt.query_map(params![since_us], |r| {
                let name: String = r.get(0)?;
                let sec: i64 = r.get(1)?;
                Ok((name, sec.max(0) as u64))
            })?;
            for r in rows {
                let (name, sec) = r?;
                proj_map.entry(name).or_insert((0, 0, 0, Vec::new())).0 += sec;
            }
        }

        // 2. Project activity from file events
        {
            let mut stmt = conn.prepare(
                r#"
                SELECT project_name, count(*) as mutations
                FROM file_events
                WHERE timestamp_us >= ?1 AND project_name IS NOT NULL
                GROUP BY project_name
                "#,
            )?;
            let rows = stmt.query_map(params![since_us], |r| {
                let name: String = r.get(0)?;
                let count: i64 = r.get(1)?;
                Ok((name, count.max(0) as u64))
            })?;
            for r in rows {
                let (name, count) = r?;
                proj_map.entry(name).or_insert((0, 0, 0, Vec::new())).1 += count;
            }
        }

        // 3. Project activity from process events
        {
            let mut stmt = conn.prepare(
                r#"
                SELECT project_name, count(*) as procs, sum(coalesce(duration_ms, 0)) / 1000 as proc_sec
                FROM process_events
                WHERE timestamp_us >= ?1 AND project_name IS NOT NULL
                GROUP BY project_name
                "#,
            )?;
            let rows = stmt.query_map(params![since_us], |r| {
                let name: String = r.get(0)?;
                let count: i64 = r.get(1)?;
                let sec: i64 = r.get(2)?;
                Ok((name, count.max(0) as u64, sec.max(0) as u64))
            })?;
            for r in rows {
                let (name, count, sec) = r?;
                let entry = proj_map.entry(name).or_insert((0, 0, 0, Vec::new()));
                entry.2 += count;
                // If no focus time was logged, use process execution time
                if entry.0 == 0 {
                    entry.0 += sec;
                }
            }
        }

        let total_time: u64 = proj_map.values().map(|v| v.0).sum();
        let mut summaries: Vec<ProjectSummary> = proj_map
            .into_iter()
            .map(|(name, (sec, file_m, proc_c, tools))| {
                let percentage = if total_time > 0 {
                    (sec as f64 / total_time as f64) * 100.0
                } else {
                    0.0
                };
                ProjectSummary {
                    name,
                    total_seconds: sec,
                    file_mutations: file_m,
                    process_invocations: proc_c,
                    percentage,
                    top_tools: tools,
                }
            })
            .collect();

        summaries.sort_by(|a, b| b.total_seconds.cmp(&a.total_seconds).then(b.file_mutations.cmp(&a.file_mutations)));
        Ok(summaries)
    }

    pub fn get_focus_sessions(&self, since_us: i64) -> Result<Vec<FlowSession>> {
        let conn = self.conn_pool.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT window_class, coalesce(window_title, window_class) as title,
                   project_name, coalesce(duration_ms, 0) / 1000 as dur_sec,
                   is_deep_work, timestamp_us
            FROM focus_events
            WHERE timestamp_us >= ?1 AND duration_ms >= 5000
            ORDER BY timestamp_us DESC
            LIMIT 50
            "#,
        )?;

        let rows = stmt.query_map(params![since_us], |r| {
            let cls: String = r.get(0)?;
            let title: String = r.get(1)?;
            let proj: Option<String> = r.get(2)?;
            let dur: i64 = r.get(3)?;
            let deep: i32 = r.get(4)?;
            let ts: i64 = r.get(5)?;
            Ok(FlowSession {
                window_class: cls,
                window_title: title,
                project_name: proj,
                duration_sec: dur.max(0) as u64,
                is_deep_work: deep == 1,
                start_us: ts,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_anomalies(&self, since_us: i64) -> Result<AnomalyReport> {
        let conn = self.conn_pool.lock().unwrap();
        let mut anomalies = Vec::new();
        let mut penalty_points: f32 = 0.0;

        // 1. File Thrashing Check (> 500 mutations in time window)
        {
            let mut stmt = conn.prepare(
                r#"
                SELECT file_path, count(*) as count
                FROM file_events
                WHERE timestamp_us >= ?1
                GROUP BY file_path
                HAVING count > 500
                ORDER BY count DESC
                LIMIT 5
                "#,
            )?;
            let rows = stmt.query_map(params![since_us], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?;
            for r in rows {
                let (path, count) = r?;
                penalty_points += 10.0;
                anomalies.push(AnomalyItem {
                    category: "Disk I/O Thrashing".to_string(),
                    severity: "HIGH".to_string(),
                    target: path.clone(),
                    metric: format!("{} mutations", count),
                    recommendation: if path.contains("subconscious") {
                        "dockN file watcher loop detected. Path has now been excluded.".to_string()
                    } else if path.contains("qdeck") {
                        "Statusbar script is writing to disk in a tight poll loop. Consider tmpfs or in-memory state.".to_string()
                    } else {
                        "File is receiving rapid disk writes. Verify if logging or caching can be buffered.".to_string()
                    },
                });
            }
        }

        // 2. Runaway Process Fork/Execution Check (> 150 invocations with < 100ms duration)
        {
            let mut stmt = conn.prepare(
                r#"
                SELECT coalesce(app_name, comm) as app, count(*) as count, avg(coalesce(duration_ms, 0)) as avg_dur
                FROM process_events
                WHERE timestamp_us >= ?1 AND duration_ms < 200
                  AND comm NOT IN ('dockn', 'tokio-rt-worker')
                GROUP BY app
                HAVING count > 100
                ORDER BY count DESC
                LIMIT 5
                "#,
            )?;
            let rows = stmt.query_map(params![since_us], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, f64>(2)?))
            })?;
            for r in rows {
                let (app, count, avg_dur) = r?;
                penalty_points += 15.0;
                anomalies.push(AnomalyItem {
                    category: "Runaway Fork / Script Spin".to_string(),
                    severity: "MEDIUM".to_string(),
                    target: app,
                    metric: format!("{} invocations (avg {:.1}ms)", count, avg_dur),
                    recommendation: "Process is spawning at high frequency. Check for cron/daemon loops or status bar polling.".to_string(),
                });
            }
        }

        // 3. Socket Storm / Network Loop (> 300 connections to same endpoint)
        {
            let mut stmt = conn.prepare(
                r#"
                SELECT dst_addr, dst_port, count(*) as count
                FROM network_events
                WHERE timestamp_us >= ?1
                GROUP BY dst_addr, dst_port
                HAVING count > 300
                ORDER BY count DESC
                LIMIT 5
                "#,
            )?;
            let rows = stmt.query_map(params![since_us], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
            })?;
            for r in rows {
                let (addr, port, count) = r?;
                penalty_points += 8.0;
                anomalies.push(AnomalyItem {
                    category: "Network Reconnect Storm".to_string(),
                    severity: "MEDIUM".to_string(),
                    target: format!("{}:{}", addr, port),
                    metric: format!("{} connections", count),
                    recommendation: "High-frequency TCP connection establishment. Verify keep-alive or connection pooling.".to_string(),
                });
            }
        }

        let system_health_score = (100.0 - penalty_points).clamp(10.0, 100.0);
        Ok(AnomalyReport {
            generated_at_us: chrono::Utc::now().timestamp_micros(),
            anomalies,
            system_health_score,
        })
    }

    pub fn get_timeline(&self, limit: usize) -> Result<Vec<String>> {
        let conn = self.conn_pool.lock().unwrap();
        let mut entries = Vec::new();

        // Interleave recent process and focus events
        let mut stmt = conn.prepare(
            r#"
            SELECT timestamp_us, 'FOCUS' as kind, coalesce(window_class, 'unknown') as title, coalesce(project_name, '') as detail
            FROM focus_events
            UNION ALL
            SELECT timestamp_us, 'PROC' as kind, coalesce(app_name, comm) as title, coalesce(cmdline, '') as detail
            FROM process_events
            WHERE event_type = 'SPAWN'
            ORDER BY timestamp_us DESC
            LIMIT ?1
            "#,
        )?;

        let rows = stmt.query_map(params![limit as i64], |r| {
            let ts: i64 = r.get(0)?;
            let kind: String = r.get(1)?;
            let title: String = r.get(2)?;
            let detail: String = r.get(3)?;
            let dt = chrono::DateTime::from_timestamp_micros(ts)
                .map(|t| t.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "00:00:00".to_string());
            Ok(format!("[{}] {:<6} {:<24} {}", dt, kind, title, detail))
        })?;

        for r in rows {
            entries.push(r?);
        }
        Ok(entries)
    }

    pub fn record_command(
        &self,
        comm: &str,
        cmdline: &str,
        cwd: &str,
        duration_ms: u64,
        exit_code: i32,
    ) -> Result<()> {
        let now_us = chrono::Utc::now().timestamp_micros();
        let app_info = crate::app_name::AppResolver::get().resolve(comm, None, &[]);
        let project_name = crate::project::ProjectDetector::get().resolve_project_from_path(cwd);

        let conn = self.conn_pool.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO process_events (
                timestamp_us, event_type, pid, comm, app_name, package_name,
                cmdline, cwd, duration_ms, exit_code, context_tag, project_name
            ) VALUES (?1, 'EXIT', 0, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                now_us,
                app_info.package_name,
                app_info.display_name,
                app_info.package_name,
                cmdline,
                cwd,
                duration_ms as i64,
                exit_code,
                app_info.category,
                project_name,
            ],
        )?;

        Ok(())
    }

    pub fn prune_older_than(&self, days: u32, vacuum: bool) -> Result<PruneSummary> {
        let conn = self.conn_pool.lock().unwrap();
        let cutoff_us = chrono::Utc::now().timestamp_micros() - (days as i64 * 86400 * 1_000_000);

        let before_bytes = std::fs::metadata(&self.db_path).map(|m| m.len()).unwrap_or(0);

        let proc_del: usize = conn.execute("DELETE FROM process_events WHERE timestamp_us < ?1", params![cutoff_us])?;
        let net_del: usize = conn.execute("DELETE FROM network_events WHERE timestamp_us < ?1", params![cutoff_us])?;
        let file_del: usize = conn.execute("DELETE FROM file_events WHERE timestamp_us < ?1", params![cutoff_us])?;
        let focus_del: usize = conn.execute("DELETE FROM focus_events WHERE timestamp_us < ?1", params![cutoff_us])?;

        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

        if vacuum {
            conn.execute_batch("VACUUM;")?;
        }

        let after_bytes = std::fs::metadata(&self.db_path).map(|m| m.len()).unwrap_or(0);

        Ok(PruneSummary {
            process_events_deleted: proc_del as u64,
            network_events_deleted: net_del as u64,
            file_events_deleted: file_del as u64,
            focus_events_deleted: focus_del as u64,
            total_deleted: (proc_del + net_del + file_del + focus_del) as u64,
            before_bytes,
            after_bytes,
        })
    }

    pub fn get_resource_profile(&self, since_us: i64) -> Result<Vec<ResourceMetric>> {
        let conn = self.conn_pool.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT coalesce(app_name, comm) as app,
                   coalesce(package_name, comm) as pkg,
                   avg(cpu_pct) as avg_cpu,
                   max(cpu_pct) as peak_cpu,
                   max(memory_bytes) as peak_mem,
                   count(*) as samples,
                   coalesce(context_tag, 'system') as tag
            FROM process_events
            WHERE timestamp_us >= ?1 AND (memory_bytes > 0 OR cpu_pct > 0.0)
            GROUP BY app
            ORDER BY peak_mem DESC
            LIMIT 15
            "#,
        )?;

        let rows = stmt.query_map(params![since_us], |r| {
            let app: String = r.get(0)?;
            let pkg: String = r.get(1)?;
            let avg_cpu: f64 = r.get(2)?;
            let peak_cpu: f64 = r.get(3)?;
            let peak_mem: i64 = r.get(4)?;
            let samples: i64 = r.get(5)?;
            let tag: String = r.get(6)?;
            Ok(ResourceMetric {
                app_name: app,
                package_name: pkg,
                avg_cpu_pct: avg_cpu as f32,
                peak_cpu_pct: peak_cpu as f32,
                peak_memory_bytes: peak_mem.max(0) as u64,
                samples_count: samples as u64,
                context_tag: tag,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn export_table(&self, table_name: &str, format: &str, writer: &mut dyn std::io::Write) -> Result<usize> {
        let conn = self.conn_pool.lock().unwrap();
        let allowed_tables = ["process_events", "network_events", "file_events", "focus_events"];
        if !allowed_tables.contains(&table_name) {
            anyhow::bail!("Invalid table name for export: {}", table_name);
        }

        let mut stmt = conn.prepare(&format!("SELECT * FROM {} ORDER BY timestamp_us ASC", table_name))?;
        let col_count = stmt.column_count();
        let col_names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();

        let mut count = 0;
        let mut rows = stmt.query([])?;

        if format == "csv" {
            writeln!(writer, "{}", col_names.join(","))?;
            while let Some(row) = rows.next()? {
                let mut line = Vec::new();
                for i in 0..col_count {
                    let val = match row.get_ref(i)? {
                        rusqlite::types::ValueRef::Null => String::new(),
                        rusqlite::types::ValueRef::Integer(v) => v.to_string(),
                        rusqlite::types::ValueRef::Real(v) => v.to_string(),
                        rusqlite::types::ValueRef::Text(v) => {
                            let s = String::from_utf8_lossy(v);
                            if s.contains(',') || s.contains('"') || s.contains('\n') {
                                format!("\"{}\"", s.replace('"', "\"\""))
                            } else {
                                s.to_string()
                            }
                        }
                        rusqlite::types::ValueRef::Blob(_) => "<blob>".to_string(),
                    };
                    line.push(val);
                }
                writeln!(writer, "{}", line.join(","))?;
                count += 1;
            }
        } else {
            // jsonl format
            while let Some(row) = rows.next()? {
                let mut map = serde_json::Map::new();
                for i in 0..col_count {
                    let name = col_names[i].clone();
                    let val = match row.get_ref(i)? {
                        rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                        rusqlite::types::ValueRef::Integer(v) => serde_json::Value::from(v),
                        rusqlite::types::ValueRef::Real(v) => serde_json::Value::from(v),
                        rusqlite::types::ValueRef::Text(v) => serde_json::Value::from(String::from_utf8_lossy(v).to_string()),
                        rusqlite::types::ValueRef::Blob(_) => serde_json::Value::from("<blob>"),
                    };
                    map.insert(name, val);
                }
                writeln!(writer, "{}", serde_json::to_string(&map)?)?;
                count += 1;
            }
        }

        Ok(count)
    }
}
