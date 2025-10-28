use clap::Parser;
use crossterm::{
    cursor, execute,
    style::Stylize,
    terminal::{Clear, ClearType},
};
use std::io;
use std::path::PathBuf;
use sysinfo::{Disks, System};
use viuer::{print_from_file, Config as ViuerConfig};

mod challenge;
mod config;
mod system_info;

use config::{Config, LogoConfig};
use system_info::SystemInfo;

#[derive(Parser)]
#[command(name = "huginn")]
#[command(about = "A beautiful system information fetcher", long_about = None)]
struct Cli {
    #[arg(short, long)]
    challenge: bool,
    /// Number of years for the challenge
    #[arg(short, long)]
    years: Option<i64>,

    /// Number of months for the challenge
    #[arg(short, long)]
    months: Option<i64>,

    // Generate a default config file at XDG config/huginn/config.toml
    #[arg(long)]
    generate_config: bool,
}

struct DisplayContext {
    in_box: bool,
    offset_x: usize,
    visual_center: usize,
}

impl DisplayContext {
    fn print_centered(&self, row: Option<u16>, text: &str, width: usize) -> io::Result<()> {
        let padding = self.visual_center.saturating_sub(width / 2);

        if self.in_box {
            if let Some(r) = row {
                execute!(io::stdout(), cursor::MoveTo(padding as u16, r))?;
            }
            print!("{}", text);
        } else {
            println!("{}{}", " ".repeat(padding + self.offset_x), text);
        }
        Ok(())
    }

    fn print_line(&self, row: Option<u16>, text: &str) -> io::Result<()> {
        if self.in_box {
            if let Some(r) = row {
                execute!(io::stdout(), cursor::MoveTo(self.offset_x as u16, r))?;
            }
            print!("{}", text);
        } else {
            println!("{}{}", " ".repeat(self.offset_x), text);
        }
        Ok(())
    }
}

enum ProgressColorScheme {
    System,
    Challenge,
}

fn expand_home(path: &str) -> String {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return path.replacen("~", &home, 1);
        }
    }
    path.to_string()
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // Handle config generation if requested
    if cli.generate_config {
        match Config::generate_default_config() {
            Ok(_) => return Ok(()),
            Err(e) => {
                eprintln!("Error generating config: {}", e);
                return Ok(());
            }
        }
    }

    // Load configuration
    let config = Config::load();

    // Determine if we're in challenge mode
    // CLI flag overrides config setting
    let in_challenge_mode = cli.challenge || config.display.mode == "challenge";

    // Determine challenge years and months
    // CLI args override config values
    let challenge_years = cli.years.unwrap_or(config.challenge.years);
    let challenge_months = cli.months.unwrap_or(config.challenge.months);

    // Run pre-fetch script if configured
    if !config.scripts.pre_fetch.is_empty() {
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(&config.scripts.pre_fetch)
            .status();
    }

    // Clear screen
    execute!(io::stdout(), Clear(ClearType::All))?;
    execute!(io::stdout(), cursor::MoveTo(0, 0))?;

    // Run normal fetch (with offset if in box)
    let (content_height, second_info_row) = run_fetch_internal(in_challenge_mode, &config)?;

    // Add challenge box if needed
    if in_challenge_mode {
        let challenge_end_row = challenge::run_challenge_countdown(
            challenge_years,
            challenge_months,
            second_info_row,
            &config.display,
        );
        let total_height = content_height.max(challenge_end_row) + 1;
        draw_outer_box(total_height)?;
        println!();
    }

    // Run post-fetch script if configured
    if !config.scripts.post_fetch.is_empty() {
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(&config.scripts.post_fetch)
            .status();
    }

    Ok(())
}
fn draw_outer_box(height: u16) -> io::Result<()> {
    let box_width = 85;

    // Top border
    execute!(io::stdout(), cursor::MoveTo(2, 1))?;
    print!("╭{}╮", "─".repeat(box_width));

    // Side borders
    for row in 2..=(height + 1) {
        execute!(io::stdout(), cursor::MoveTo(2, row))?;
        print!("│");
        execute!(io::stdout(), cursor::MoveTo((box_width + 3) as u16, row))?;
        print!("│");
    }

    // Bottom border
    execute!(io::stdout(), cursor::MoveTo(2, height + 2))?;
    print!("╰{}╯", "─".repeat(box_width));

    Ok(())
}

