use crate::config::DisplayConfig;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use sysinfo::System;

pub struct SystemInfo {
    pub distro: Option<String>,
    pub age: Option<String>,
    pub kernel: Option<String>,
    pub packages: Option<String>,
    pub shell: Option<String>,
    pub term: Option<String>,
    pub wm: Option<String>,
    pub cpu: Option<String>,
    pub gpu: Option<String>,
    pub theme: Option<String>,
    pub nix: Option<String>,
}

impl SystemInfo {
    pub fn new() -> Self {
        Self {
            distro: None,
            age: None,
            kernel: None,
            packages: None,
            shell: None,
            term: None,
            wm: None,
            cpu: None,
            gpu: None,
            theme: None,
            nix: None,
        }
    }

    pub fn collect_all(&mut self) {
        self.distro = Some(get_os_name());
        self.age = Some(get_system_age());
        self.kernel = System::kernel_version();
        self.packages = Some(get_package_count());
        self.shell = Some(get_shell());
        self.term = Some(get_terminal());
        self.wm = Some(get_window_manager());
        self.cpu = get_cpu_model();
        self.gpu = get_gpu();
        self.theme = get_theme();
        self.nix = get_nix_generation();
    }

    // Helper to convert to vec of tuples for display
    // Check to see if the field is enabled to print
    pub fn to_info_items(
        &self,
        include_age: bool,
        display_config: &DisplayConfig,
    ) -> Vec<(&str, String)> {
        let mut items = Vec::new();

        // Helper to truncate long strings
        fn truncate(s: &str, max_len: usize) -> String {
            if s.len() > max_len {
                s[..max_len].to_string()
            } else {
                s.to_string()
            }
        }

        // Macro to conditionally add fields based on config
        macro_rules! add_if_enabled {
            ($field:expr, $label:expr, $enabled:expr, $max_len:expr) => {
                if $enabled {
                    if let Some(ref val) = $field {
                        items.push(($label, truncate(val, $max_len)));
                    }
                }
            };
        }

        // Add all fields using the macro
        add_if_enabled!(self.distro, "distro", display_config.distro, 50);

        // Age is special - only include if requested
        if include_age {
            add_if_enabled!(self.age, "age", display_config.age, 50);
        }

        add_if_enabled!(self.kernel, "kernel", display_config.kernel, 50);
        add_if_enabled!(self.packages, "packages", display_config.packages, 50);
        add_if_enabled!(self.shell, "shell", display_config.shell, 50);
        add_if_enabled!(self.term, "term", display_config.term, 50);
        add_if_enabled!(self.wm, "wm", display_config.wm, 50);
        add_if_enabled!(self.cpu, "cpu", display_config.cpu, 50);
        add_if_enabled!(self.gpu, "gpu", display_config.gpu, 55);
        add_if_enabled!(self.theme, "theme", display_config.theme, 50);
        add_if_enabled!(self.nix, "nix", display_config.nix, 50);

        items
    }
}

// Helper functions

fn get_os_name() -> String {
    use libmacchina::{traits::GeneralReadout as _, GeneralReadout};
    let general = GeneralReadout::new();
    general
        .distribution()
        .unwrap_or_else(|_| general.os_name().unwrap_or_else(|_| "Unknown".to_string()))
}

fn get_system_age() -> String {
    let metadata = fs::metadata("/").ok();
    let install_time = metadata
        .and_then(|m| m.modified().ok())
        .unwrap_or(std::time::UNIX_EPOCH);

    let now = std::time::SystemTime::now();
    let duration = now.duration_since(install_time).unwrap_or_default();
    let days = duration.as_secs() / 86400;

    format!("{} days", days)
}

fn get_package_count() -> String {
    use libmacchina::{traits::PackageReadout as _, PackageReadout};
    let packages = PackageReadout::new();
    let pkg_counts = packages.count_pkgs();

    if !pkg_counts.is_empty() {
        let total: usize = pkg_counts.iter().map(|(_, count)| count).sum();
        return total.to_string();
    }

    let package_managers = [
        ("guix", vec!["package", "--list-installed"]),
        ("slackpkg", vec!["search"]),
    ];

    for (manager, args) in package_managers.iter() {
        if which::which(manager).is_ok() {
            let result = Command::new(manager).args(args).output();
            if let Ok(output) = result {
                let count = String::from_utf8_lossy(&output.stdout).lines().count();
                if count > 0 {
                    return count.to_string();
                }
            }
        }
    }

    "0".to_string()
}

