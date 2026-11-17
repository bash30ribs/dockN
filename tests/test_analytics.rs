use dockN::analytics::AnalyticsEngine;
use dockN::demo_seed;
use dockN::storage::Storage;

#[tokio::test]
async fn test_analytics_report_computation() {
    let temp_dir = std::env::temp_dir().join(format!("dockn_test_analytics_{}", std::process::id()));
    let db_path = temp_dir.join("test_analytics.db");

    let storage = Storage::open(&db_path).expect("Failed to open test storage");
    demo_seed::seed_sample_data(&storage).await.expect("Failed to seed demo data");

    let report = AnalyticsEngine::compute_report(&storage, 24).expect("Failed to compute report");

    assert!(report.total_events > 0);
    assert!(report.deep_work_score > 0.0);
    assert!(!report.top_applications.is_empty());
    assert!(!report.bottlenecks.is_empty());

    // Test AI Context Summary
    let ai_context = AnalyticsEngine::compute_ai_context(&storage, 4).expect("Failed to compute AI context");
    assert!(ai_context.standup_markdown.contains("Subconscious Developer Context"));
    assert!(ai_context.standup_markdown.contains("Active Projects"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