fn display_greeting(ctx: &DisplayContext, name: &str, row: &mut u16) -> io::Result<()> {
    let greeting_text = format!("Hi! {}", name);
    let greeting_width = greeting_text.len();
    let formatted = format!("{} {}", "Hi!".cyan(), name.green().bold());

    ctx.print_centered(Some(*row), &formatted, greeting_width)?;
    if ctx.in_box {
        *row += 1;
    }
    Ok(())
}

fn display_uptime(ctx: &DisplayContext, uptime: &str, row: &mut u16) -> io::Result<()> {
    let uptime_text = format!("up {}", uptime);
    let uptime_width = uptime_text.len();
    let formatted = format!("{} {}", "up".yellow(), uptime.cyan().bold());

    ctx.print_centered(Some(*row), &formatted, uptime_width)?;
    if ctx.in_box {
        *row += 1;
    }
    Ok(())
}

fn display_progress_bars(
    ctx: &DisplayContext,
    cpu: i32,
    ram: i32,
    disk: i32,
    dot_position: usize,
    row: &mut u16,
) -> io::Result<()> {
    let items = vec![("cpu", cpu, "  "), ("ram", ram, "  "), ("disk", disk, " ")];

    for (label, value, spacing) in items {
        let text = format!(
            "{}{}{:>2}% {}",
            label.green(),
            spacing,
            value,
            draw_progress(value, 14, ProgressColorScheme::System)
        );

        // Calculate visual width (without ANSI codes)
        let visual_width = label.len() + spacing.len() + 3 + 14; // label + spacing + "XX% " + bar

        if ctx.in_box {
            // Center the progress bars like the greeting/uptime
            let padding = ctx.visual_center.saturating_sub(visual_width / 2);
            execute!(io::stdout(), cursor::MoveTo(padding as u16, *row))?;
            print!("{}", text);
            *row += 1;
        } else {
            // Normal mode: keep left-aligned with dot_position
            let progress_padding = dot_position + 2;
            println!(
                "{}{}",
                " ".repeat(ctx.offset_x + progress_padding.saturating_sub(23)),
                text
            );
        }
    }
    Ok(())
}

fn run_fetch_internal(in_box: bool, config: &Config) -> io::Result<(u16, u16)> {
    let offset_x = if in_box { 4 } else { 0 };

    let mut sys = System::new_all();
    sys.refresh_all();

    let name = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let uptime = format_uptime(System::uptime());

    // Collect all system info
    let mut sys_info = SystemInfo::new();
    sys_info.collect_all(&config.display);

    // Convert to info_items, excluding age in box mode
    let info_items = sys_info.to_info_items(!in_box, &config.display);

    let distro = sys_info
        .distro
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());

    let info_lines = format_system_info(info_items);
    let first_line = &info_lines[0];
    let dot_position = first_line.find('•').unwrap_or(20);

    let visual_center = if in_box {
        44 // box width is 85, and starts at x=2
    } else {
        dot_position.saturating_sub(10)
    };

    // Create display context
    let ctx = DisplayContext {
        in_box,
        offset_x,
        visual_center,
    };

    // Use custom logo if configured, otherwise use distro logo
    let logo_height = if !config.logo.custom_path.is_empty() {
        let expand_path = expand_home(&config.logo.custom_path);
        let height = config.logo.height.unwrap_or(18); // Default custom logo height
        display_custom_logo(&expand_path, visual_center, &config.logo);
        height
    } else {
        display_logo(&distro, visual_center);
        10 // Default distro logo height
    };

    let cpu_usage = sys.global_cpu_usage() as i32;
    let ram_usage = ((sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0) as i32;
    let disk_usage = get_disk_usage();

    let colorbar = get_colorbar();
    let colorbar_width = 25;
    let colorbar_padding = visual_center.saturating_sub(colorbar_width / 2);

    let final_row = if in_box {
        // Use absolute positioning for everything
        let mut row = 2 + logo_height as u16 + 2;

        // Colorbar
        execute!(io::stdout(), cursor::MoveTo(colorbar_padding as u16, row))?;
        print!("{}", colorbar);
        row += 2;

        // Greeting and uptime
        display_greeting(&ctx, &name, &mut row)?;
        display_uptime(&ctx, &uptime, &mut row)?;
        row += 1;

        // System info
        let mut second_info_row = 0;
        for (idx, line) in info_lines.iter().enumerate() {
            ctx.print_line(Some(row), line)?;
            if idx == 1 {
                // Second line (index 1)
                second_info_row = row;
            }
            row += 1;
        }
        row += 1;

        // Progress bars
        display_progress_bars(
            &ctx,
            cpu_usage,
            ram_usage,
            disk_usage,
            dot_position,
            &mut row,
        )?;

        use std::io::Write;
        std::io::stdout().flush()?;
        // Keeps progress bar in box hopefully
        let content_end_row = row;

        (content_end_row, second_info_row)
    } else {
        // Normal mode: use println!
        println!("\n{}{}", " ".repeat(colorbar_padding + offset_x), colorbar);
        println!();

        // Greeting and uptime
        let mut row = 0; // Not used in non-box mode but needed for function signature
        display_greeting(&ctx, &name, &mut row)?;
        display_uptime(&ctx, &uptime, &mut row)?;
        println!();

        // System info
        for line in info_lines {
            ctx.print_line(None, &line)?;
        }
        println!();

        // Progress bars
        display_progress_bars(
            &ctx,
            cpu_usage,
            ram_usage,
            disk_usage,
            dot_position,
            &mut row,
        )?;

        (0, 0) // return for normal
    };

    Ok(final_row)
}

