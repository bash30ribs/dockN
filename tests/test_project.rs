use dockN::project::ProjectDetector;
use std::path::Path;

#[test]
fn test_project_root_detection() {
    let detector = ProjectDetector::get();

    // The current dockN repository contains Cargo.toml and .git
    let dockn_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proj = detector.resolve_project_from_path(dockn_dir);
    assert_eq!(proj.as_deref(), Some("dockN"));

    let src_file = dockn_dir.join("src").join("main.rs");
    let proj_file = detector.resolve_project_from_path(src_file);
    assert_eq!(proj_file.as_deref(), Some("dockN"));
}

#[test]
fn test_window_title_inference() {
    let detector = ProjectDetector::get();

    let title1 = "src/main.rs — dockN — Antigravity IDE";
    assert_eq!(detector.infer_from_window_title(title1).as_deref(), Some("dockN"));

    let title2 = "AquaShield Network Monitor";
    assert_eq!(detector.infer_from_window_title(title2).as_deref(), Some("AquaShield"));

    let title3 = "CIPHER Security Terminal";
    assert_eq!(detector.infer_from_window_title(title3).as_deref(), Some("CIPHER"));

    let title4 = "hyprland.conf — dotfiles";
    assert_eq!(detector.infer_from_window_title(title4).as_deref(), Some("dotfiles"));
}
