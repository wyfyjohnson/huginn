use clap::Parser;
use crossterm::{
    cursor, execute,
    style::Stylize,
    terminal::{Clear, ClearType},
};
use libmacchina::{
    traits::{GeneralReadout as _, PackageReadout as _, ShellFormat, ShellKind},
    GeneralReadout, PackageReadout,
};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Disks, System};
use viuer::{print_from_file, Config};
mod challenge;

#[derive(Parser)]
#[command(name = "huginn")]
#[command(about = "A beautiful system information fetcher", long_about = None)]
struct Cli {
    #[arg(short, long)]
    challenge: bool,
    /// Number of years for the challenge
    #[arg(long, default_value_t = 2)]
    years: i64,

    /// Number of months for the challenge
    #[arg(long, default_value_t = 0)]
    months: i64,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // Clear screen
    execute!(io::stdout(), Clear(ClearType::All))?;
    execute!(io::stdout(), cursor::MoveTo(0, 0))?;

    // Run normal fetch (with offset if in box)
    run_fetch_internal(cli.challenge)?;

    // Add challenge box if needed
    if cli.challenge {
        challenge::run_challenge_countdown(cli.years, cli.months);
        draw_outer_box()?;
        println!();
    }
    Ok(())
}

fn draw_outer_box() -> io::Result<()> {
    let box_width = 85;
    let box_height = 28;

    // Top border
    execute!(io::stdout(), cursor::MoveTo(2, 1))?;
    print!("╭{}╮", "─".repeat(box_width));

    // Side borders
    for row in 2..=(box_height + 1) {
        execute!(io::stdout(), cursor::MoveTo(2, row as u16))?;
        print!("│");
        execute!(
            io::stdout(),
            cursor::MoveTo((box_width + 3) as u16, row as u16)
        )?;
        print!("│");
    }

    // Bottom border
    execute!(io::stdout(), cursor::MoveTo(2, (box_height + 2) as u16))?;
    print!("╰{}╯", "─".repeat(box_width));

    Ok(())
}

