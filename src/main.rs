use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
    terminal::{Clear, ClearType},
};
use libmacchina::{
    traits::GeneralReadOut as _, traits::PackageReadOut as _, GeneralReadOut, PackageReadOut,
};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{CpuExt, DiskExt, System, SystemExt};
use viuer::{print_from_file, Config};

fn main() -> io::Result<()> {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Clear screen
    execute!(io::stdout(), Clear(ClearType::All))?;
    execute!(io::stdout(), cursor::MoveTo(0, 0))?;

    // Get distro first for logo selection
    let distro = get_distro();

    // Display logo
    display_logo(&distro);

    // Get system info
    let name = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let package_count = get_package_count();
    let wm = get_window_manager();
    let storage = get_storage_used();
    let term = get_terminal();
    let uptime = format_uptime(sys.uptime());
    let age_val = get_system_age();

    // CPU, RAM, DISK usage
    let cpu_usage = sys.global_cpu_info().cpu_usage() as i32;
    let ram_usage = ((sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0) as i32;
    let disk_usage = get_disk_usage(&mut sys);

    // Print colorbar
    println!("\n    {}", get_colorbar());
    println!();

    // Greetings
    println!(
        "                     {} {}",
        "H!".bright_cyan(),
        name.green().bold()
    );
    println!(
        "                   {} {}",
        "up".bright_yellow(),
        uptime.cyan().bold()
    );
    println!();

    // System info
    println!("{}         distro {} {}", "".yellow(), "•".green(), distro);
    println!("{}            age {} {}", "".yellow(), "•".green(), age_val);
    println!(
        "{}         kernel {} {}",
        "".yellow(),
        "•".green(),
        sys.kernel_version().unwrap_or_default()
    );
    println!(
        "{}       packages {} {}",
        "".yellow(),
        "•".green(),
        package_count
    );
    println!(
        "{}          shell {} {}",
        "".yellow(),
        "•".green(),
        get_shell()
    );
    println!("{}           term {} {}", "".yellow(), "•".green(), term);
    println!("{}             wm {} {}", "".yellow(), "•".green(), wm);
    println!();

    // Progress bars
    println!("        {}", format_stat("cpu", cpu_usage));
    println!("        {}", format_stat("ram", ram_usage));
    println!("       {}", format_stat("disk", disk_usage));

    // Wait for input
    wait_for_keypress();

    Ok(())
}

fn draw_progress(percentage: i32, size: usize) -> String {
    let filled = (percentage * size as i32 / 100) as usize;
    let full = "━".repeat(filled);
    let empty = "━".repeat(size.saturating_sub(filled));
    format!("{}{}", full.magenta(), empty.white())
}

fn format_stat(name: &str, value: i32) -> String {
    format!("{} {}% {}", name.green(), value, draw_progress(value, 14))
}

fn get_colorbar() -> String {
    format!(
        "{}{}{}{}{}{}{}{}",
        "░▒".red(),
        "█▓▒░".on_red().bright_red(),
        "█▓▒░".on_green().bright_green(),
        "█▓▒░".on_yellow().bright_yellow(),
        "█▓▒░".on_blue().bright_blue(),
        "█▓▒░".on_magenta().bright_magenta(),
        "█▓▒░".on_cyan().bright_cyan(),
        "█▒░".on_white().bright_white()
    )
}

fn get_distro() -> String {
    let general = GeneralReadOut::new();
    general
        .distribution()
        .unwrap_or_else(|| "Unknown".to_string())
}

fn get_logo_path(distro: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    let config_dir =
        std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", home));

    let logo_name = match distro.to_lowercase().as_str() {
        d if d.contains("arch") => "arch.svg",
        d if d.contains("guix") => "guix.svg",
        d if d.contains("gentoo") => "gentoo.svg",
        d if d.contains("obsidian") => "obsidian.svg",
        d if d.contains("popos") => "popos.svg",
        d if d.contains("venom") => "venom.svg",
        d if d.contains("mint") => "mint.svg",
        d if d.contains("lmde") => "lmde.svg",
        d if d.contains("nixos") => "nixos.svg",
        d if d.contains("ubuntu") => "ubuntu.svg",
        d if d.contains("fedora") => "fedora.svg",
        d if d.contains("debian") => "debian.svg",
        d if d.contains("manjaro") => "manjaro.svg",
        d if d.contains("garuda") => "garuda.svg",
        d if d.contains("endeavour") => "endeavouros.svg",
        _ => "linux.svg",
    };

    PathBuf::from(format!("{}/huginn/logos/{}", config_dir, logo_name))
}

fn svg_to_png_temp(svg_path: &PathBuf, width: u32, height: u32) -> Option<PathBuf> {
    use resvg::usvg;

    let svg_data = std::fs::read(svg_path).ok()?;
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &options).ok()?;

    let size = tree.size();
    let scale_x = width as f32 / size.width();
    let scale_y = height as f32 / size.height();
    let scale = scale_x.min(scale_y);

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Save to temp file
    let temp_png = PathBuf::from("/tmp/huginn_logo.png");
    pixmap.save_png(&temp_png).ok()?;

    Some(temp_png)
}

