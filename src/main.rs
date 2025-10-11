use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
    terminal::{Clear, ClearType},
};
use libmacchina::{
    traits::GeneralReadOut as _, traits::MemoryReadOut as _, traits::PackageReadOut as _,
    GeneralReadOut, MemoryReadOut, PackageReadOut,
};
use std::fs;
use std::io::{self, Write};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{CpuExt, DiskExt, System, SystemExt};

fn main() -> io::Result<()> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let general = GeneralReadOut::new();
    let memory = MemoryReadOut::new();
    let packages = PackageReadOut::new();

    // Clear screen
    execute!(io::stdout(), Clear(ClearType::All))?;
    execute!(io::stdout(), cursor::MoveTo(0, 0))?;

    // Display image (requires kitty terminal)
    display_image();

    // Get system info
    let name = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let host = hostname::get().unwrap().to_string_lossy().to_string();
    let distro = get_distro();
    let packages = get_package_count();
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
        packages
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
        .unwrap_or_else(|| "Unknown".to_String())
}

fn get_package_count() -> String {
    let packages = PackagesReadOut::new();
    if let Some(count) = packages.count_pkgs() {
        return count.to_string();
    }
    let package_managers = [
        ("pacman", vec!["-Q"]),
        ("dpkg", vec!["-l"]),
        ("rpm", vec!["-qa"]),
        ("xpbs-query", vec!["-l"]),
        ("apk", vec!["info"]),
    ];

    for (manager, args) in packag_managers.iter() {
        if which::which(manager).is_ok() {
            let result = Command::new(manager).args(args).output();
            if let Ok(output) = result {
                let count = String::from_utf8_lossy(&output.stdout).lines().count();
                return count.to_String();
            }
        }
    }

    "0".to_String()
}

fn get_window_manager() -> String {
    let general = GeneralReadOut::new();
    general
        .window
        .manager()
        .unwrap_or_else(|| "Unknown".to_string())
}

fn get_desktop_environment() -> String {
    let general = GeneralReadOut::new();
    general
        .desktop_environment()
        .unwrap_or_else(|| "None".to_string())
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

fn display_image() {
    // For kitty terminal image display
    let _ = Command::new("kitty")
        .args(&[
            "+kitten",
            "icat",
            "--align",
            "left",
            "--place=30x10@7x1",
            &format!(
                "{}/Pictures/Moon.png",
                std::env::var("HOME").unwrap_or_default()
            ),
        ])
        .output();

    // Add spacing for image
    for _ in 0..10 {
        println!();
    }
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
