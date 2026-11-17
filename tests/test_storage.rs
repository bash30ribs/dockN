use dockN::storage::Storage;

#[test]
fn test_database_initialization_and_status() {
    let temp_dir = std::env::temp_dir().join(format!("dockn_test_{}", std::process::id()));
    let db_path = temp_dir.join("test_subconscious.db");

    let storage = Storage::open(&db_path).expect("Failed to open test storage");
    let status = storage.get_status().expect("Failed to get status");

    assert_eq!(status.total_process_events, 0);
    assert_eq!(status.total_network_events, 0);
    assert_eq!(status.total_file_events, 0);
    assert_eq!(status.total_focus_events, 0);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_command_recording_and_pruning() {
    let temp_dir = std::env::temp_dir().join(format!("dockn_test_cmd_{}", std::process::id()));
    let db_path = temp_dir.join("test_cmd.db");

    let storage = Storage::open(&db_path).expect("Failed to open test storage");

    // Record commands
    storage
        .record_command("cargo", "cargo test", "/home/ribs/For Ide/dockN", 1500, 0)
        .expect("Failed to record command");
    storage
        .record_command("git", "git status", "/home/ribs/For Ide/dockN", 80, 0)
        .expect("Failed to record command");

    let status = storage.get_status().expect("Failed to get status");
    assert_eq!(status.total_process_events, 2);

    // Test export as CSV
    let mut csv_buf = Vec::new();
    let count = storage
        .export_table("process_events", "csv", &mut csv_buf)
        .expect("Failed to export table");
    assert_eq!(count, 2);
    let csv_str = String::from_utf8(csv_buf).unwrap();
    assert!(csv_str.contains("cargo"));
    assert!(csv_str.contains("git"));

    // Test export as JSONL
    let mut json_buf = Vec::new();
    let json_count = storage
        .export_table("process_events", "jsonl", &mut json_buf)
        .expect("Failed to export jsonl");
    assert_eq!(json_count, 2);
    let json_str = String::from_utf8(json_buf).unwrap();
    assert!(json_str.contains("cargo test"));

    // Test pruning (older than 10 days should not delete these new events)
    let prune_res = storage.prune_older_than(10, false).unwrap();
    assert_eq!(prune_res.total_deleted, 0);

    // Test pruning (older than 0 days should delete everything)
    let prune_all = storage.prune_older_than(0, true).unwrap();
    assert_eq!(prune_all.total_deleted, 2);

    let _ = std::fs::remove_dir_all(&temp_dir);
}
