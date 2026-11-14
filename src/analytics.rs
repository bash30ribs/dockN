use anyhow::Result;
use rusqlite::params;
use std::collections::HashMap;

use crate::model::*;
use crate::storage::Storage;

pub struct AnalyticsEngine;

impl AnalyticsEngine {
    pub fn compute_report(storage: &Storage, hours: u32) -> Result<SubconsciousReport> {
        let conn = storage.get_raw_connection();
        let now_us = chrono::Utc::now().timestamp_micros();
        let since_us = if hours > 0 {
            now_us - (hours as i64 * 3600 * 1_000_000)
        } else {
            0
        };

        // 1. Total event counts
        let total_proc: i64 = conn.query_row(
            "SELECT count(*) FROM process_events WHERE timestamp_us >= ?1",
            params![since_us],
            |r| r.get(0),
        ).unwrap_or(0);

        let total_net: i64 = conn.query_row(
            "SELECT count(*) FROM network_events WHERE timestamp_us >= ?1",
            params![since_us],
            |r| r.get(0),
        ).unwrap_or(0);

        let total_file: i64 = conn.query_row(
            "SELECT count(*) FROM file_events WHERE timestamp_us >= ?1",
            params![since_us],
            |r| r.get(0),
        ).unwrap_or(0);

        let total_focus: i64 = conn.query_row(
            "SELECT count(*) FROM focus_events WHERE timestamp_us >= ?1",
            params![since_us],
            |r| r.get(0),
        ).unwrap_or(0);

        let total_events = (total_proc + total_net + total_file + total_focus) as u64;

        // 2. Focus Sessions & Deep Work calculation from active window tracking
        drop(conn); // release lock before storage helper calls
        let deep_work_sessions = storage.get_focus_sessions(since_us)?;
        let project_breakdown = storage.get_project_summaries(since_us)?;
        let anomalies_report = storage.get_anomalies(since_us)?;
        let anomalies_detected = anomalies_report.anomalies.len();
        let conn = storage.get_raw_connection();

        let actual_hours = if hours > 0 {
            hours as f32
        } else {
            24.0
        };

        let (deep_work_score, context_switches_per_hour) = if !deep_work_sessions.is_empty() {
            let total_focus_sec: u64 = deep_work_sessions.iter().map(|s| s.duration_sec).sum();
            let deep_sec: u64 = deep_work_sessions.iter().filter(|s| s.is_deep_work).map(|s| s.duration_sec).sum();
            let switches = deep_work_sessions.len().saturating_sub(1);
            let switches_per_hr = (switches as f32 / actual_hours).max(0.0);

            let score = if total_focus_sec > 0 {
                let ratio = (deep_sec as f64 / total_focus_sec as f64) * 100.0;
                let penalty = (switches_per_hr * 1.2).min(30.0);
                (ratio - penalty as f64).clamp(10.0, 100.0) as f32
            } else {
                50.0
            };
            (score, switches_per_hr)
        } else {
            // Intelligent fallback: estimate from unique user applications (excluding system threads)
            let dev_procs: i64 = conn.query_row(
                r#"
                SELECT count(DISTINCT coalesce(app_name, comm))
                FROM process_events
                WHERE timestamp_us >= ?1 AND context_tag IN ('development', 'ide', 'terminal')
                "#,
                params![since_us],
                |r| r.get(0),
            ).unwrap_or(1);

            let estimated_switches = (dev_procs * 3) as f32;
            let switches_per_hr = (estimated_switches / actual_hours).max(0.5);
            let score = (85.0 - (switches_per_hr * 2.0)).clamp(20.0, 95.0);
            (score, switches_per_hr)
        };

        // 3. Focus Breakdown by Category
        let mut focus_map: HashMap<String, u64> = HashMap::new();
        {
            let mut stmt = conn.prepare(
                r#"
                SELECT coalesce(context_tag, 'other') as tag,
                       sum(coalesce(duration_ms, 1000)) / 1000 as total_sec
                FROM process_events
                WHERE timestamp_us >= ?1
                GROUP BY tag
                ORDER BY total_sec DESC
                "#,
            )?;

            let rows = stmt.query_map(params![since_us], |r| {
                let tag: String = r.get(0)?;
                let sec: i64 = r.get(1)?;
                Ok((tag, sec.max(0) as u64))
            })?;

            for r in rows {
                let (tag, sec) = r?;
                focus_map.insert(tag, sec);
            }
        }

        let total_active_sec: u64 = focus_map.values().sum();
        let focus_breakdown: Vec<FocusBreakdown> = focus_map
            .into_iter()
            .map(|(category, active_seconds)| {
                let percentage = if total_active_sec > 0 {
                    (active_seconds as f64 / total_active_sec as f64) * 100.0
                } else {
                    0.0
                };
                FocusBreakdown {
                    category,
                    active_seconds,
                    percentage,
                }
            })
            .collect();

        // 4. Top Running Applications & Packages (User-centric)
        let top_applications = {
            let mut stmt = conn.prepare(
                r#"
                SELECT coalesce(app_name, comm) as app,
                       coalesce(package_name, comm) as pkg,
                       coalesce(context_tag, 'system') as cat,
                       sum(coalesce(duration_ms, 1000)) / 1000 as total_sec
                FROM process_events
                WHERE timestamp_us >= ?1
                  AND comm NOT IN ('tokio-rt-worker', 'notify-rs inoti', 'DedicatedWorker', 'QSGRenderThread', 'qs:traceq0', 'qs:gdrv0', 'ThreadPoolForeg', 'Chrome_IOThread', 'AudioService', 'QQmlThread', 'QQuickPixmapRea', 'gmain', 'Thread (pooled)')
                  AND comm NOT LIKE 'qs:%'
                  AND comm NOT LIKE 'pool-%'
                  AND comm NOT LIKE '%worker%'
                  AND comm NOT LIKE 'Q%'
                  AND coalesce(app_name, comm) NOT LIKE '%thread%'
                  AND coalesce(app_name, comm) NOT LIKE '%pooled%'
                GROUP BY app
                ORDER BY total_sec DESC
                LIMIT 10
                "#,
            )?;
            let rows = stmt.query_map(params![since_us], |r| {
                let display_name: String = r.get(0)?;
                let package_name: String = r.get(1)?;
                let category: String = r.get(2)?;
                let sec: i64 = r.get(3)?;
                Ok((display_name, package_name, category, sec.max(0) as u64))
            })?;
            let mut list = Vec::new();
            let mut total_app_sec = 0;
            for r in rows {
                let item = r?;
                total_app_sec += item.3;
                list.push(item);
            }
            list.into_iter().map(|(display_name, package_name, category, active_seconds)| {
                let percentage = if total_app_sec > 0 {
                    (active_seconds as f64 / total_app_sec as f64) * 100.0
                } else {
                    0.0
                };
                ApplicationBreakdown {
                    display_name,
                    package_name,
                    category,
                    active_seconds,
                    percentage,
                }
            }).collect::<Vec<_>>()
        };

        // 5. Top Tools & Invocations
        let top_tools = {
            let mut stmt = conn.prepare(
                r#"
                SELECT coalesce(app_name, comm) as app,
                       coalesce(package_name, comm) as pkg,
                       comm,
                       count(*) as invocations,
                       sum(coalesce(duration_ms, 0)) as total_dur,
                       avg(coalesce(duration_ms, 0)) as avg_dur,
                       max(coalesce(duration_ms, 0)) as max_dur,
                       coalesce(context_tag, 'tool') as tag
                FROM process_events
                WHERE timestamp_us >= ?1
                  AND comm NOT IN ('tokio-rt-worker', 'notify-rs inoti', 'DedicatedWorker', 'QSGRenderThread', 'qs:traceq0', 'qs:gdrv0', 'ThreadPoolForeg', 'Chrome_IOThread', 'AudioService', 'QQmlThread', 'QQuickPixmapRea', 'gmain', 'Thread (pooled)')
                  AND comm NOT LIKE 'qs:%'
                  AND comm NOT LIKE 'pool-%'
                  AND comm NOT LIKE '%worker%'
                  AND comm NOT LIKE 'Q%'
                  AND coalesce(app_name, comm) NOT LIKE '%thread%'
                  AND coalesce(app_name, comm) NOT LIKE '%pooled%'
                GROUP BY app
                ORDER BY invocations DESC
                LIMIT 10
                "#,
            )?;
            let rows = stmt.query_map(params![since_us], |r| {
                let app_name: String = r.get(0)?;
                let package_name: String = r.get(1)?;
                let comm: String = r.get(2)?;
                let invocations: i64 = r.get(3)?;
                let total_dur: i64 = r.get(4)?;
                let max_dur: i64 = r.get(6)?;
                Ok(ToolMetric {
                    comm,
                    app_name,
                    package_name,
                    invocations: invocations as u64,
                    total_duration_ms: total_dur as u64,
                    avg_duration_ms: r.get(5)?,
                    max_duration_ms: max_dur as u64,
                    context_tag: r.get(7)?,
                })
            })?;
            let mut list = Vec::new();
            for r in rows {
                list.push(r?);
            }
            list
        };

        // 6. Bottlenecks (highest max/average latency)
        let bottlenecks = {
            let mut stmt = conn.prepare(
                r#"
                SELECT coalesce(app_name, comm) as app,
                       coalesce(package_name, comm) as pkg,
                       comm,
                       count(*) as invocations,
                       sum(duration_ms) as total_dur,
                       avg(duration_ms) as avg_dur,
                       max(duration_ms) as max_dur,
                       coalesce(context_tag, 'tool') as tag
                FROM process_events
                WHERE timestamp_us >= ?1 AND duration_ms IS NOT NULL AND duration_ms > 100
                  AND comm NOT IN ('tokio-rt-worker', 'notify-rs inoti', 'DedicatedWorker', 'QSGRenderThread', 'qs:traceq0', 'qs:gdrv0', 'ThreadPoolForeg', 'Chrome_IOThread', 'AudioService', 'QQmlThread', 'QQuickPixmapRea', 'gmain', 'Thread (pooled)')
                  AND comm NOT LIKE 'qs:%'
                  AND comm NOT LIKE 'pool-%'
                  AND comm NOT LIKE '%worker%'
                  AND comm NOT LIKE 'Q%'
                  AND coalesce(app_name, comm) NOT LIKE '%thread%'
                  AND coalesce(app_name, comm) NOT LIKE '%pooled%'
                GROUP BY app
                ORDER BY max_dur DESC
                LIMIT 8
                "#,
            )?;
            let rows = stmt.query_map(params![since_us], |r| {
                let app_name: String = r.get(0)?;
                let package_name: String = r.get(1)?;
                let comm: String = r.get(2)?;
                let invocations: i64 = r.get(3)?;
                let total_dur: i64 = r.get(4)?;
                let max_dur: i64 = r.get(6)?;
                Ok(ToolMetric {
                    comm,
                    app_name,
                    package_name,
                    invocations: invocations as u64,
                    total_duration_ms: total_dur as u64,
                    avg_duration_ms: r.get(5)?,
                    max_duration_ms: max_dur as u64,
                    context_tag: r.get(7)?,
                })
            })?;
            let mut list = Vec::new();
            for r in rows {
                list.push(r?);
            }
            list
        };

        // 7. File Hotspots (Filtered to exclude database self-mutation noise)
        let file_hotspots = {
            let mut stmt = conn.prepare(
                r#"
                SELECT file_path, count(*) as cnt, max(timestamp_us) as last_us, project_name
                FROM file_events
                WHERE timestamp_us >= ?1
                  AND file_path NOT LIKE '%dockn%'
                  AND file_path NOT LIKE '%.db%'
                GROUP BY file_path
                ORDER BY cnt DESC
                LIMIT 10
                "#,
            )?;
            let rows = stmt.query_map(params![since_us], |r| {
                let cnt: i64 = r.get(1)?;
                Ok(FileHotspot {
                    file_path: r.get(0)?,
                    change_count: cnt as u64,
                    last_seen_us: r.get(2)?,
                    project_name: r.get(3)?,
                })
            })?;
            let mut list = Vec::new();
            for r in rows {
                list.push(r?);
            }
            list
        };

        // 8. Network Destinations
        let network_destinations = {
            let mut stmt = conn.prepare(
                r#"
                SELECT coalesce(domain, dst_addr) as dest, dst_port,
                       count(*) as cnt, group_concat(DISTINCT comm) as comms
                FROM network_events
                WHERE timestamp_us >= ?1
                GROUP BY dest, dst_port
                ORDER BY cnt DESC
                LIMIT 8
                "#,
            )?;
            let rows = stmt.query_map(params![since_us], |r| {
                let dest: String = r.get(0)?;
                let port: u16 = r.get(1)?;
                let cnt: i64 = r.get(2)?;
                let comm_str: String = r.get(3).unwrap_or_default();
                let comms = comm_str.split(',').map(|s| s.trim().to_string()).collect();
                Ok(NetworkDestination {
                    destination: dest,
                    port,
                    connection_count: cnt as u64,
                    comms,
                })
            })?;
            let mut list = Vec::new();
            for r in rows {
                list.push(r?);
            }
            list
        };

        Ok(SubconsciousReport {
            generated_at_us: now_us,
            timeframe_hours: hours,
            total_events,
            deep_work_score,
            context_switches_per_hour,
            top_applications,
            focus_breakdown,
            project_breakdown,
            deep_work_sessions,
            top_tools,
            bottlenecks,
            file_hotspots,
            network_destinations,
            anomalies_detected,
        })
    }

