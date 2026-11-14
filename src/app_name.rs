use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
pub struct AppInfo {
    pub display_name: String,
    pub package_name: String,
    pub category: String,
}

pub struct AppResolver {
    desktop_map: HashMap<String, AppInfo>,
    package_cache: Mutex<HashMap<PathBuf, String>>,
}

static RESOLVER: OnceLock<AppResolver> = OnceLock::new();

impl AppResolver {
    pub fn get() -> &'static AppResolver {
        RESOLVER.get_or_init(Self::load)
    }

    fn load() -> Self {
        let mut desktop_map = HashMap::new();

        // 1. Dynamically scan all standard FreeDesktop .desktop entry directories
        let desktop_dirs = [
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
            dirs::data_local_dir().unwrap_or_default().join("applications"),
            PathBuf::from("/var/lib/flatpak/exports/share/applications"),
            dirs::data_local_dir().unwrap_or_default().join("flatpak/exports/share/applications"),
            PathBuf::from("/var/lib/snapd/desktop/applications"),
        ];

        for dir in desktop_dirs {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                        let stem = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();

                        if let Ok(content) = fs::read_to_string(&path) {
                            Self::parse_desktop_entry(&stem, &content, &mut desktop_map);
                        }
                    }
                }
            }
        }

        Self {
            desktop_map,
            package_cache: Mutex::new(HashMap::new()),
        }
    }

    fn parse_desktop_entry(file_stem: &str, content: &str, map: &mut HashMap<String, AppInfo>) {
        let mut name = None;
        let mut generic_name = None;
        let mut exec = None;
        let mut categories = None;
        let mut wm_class = None;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed != "[Desktop Entry]" {
                break;
            }
            if let Some(val) = trimmed.strip_prefix("Name=") {
                if name.is_none() {
                    name = Some(val.trim().to_string());
                }
            } else if let Some(val) = trimmed.strip_prefix("GenericName=") {
                if generic_name.is_none() {
                    generic_name = Some(val.trim().to_string());
                }
            } else if let Some(val) = trimmed.strip_prefix("Exec=") {
                if exec.is_none() {
                    exec = Some(val.trim().to_string());
                }
            } else if let Some(val) = trimmed.strip_prefix("Categories=") {
                if categories.is_none() {
                    categories = Some(val.trim().to_string());
                }
            } else if let Some(val) = trimmed.strip_prefix("StartupWMClass=") {
                if wm_class.is_none() {
                    wm_class = Some(val.trim().to_string());
                }
            }
        }

        if let (Some(app_name), Some(exec_cmd)) = (name, exec) {
            let bin_token = exec_cmd
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches('"');

            let bin_name = Path::new(bin_token)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();

            let category = if let Some(cats) = categories {
                Self::categorize_from_xdg(&cats)
            } else {
                "system".to_string()
            };

            let display = if let Some(generic) = generic_name {
                if !generic.is_empty() && !app_name.contains(&generic) {
                    format!("{} ({})", app_name, generic)
                } else {
                    app_name.clone()
                }
            } else {
                app_name.clone()
            };

            let info = AppInfo {
                display_name: display,
                package_name: if !bin_name.is_empty() {
                    bin_name.clone()
                } else {
                    file_stem.to_string()
                },
                category,
            };

            if !bin_name.is_empty() {
                map.insert(bin_name.clone(), info.clone());
                map.insert(bin_name.to_lowercase(), info.clone());

                // Also index without common prefixes/suffixes
                if let Some(stripped) = bin_name.strip_suffix("-bin") {
                    map.insert(stripped.to_string(), info.clone());
                }
            }

            if !file_stem.is_empty() {
                map.insert(file_stem.to_string(), info.clone());
                map.insert(file_stem.to_lowercase(), info.clone());
                if let Some(stripped) = file_stem.strip_suffix("-origin") {
                    map.insert(stripped.to_string(), info.clone());
                }
            }

            if let Some(wm) = wm_class {
                map.insert(wm.to_lowercase(), info);
            }
        }
    }

    fn categorize_from_xdg(cats: &str) -> String {
        let upper = cats.to_uppercase();
        if upper.contains("WEBBROWSER") || upper.contains("NETWORK") {
            "browser".to_string()
        } else if upper.contains("DEVELOPMENT") || upper.contains("IDE") || upper.contains("TEXTEDITOR") || upper.contains("BUILDING") {
            "ide".to_string()
        } else if upper.contains("TERMINALEMULATOR") {
            "terminal".to_string()
        } else if upper.contains("CHAT") || upper.contains("INSTANTMESSAGING") || upper.contains("AUDIOVIDEO") {
            "communication".to_string()
        } else {
            "system".to_string()
        }
    }

    /// Dynamically resolves any process without hardcoded assumptions
    pub fn resolve(
        &self,
        comm: &str,
        exe_path: Option<&Path>,
        cmd: &[String],
    ) -> AppInfo {
        // 1. Extract canonical executable name from on-disk binary
        let exe_base = exe_path
            .and_then(|p| p.file_name())
            .map(|f| f.to_string_lossy().to_string());

        // 2. Dynamic Thread Normalization:
        // If comm does not match exe_base (e.g. comm="DedicatedWorker" vs exe_base="brave",
        // or comm="tokio-rt-worker" vs exe_base="dockn"), it's an internal thread!
        let effective_binary = match &exe_base {
            Some(base) if !base.is_empty() => {
                let is_different = comm != base.as_str()
                    && !base.starts_with(comm)
                    && !comm.starts_with(base.as_str());

                if is_different {
                    base.clone()
                } else {
                    comm.to_string()
                }
            }
            _ => comm.to_string(),
        };

        // 3. Check dynamically indexed .desktop entries
        if let Some(info) = self.desktop_map.get(&effective_binary) {
            return info.clone();
        }
        let lower = effective_binary.to_lowercase();
        if let Some(info) = self.desktop_map.get(&lower) {
            return info.clone();
        }

        // 4. Dynamic Script & Interpreter Detection (e.g. python3 script.py, node server.js)
        if Self::is_script_interpreter(&lower) && cmd.len() > 1 {
            let script_arg = &cmd[1];
            if !script_arg.starts_with('-') {
                let script_name = Path::new(script_arg)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| script_arg.clone());

                let display = format!("{} ({})", script_name, Self::title_case(&effective_binary));
                return AppInfo {
                    display_name: display,
                    package_name: effective_binary,
                    category: "development".to_string(),
                };
            }
        }

        // 5. Dynamic Linux Package Origin Query (pacman / system package manager)
        let package_name = if let Some(path) = exe_path {
            self.query_system_package(path)
        } else {
            effective_binary.clone()
        };

        // 6. Dynamic Category & Title Generation
        let category = Self::infer_category(&effective_binary, &package_name);
        let display_name = if !package_name.is_empty() && package_name != effective_binary {
            format!("{} ({})", Self::title_case(&effective_binary), package_name)
        } else {
            Self::title_case(&effective_binary)
        };

        AppInfo {
            display_name,
            package_name,
            category,
        }
    }

    /// Queries Linux package manager dynamically with thread-safe caching
    fn query_system_package(&self, exe_path: &Path) -> String {
        {
            let cache = self.package_cache.lock().unwrap();
            if let Some(pkg) = cache.get(exe_path) {
                return pkg.clone();
            }
        }

        // Query Arch Linux pacman dynamically
        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = Command::new("pacman")
                .args(["-Qo", &exe_path.to_string_lossy()])
                .output()
            {
                if output.status.success() {
                    let out_str = String::from_utf8_lossy(&output.stdout);
                    // Output format: "/usr/bin/git is owned by git 2.56.0-1"
                    if let Some(idx) = out_str.find("is owned by ") {
                        let remainder = &out_str[idx + 12..];
                        if let Some(pkg) = remainder.split_whitespace().next() {
                            let pkg_str = pkg.trim().to_string();
                            let mut cache = self.package_cache.lock().unwrap();
                            cache.insert(exe_path.to_path_buf(), pkg_str.clone());
                            return pkg_str;
                        }
                    }
                }
            }
        }

        let fallback = exe_path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let mut cache = self.package_cache.lock().unwrap();
        cache.insert(exe_path.to_path_buf(), fallback.clone());
        fallback
    }

    fn is_script_interpreter(bin: &str) -> bool {
        bin.starts_with("python")
            || bin.starts_with("node")
            || bin.starts_with("ruby")
            || bin.starts_with("perl")
            || bin == "deno"
            || bin == "bun"
            || bin == "bash"
            || bin == "zsh"
            || bin == "sh"
    }

    fn infer_category(bin: &str, pkg: &str) -> String {
        let text = format!("{} {}", bin, pkg).to_lowercase();
        if text.contains("browser") || text.contains("chrome") || text.contains("firefox") || text.contains("brave") {
            "browser".to_string()
        } else if text.contains("code") || text.contains("nvim") || text.contains("vim") || text.contains("ide") || text.contains("editor") {
            "ide".to_string()
        } else if text.contains("term") || text.contains("bash") || text.contains("zsh") || text.contains("shell") {
            "terminal".to_string()
        } else if text.contains("cargo") || text.contains("rust") || text.contains("git") || text.contains("compiler") || text.contains("build") || text.contains("python") || text.contains("node") {
            "development".to_string()
        } else if text.contains("hyprland") || text.contains("waybar") || text.contains("compositor") || text.contains("desktop") {
            "desktop".to_string()
        } else {
            "system".to_string()
        }
    }

    fn title_case(bin: &str) -> String {
        if bin.is_empty() {
            return "Unknown App".to_string();
        }
        bin.split(|c| c == '-' || c == '_')
            .filter(|s| !s.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}