fn get_shell() -> String {
    use libmacchina::{
        traits::{GeneralReadout as _, ShellFormat, ShellKind},
        GeneralReadout,
    };
    let general = GeneralReadout::new();
    general
        .shell(ShellFormat::Relative, ShellKind::Default)
        .unwrap_or_else(|_| "Unknown".to_string())
}

fn get_terminal() -> String {
    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        return match term_program.to_lowercase().as_str() {
            "ghostty" => "Ghostty".to_string(),
            "kitty" => "Kitty".to_string(),
            "wezterm" => "Wezterm".to_string(),
            "alacritty" => "Alacritty".to_string(),
            "foot" => "󰽒".to_string(),
            _ => term_program,
        };
    }

    use libmacchina::{traits::GeneralReadout as _, GeneralReadout};
    let general = GeneralReadout::new();
    let term = general.terminal().unwrap_or_else(|_| "Unknown".to_string());

    match term.as_str() {
        t if t.contains("ghostty") => "Ghostty".to_string(),
        t if t.contains("kitty") => "meow".to_string(),
        _ => term,
    }
}

fn get_window_manager() -> String {
    if let Ok(wm_env) = std::env::var("XDG_CURRENT_DESKTOP") {
        return match wm_env.to_lowercase().as_str() {
            "hyprland" => "Hyprland".to_string(),
            "sway" => "Sway".to_string(),
            _ => wm_env,
        };
    }

    use libmacchina::{traits::GeneralReadout as _, GeneralReadout};
    let general = GeneralReadout::new();
    general
        .window_manager()
        .unwrap_or_else(|_| "Unknown".to_string())
}

fn get_cpu_model() -> Option<String> {
    let sys = System::new_all();
    sys.cpus().first().map(|cpu| {
        let brand = cpu.brand().trim();
        brand
            .replace("(R)", "")
            .replace("(TM)", "")
            .replace("  ", " ")
            .trim()
            .to_string()
    })
}

fn get_gpu() -> Option<String> {
    if let Ok(output) = Command::new("lspci").output() {
        let lspci_output = String::from_utf8_lossy(&output.stdout);
        for line in lspci_output.lines() {
            if line.contains("VGA compatible controller") || line.contains("3D controller") {
                if let Some(gpu_part) = line.split(':').nth(2) {
                    let gpu = gpu_part.trim();
                    let cleaned = gpu
                        .replace("NVIDIA Corporation", "NVIDIA")
                        .replace("Advanced Micro Devices, Inc. [AMD/ATI]", "AMD")
                        .replace("Advanced Micro Devices, Inc.", "AMD")
                        .replace("Intel Corporation", "Intel")
                        .replace("[AMD/ATI]", "")
                        .trim()
                        .to_string();
                    return Some(cleaned);
                }
            }
        }
    }
    None
}

fn get_theme() -> Option<String> {
    if let Ok(theme) = std::env::var("GTK_THEME") {
        return Some(theme);
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let gtk3_config = format!("{}/.config/gtk-3.0/settings.ini", home);

    if let Ok(contents) = fs::read_to_string(&gtk3_config) {
        for line in contents.lines() {
            if line.starts_with("gtk-theme-name") {
                if let Some(theme) = line.split('=').nth(1) {
                    return Some(theme.trim().to_string());
                }
            }
        }
    }

    None
}

fn get_nix_generation() -> Option<String> {
    if !PathBuf::from("/etc/NIXOS").exists() && !PathBuf::from("/run/current-system").exists() {
        return None;
    }

    // Helper function to extract generation number from path like "system-123-link"
    fn extract_generation(path: &str) -> Option<String> {
        // Split by '-' and find the numeric part
        let parts: Vec<&str> = path.split('-').collect();
        for part in parts {
            if part.chars().all(|c| c.is_numeric()) && !part.is_empty() {
                return Some(part.to_string());
            }
        }
        None
    }

    if let Ok(link) = fs::read_link("/nix/var/nix/profiles/system") {
        if let Some(link_str) = link.to_str() {
            if let Some(gen) = extract_generation(link_str) {
                return Some(gen);
            }
        }
    }

    if let Ok(link) = fs::read_link("/run/current-system") {
        if let Some(link_str) = link.to_str() {
            if let Some(gen) = extract_generation(link_str) {
                return Some(gen);
            }
        }
    }

    None
}
