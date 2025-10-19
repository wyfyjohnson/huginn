use chrono::{DateTime, Duration, Utc};
use crossterm::style::Stylize;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn draw_challenge_progress(percentage: i32, size: usize) -> String {
    let filled = (percentage * size as i32 / 100) as usize;
    let full = "━".repeat(filled);
    let empty = "━".repeat(size.saturating_sub(filled));

    let colored_full = match percentage {
        90..=100 => full.green(),
        70..=89 => full.dark_green(),
        50..=69 => full.dark_yellow(),
        30..=49 => full.dark_cyan(),
        _ => full.cyan(),
    };

    format!("{}{}", colored_full, empty.dark_grey())
}

pub fn run_challenge_countdown(years: i64, months: i64) {
    use crossterm::{cursor, execute};
    use std::io;

    let metadata = fs::metadata("/").ok();
    let install_time = metadata
        .and_then(|m| m.modified().ok())
        .unwrap_or(UNIX_EPOCH);

    let install_dt: DateTime<Utc> = install_time.into();
    let now_dt: DateTime<Utc> = SystemTime::now().into();

    let days_from_years = 365 * years;
    let days_from_months = (months as f64 * 30.44).round() as i64;
    let total_challenge_days = days_from_years + days_from_months;
    let target_dt = install_dt + Duration::days(total_challenge_days);

    let days_old = now_dt.signed_duration_since(install_dt).num_days();
    let progress_percentage = ((days_old as f64 / total_challenge_days as f64) * 100.0)
        .min(100.0)
        .max(0.0) as i32;

    let mut info_items = vec![
        ("Installed", install_dt.format("%Y-%m-%d").to_string()),
        ("Current Age", format!("{} days", days_old)),
    ];

    let remaining_duration = target_dt.signed_duration_since(now_dt);

    if remaining_duration.num_seconds() <= 0 {
        info_items.push(("Status", "Challenge Complete!".green().bold().to_string()));
    } else {
        let rem_days = remaining_duration.num_days();
        let rem_hours = remaining_duration.num_hours() % 24;
        info_items.push((
            "Time Left",
            format!("{} days, {} hours", rem_days, rem_hours)
                .magenta()
                .to_string(),
        ));
    }

    let max_label_width = info_items
        .iter()
        .map(|(label, _)| label.len())
        .max()
        .unwrap_or(0);

    let padding_left = 50;
    let mut current_row = 20;

    for (label, value) in info_items {
        let _ = execute!(io::stdout(), cursor::MoveTo(padding_left, current_row));
        print!(
            "{: >width$} {} {}",
            label,
            "•".green(),
            value,
            width = max_label_width
        );
        current_row += 1;
    }

    let _ = execute!(
        io::stdout(),
        cursor::MoveTo(padding_left + max_label_width as u16 - 8, current_row)
    );
    print!(
        "{:>3}% {}",
        progress_percentage,
        draw_challenge_progress(progress_percentage, 14)
    );
}