    pub fn compute_ai_context(storage: &Storage, hours: u32) -> Result<AiContextSummary> {
        let report = Self::compute_report(storage, hours)?;
        let conn = storage.get_raw_connection();
        let now_us = chrono::Utc::now().timestamp_micros();
        let since_us = if hours > 0 {
            now_us - (hours as i64 * 3600 * 1_000_000)
        } else {
            now_us - (4 * 3600 * 1_000_000)
        };

        // Get key modified files
        let mut modified_files = Vec::new();
        {
            let mut stmt = conn.prepare(
                r#"
                SELECT file_path, count(*) as mutations
                FROM file_events
                WHERE timestamp_us >= ?1
                  AND file_path NOT LIKE '%dockn%'
                  AND file_path NOT LIKE '%.db%'
                  AND op_type IN ('CREATE', 'MODIFY')
                GROUP BY file_path
                ORDER BY mutations DESC
                LIMIT 12
                "#,
            )?;
            let rows = stmt.query_map(params![since_us], |r| r.get::<_, String>(0))?;
            for r in rows {
                if let Ok(f) = r {
                    modified_files.push(f);
                }
            }
        }

        let top_tools_used: Vec<String> = report.top_tools.iter().map(|t| t.app_name.clone()).take(6).collect();

        // Build Markdown Standup / LLM Prompt text
        let mut md = String::new();
        md.push_str(&format!("# 🧠 Subconscious Developer Context (Past {} Hours)\n\n", if hours > 0 { hours } else { 24 }));
        md.push_str(&format!("**Deep Work Index**: {:.1}% | **Context Switch Velocity**: {:.1}/hr\n\n", report.deep_work_score, report.context_switches_per_hour));

        md.push_str("### 📂 Active Projects Worked On\n");
        if report.project_breakdown.is_empty() {
            md.push_str("- General system development and configuration\n");
        } else {
            for proj in &report.project_breakdown {
                let hrs = proj.total_seconds as f64 / 3600.0;
                md.push_str(&format!(
                    "- **{}**: {:.1} hrs ({:.1}%) | {} file writes | {} process launches\n",
                    proj.name, hrs, proj.percentage, proj.file_mutations, proj.process_invocations
                ));
            }
        }
        md.push_str("\n");

        md.push_str("### 📝 Key Files Touched\n");
        if modified_files.is_empty() {
            md.push_str("- No workspace files touched recently.\n");
        } else {
            for f in &modified_files {
                md.push_str(&format!("- `{}`\n", f));
            }
        }
        md.push_str("\n");

        md.push_str("### ⚡ Primary Tools & Latency Bottlenecks\n");
        if report.bottlenecks.is_empty() {
            md.push_str("- No significant execution latency bottlenecks detected.\n");
        } else {
            for b in report.bottlenecks.iter().take(5) {
                md.push_str(&format!(
                    "- **{}** ({}): avg {:.1}ms, max {:.2}s ({} runs)\n",
                    b.app_name, b.package_name, b.avg_duration_ms, b.max_duration_ms as f64 / 1000.0, b.invocations
                ));
            }
        }
        md.push_str("\n");

        if report.anomalies_detected > 0 {
            md.push_str(&format!("⚠️ **System Anomaly Notice**: {} background performance anomalies or thrash loops flagged.\n", report.anomalies_detected));
        }

        Ok(AiContextSummary {
            timeframe_hours: hours,
            deep_work_score: report.deep_work_score,
            context_switches_per_hour: report.context_switches_per_hour,
            active_projects: report.project_breakdown,
            modified_files,
            top_tools_used,
            bottlenecks: report.bottlenecks,
            standup_markdown: md,
        })
    }
}