fn draw_progress(percentage: i32, size: usize, scheme: ProgressColorScheme) -> String {
    let filled = (percentage * size as i32 / 100) as usize;
    let full = "━".repeat(filled);
    let empty = "━".repeat(size.saturating_sub(filled));

    let colored_full = match scheme {
        ProgressColorScheme::System => match percentage {
            90..=100 => full.dark_red(),
            70..=89 => full.red(),
            50..=69 => full.yellow(),
            30..=49 => full.dark_green(),
            _ => full.green(),
        },
        ProgressColorScheme::Challenge => match percentage {
            90..=100 => full.green(),
            70..=89 => full.dark_green(),
            50..=69 => full.dark_yellow(),
            30..=49 => full.dark_cyan(),
            _ => full.cyan(),
        },
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
                " ".green(),
                value,
                width = max_label_width
            )
        })
        .collect()
}

fn get_colorbar() -> String {
    use crossterm::style::Stylize;
    let first_blocks = ["░", "▒", "▓"];
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
        d if d.contains("macos") => "macos.svg",
        d if d.contains("manjaro") => "manjaro.svg",
        d if d.contains("mint") => "mint.svg",
        d if d.contains("nixos") => "nixos.svg",
        d if d.contains("obsidian") => "obsidian.svg",
        d if d.contains("popos") => "popos.svg",
        d if d.contains("ubuntu") => "ubuntu.svg",
        d if d.contains("venom") => "venom.svg",
        d if d.contains("windows") => "windows.svg",
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

    let conf = ViuerConfig {
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

fn display_custom_logo(image_path: &str, dot_position: usize, logo_config: &LogoConfig) {
    let default_width = logo_config.width.unwrap_or(35);
    let logo_x = (dot_position as u16).saturating_sub((default_width / 2) as u16);

    const DEFAULT_MAX_WIDTH: u32 = 35;
    const DEFAULT_MAX_HEIGHT: u32 = 18;

    let conf = ViuerConfig {
        width: Some(logo_config.width.unwrap_or(DEFAULT_MAX_WIDTH)),
        height: Some(logo_config.height.unwrap_or(DEFAULT_MAX_HEIGHT)),
        x: logo_x,
        y: 2,
        absolute_offset: true,
        transparent: true,
        ..Default::default()
    };

    // Try to display the custom image
    let path = PathBuf::from(image_path);
    if path.exists() {
        let _ = print_from_file(&path, &conf);
    } else {
        eprintln!("Warning: Custom logo not found at: {}", image_path);
    }
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