fn run_fetch_internal(in_box: bool) -> io::Result<()> {
    let offset_x = if in_box { 4 } else { 0 };

    let mut sys = System::new_all();
    sys.refresh_all();

    let distro = get_os_name();
    let name = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let package_count = get_package_count();
    let wm = get_window_manager();
    let term = get_terminal();
    let uptime = format_uptime(System::uptime());
    let age_val = get_system_age();
    let kernel = System::kernel_version().unwrap_or_default();

    let info_items = vec![
        ("distro", distro.clone()),
        ("age", age_val),
        ("kernel", kernel),
        ("packages", package_count),
        ("shell", get_shell()),
        ("term", term),
        ("wm", wm),
    ];

    let info_lines = format_system_info(info_items);
    let first_line = &info_lines[0];
    let dot_position = first_line.find('•').unwrap_or(20);
    let visual_center = dot_position.saturating_sub(10);

    display_logo(&distro, visual_center + offset_x);

    let cpu_usage = sys.global_cpu_usage() as i32;
    let ram_usage = ((sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0) as i32;
    let disk_usage = get_disk_usage();

    let colorbar = get_colorbar();
    let colorbar_width = 24;
    let colorbar_padding = visual_center.saturating_sub(colorbar_width / 2);

    if in_box {
        // Use absolute positioning for everything
        let mut row = 13;

        // Colorbar
        execute!(
            io::stdout(),
            cursor::MoveTo((offset_x + colorbar_padding) as u16, row)
        )?;
        print!("{}", colorbar);
        row += 2;

        // Greeting
        let greeting_padding = visual_center.saturating_sub(10);
        execute!(
            io::stdout(),
            cursor::MoveTo((offset_x + greeting_padding) as u16, row)
        )?;
        print!("{} {}", "Hi!".cyan(), name.green().bold());
        row += 1;

        // Uptime
        let uptime_padding = visual_center.saturating_sub(10);
        execute!(
            io::stdout(),
            cursor::MoveTo((offset_x + uptime_padding) as u16, row)
        )?;
        print!("{} {}", "up".yellow(), uptime.cyan().bold());
        row += 2;

        // System info
        for line in &info_lines {
            execute!(io::stdout(), cursor::MoveTo(offset_x as u16, row))?;
            print!("{}", line);
            row += 1;
        }
        row += 1;

        // Progress bars
        let progress_padding = dot_position + 2;
        execute!(
            io::stdout(),
            cursor::MoveTo((offset_x + progress_padding.saturating_sub(23)) as u16, row)
        )?;
        print!(
            "{}  {:>2}% {}",
            "cpu".green(),
            cpu_usage,
            draw_progress(cpu_usage, 14)
        );
        row += 1;

        execute!(
            io::stdout(),
            cursor::MoveTo((offset_x + progress_padding.saturating_sub(23)) as u16, row)
        )?;
        print!(
            "{}  {:>2}% {}",
            "ram".green(),
            ram_usage,
            draw_progress(ram_usage, 14)
        );
        row += 1;

        execute!(
            io::stdout(),
            cursor::MoveTo((offset_x + progress_padding.saturating_sub(23)) as u16, row)
        )?;
        print!(
            "{} {:>2}% {}",
            "disk".green(),
            disk_usage,
            draw_progress(disk_usage, 14)
        );

        use std::io::Write;
        std::io::stdout().flush()?;
    } else {
        // Normal mode: use println!
        println!("\n{}{}", " ".repeat(colorbar_padding + offset_x), colorbar);
        println!();

        let greeting_padding = visual_center.saturating_sub(10);
        println!(
            "{}{} {}",
            " ".repeat(greeting_padding + offset_x),
            "Hi!".cyan(),
            name.green().bold()
        );

        let uptime_padding = visual_center.saturating_sub(10);
        println!(
            "{}{} {}",
            " ".repeat(uptime_padding + offset_x),
            "up".yellow(),
            uptime.cyan().bold()
        );
        println!();

        for line in info_lines {
            println!("{}{}", " ".repeat(offset_x), line);
        }
        println!();

        let progress_padding = dot_position + 2;
        println!(
            "{}{}  {:>2}% {}",
            " ".repeat(offset_x + progress_padding.saturating_sub(23)),
            "cpu".green(),
            cpu_usage,
            draw_progress(cpu_usage, 14)
        );
        println!(
            "{}{}  {:>2}% {}",
            " ".repeat(offset_x + progress_padding.saturating_sub(23)),
            "ram".green(),
            ram_usage,
            draw_progress(ram_usage, 14)
        );
        println!(
            "{}{} {:>2}% {}",
            " ".repeat(offset_x + progress_padding.saturating_sub(23)),
            "disk".green(),
            disk_usage,
            draw_progress(disk_usage, 14)
        );
    }

    Ok(())
}

fn draw_progress(percentage: i32, size: usize) -> String {
    let filled = (percentage * size as i32 / 100) as usize;
    let full = "━".repeat(filled);
    let empty = "━".repeat(size.saturating_sub(filled));

    let colored_full = match percentage {
        90..=100 => full.dark_red(),
        70..=89 => full.red(),
        50..=69 => full.yellow(),
        30..=49 => full.dark_green(),
        _ => full.green(),
    };

    format!("{}{}", colored_full, empty.dark_grey())
}

fn format_system_info(items: Vec<(&str, String)>) -> Vec<String> {
    let max_label_width = items
        .iter()
        .map(|(label, _)| label.len())
        .max()
        .unwrap_or(0);

    items
        .iter()
        .map(|(label, value)| {
            format!(
                "{} {: >width$} {} {}",
                " ".repeat(10),
                label,
                "•".green(),
                value,
                width = max_label_width
            )
        })
        .collect()
}

fn get_colorbar() -> String {
    use crossterm::style::Stylize;
    let first_blocks = ["░", "▒"];
    let middle_blocks = ["▓", "▒"];
    let last_blocks = ["▒", "░"];
    let mut bar = String::new();

    // Helper macro to add colors with specific block pattern
    macro_rules! add_colors {
        (first: $color:ident) => {
            for block in &first_blocks {
                bar.push_str(&format!("{}", block.$color()));
            }
        };
        (middle: $color:ident) => {
            for block in &middle_blocks {
                bar.push_str(&format!("{}", block.$color()));
            }
        };
        (last: $color:ident) => {
            for block in &last_blocks {
                bar.push_str(&format!("{}", block.$color()));
            }
        };
    }

    add_colors!(first: dark_red);
    add_colors!(middle: red);
    add_colors!(middle: dark_yellow);
    add_colors!(middle: yellow);
    add_colors!(middle: dark_green);
    add_colors!(middle: green);
    add_colors!(middle: dark_cyan);
    add_colors!(middle: cyan);
    add_colors!(middle: dark_blue);
    add_colors!(middle: blue);
    add_colors!(middle: dark_magenta);
    add_colors!(last: magenta);

    bar
}

fn get_os_name() -> String {
    let general = GeneralReadout::new();
    general
        .distribution()
        .unwrap_or_else(|_| general.os_name().unwrap_or_else(|_| "Unknown".to_string()))
}

fn get_logo_path(distro: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    let data_dir =
        std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{}/.local/share", home));

    let logo_name = match distro.to_lowercase().as_str() {
        d if d.contains("arch") => "arch.svg",
        d if d.contains("debian") => "debian.svg",
        d if d.contains("endeavour") => "endeavouros.svg",
        d if d.contains("fedora") => "fedora.svg",
        d if d.contains("garuda") => "garuda.svg",
        d if d.contains("gentoo") => "gentoo.svg",
        d if d.contains("guix") => "guix.svg",
        d if d.contains("lmde") => "lmde.svg",
        d if d.contains("manjaro") => "manjaro.svg",
        d if d.contains("mint") => "mint.svg",
        d if d.contains("nixos") => "nixos.svg",
        d if d.contains("obsidian") => "obsidian.svg",
        d if d.contains("popos") => "popos.svg",
        d if d.contains("ubuntu") => "ubuntu.svg",
        d if d.contains("venom") => "venom.svg",
        _ => "linux.svg",
    };

    PathBuf::from(format!("{}/huginn/logos/{}", data_dir, logo_name))
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

fn display_logo(distro: &str, dot_position: usize) {
    let svg_path = get_logo_path(distro);
    let logo_x = (dot_position as u16).saturating_sub(10);

    let conf = Config {
        width: Some(20),
        height: Some(10),
        x: logo_x,
        y: 3,
        absolute_offset: true,
        transparent: true,
        ..Default::default()
    };

    // Check if SVG exists and convert to PNG
    if svg_path.exists() {
        if let Some(png_path) = svg_to_png_temp(&svg_path, 400, 400) {
            let _ = print_from_file(&png_path, &conf);
            let _ = std::fs::remove_file(png_path);
        }
    } else {
        let home = std::env::var("HOME").unwrap_or_default();
        let data_dir =
            std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{}/.local/share", home));
        let fallback_path = PathBuf::from(format!("{}/huginn/logos/linux.svg", data_dir));

        if fallback_path.exists() {
            if let Some(png_path) = svg_to_png_temp(&fallback_path, 400, 400) {
                let _ = print_from_file(&png_path, &conf);
                let _ = std::fs::remove_file(png_path);
            }
        } else {
            eprintln!("No logo found: {:?}", data_dir);
            eprintln!("Place logos in: {}/huginn/logos", data_dir);
        }
    }
}

fn get_package_count() -> String {
    let packages = PackageReadout::new();
    let pkg_counts = packages.count_pkgs();

    // count_pkgs() returns Vec<(PackageManager, usize)>
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

fn get_window_manager() -> String {
    if let Ok(wm_env) = std::env::var("XDG_CURRENT_DESKTOP") {
        return match wm_env.to_lowercase().as_str() {
            "hyprland" => "Hyprland".to_string(),
            "sway" => "Sway".to_string(),
            _ => wm_env,
        };
    }

    let general = GeneralReadout::new();
    general
        .window_manager()
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

    // Fallback to libmacchina detection
    let general = GeneralReadout::new();
    let term = general.terminal().unwrap_or_else(|_| "Unknown".to_string());

    // Clean up them names
    match term.as_str() {
        t if t.contains("ghostty") => "Ghostty".to_string(),
        t if t.contains("kitty") => "meow".to_string(),
        _ => term,
    }
}

fn get_shell() -> String {
    let general = GeneralReadout::new();
    general
        .shell(ShellFormat::Relative, ShellKind::Default)
        .unwrap_or_else(|_| "Unknown".to_string())
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

fn get_disk_usage() -> i32 {
    let disks = Disks::new_with_refreshed_list();

    disks
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