fn display_logo(distro: &str) {
    let svg_path = get_logo_path(distro);

    let conf = Config {
        width: Some(20),
        height: Some(10),
        x: 2,
        y: 1,
        absolute_offset: false,
        transparent: true,
        ..Default::default()
    };

    // Check if SVG exists and convert to PNG
    if svg_path.exists() {
        if let Some(png_path) = svg_to_png_temp(&svg_path, 400, 400) {
            let _ = print_from_file(&png_path, &conf);
            // Cleanup temp file
            let _ = std::fs::remove_file(png_path);
        }
    } else {
        eprintln!("Logo not found: {:?}", svg_path);
    }

    // Add spacing for content below image
    for _ in 0..2 {
        println!();
    }
}

fn get_package_count() -> String {
    let packages = PackageReadOut::new();
    if let Some(count) = packages.count_pkgs() {
        return count.to_string();
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
                return count.to_string();
            }
        }
    }

    "0".to_string()
}

fn get_window_manager() -> String {
    let general = GeneralReadOut::new();
    general
        .window_manager()
        .unwrap_or_else(|| "Unknown".to_string())
}

fn get_storage_used() -> String {
    Command::new("df")
        .args(&["-h", "--output=used", "/"])
        .output()
        .ok()
        .and_then(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .nth(1)
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "0G".to_string())
}

fn get_terminal() -> String {
    let general = GeneralReadOut::new();
    general.terminal().unwrap_or_else(|| "Unknown".to_string())
}

fn get_shell() -> String {
    let general = GeneralReadOut::new();
    general
        .shell(
            libmacchina::ShellFormat::Relative,
            libmacchina::ShellKind::Default,
        )
        .unwrap_or_else(|| "Unknown".to_string())
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;

    if days > 0 {
        format!("{} days, {} hrs", days, hours)
    } else if hours > 0 {
        format!("{} hrs, {} mins", hours, minutes)
    } else {
        format!("{} mins", minutes)
    }
}

fn get_system_age() -> String {
    let metadata = fs::metadata("/").ok();
    let install_time = metadata
        .and_then(|m| m.modified().ok())
        .unwrap_or(UNIX_EPOCH);

    let now = SystemTime::now();
    let duration = now.duration_since(install_time).unwrap_or_default();
    let days = duration.as_secs() / 86400;

    format!("{} days", days)
}

fn get_disk_usage(sys: &mut System) -> i32 {
    sys.refresh_disks_list();
    sys.disks()
        .iter()
        .find(|d| d.mount_point().to_str() == Some("/"))
        .map(|d| {
            let total = d.total_space();
            let available = d.available_space();
            let used = total - available;
            ((used as f64 / total as f64) * 100.0) as i32
        })
        .unwrap_or(0)
}

fn wait_for_keypress() {
    use crossterm::event::{read, Event, KeyCode};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

    let _ = enable_raw_mode();
    loop {
        if let Ok(Event::Key(key)) = read() {
            if matches!(key.code, KeyCode::Enter | KeyCode::Char(_)) {
                break;
            }
        }
    }
    let _ = disable_raw_mode();
}
