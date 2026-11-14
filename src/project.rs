use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

static PROJECT_DETECTOR: OnceLock<Arc<ProjectDetector>> = OnceLock::new();

pub struct ProjectDetector {
    cache: Mutex<HashMap<PathBuf, Option<String>>>,
    home_dir: Option<PathBuf>,
}

impl ProjectDetector {
    pub fn get() -> Arc<Self> {
        PROJECT_DETECTOR
            .get_or_init(|| {
                Arc::new(Self {
                    cache: Mutex::new(HashMap::new()),
                    home_dir: dirs::home_dir(),
                })
            })
            .clone()
    }

    /// Resolve project name from an arbitrary file path or directory
    pub fn resolve_project_from_path<P: AsRef<Path>>(&self, path: P) -> Option<String> {
        let p = path.as_ref();
        let dir = if p.is_dir() {
            p.to_path_buf()
        } else {
            p.parent()?.to_path_buf()
        };

        // Check thread-safe cache
        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(&dir) {
                return cached.clone();
            }
        }

        let detected = self.detect_project_root(&dir);

        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(dir, detected.clone());
        }

        detected
    }

    /// Infer project name from window title (e.g. "Cargo.toml — dockN — Antigravity IDE")
    pub fn infer_from_window_title(&self, title: &str) -> Option<String> {
        let lower = title.to_lowercase();
        if lower.contains("dockn") {
            return Some("dockN".to_string());
        }
        if lower.contains("aquashield") {
            return Some("AquaShield".to_string());
        }
        if lower.contains("cipher") {
            return Some("CIPHER".to_string());
        }
        if lower.contains("dotfiles") || lower.contains("hyprland.conf") {
            return Some("dotfiles".to_string());
        }

        // Try extracting between separators like " — " or " - "
        for part in title.split(&['—', '-', '|', '•'][..]) {
            let trimmed = part.trim();
            if trimmed.len() >= 3 && trimmed.len() <= 24 && !trimmed.contains('/') && !trimmed.contains('\\') {
                let t_low = trimmed.to_lowercase();
                if !t_low.contains("ide") && !t_low.contains("editor") && !t_low.contains("code") && !t_low.contains("brave") && !t_low.contains("terminal") {
                    return Some(trimmed.to_string());
                }
            }
        }

        None
    }

    fn detect_project_root(&self, start_dir: &Path) -> Option<String> {
        let mut curr = start_dir;
        let home = self.home_dir.as_deref();

        while let Some(parent) = curr.parent() {
            // Stop if reached system root or home directory root directly
            if curr == Path::new("/") || Some(curr) == home {
                break;
            }

            // 1. Check for Git repo
            if curr.join(".git").exists() {
                return curr.file_name().map(|n| n.to_string_lossy().to_string());
            }

            // 2. Check for language build files
            if curr.join("Cargo.toml").exists()
                || curr.join("package.json").exists()
                || curr.join("pyproject.toml").exists()
                || curr.join("go.mod").exists()
                || curr.join("CMakeLists.txt").exists()
            {
                return curr.file_name().map(|n| n.to_string_lossy().to_string());
            }

            // 3. Check for dotfiles in ~/.config
            if let Some(h) = home {
                if curr.starts_with(h.join(".config")) {
                    return Some("dotfiles".to_string());
                }
            }

            curr = parent;
        }

        None
    }
}
