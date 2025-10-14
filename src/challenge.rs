use chrono::{DateTime, Duration, Utc};
use crossterm::style::Stylize;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn visual_width(s: &str) -> usize {
    s.chars().filter(|c| !c.is_control()).count()
}

fn format_challenge_box(items: Vec<(&str, String)>) -> String {
    let padding_left = " ".repeat(20);
    // Find width of longest label to align dot separators
    let max_label_width = items
        .iter()
        .map(|(label, _)| label.len())
        .max()
        .unwrap_or(0);
    // Format each line of text that will be inside box
    let formatted_lines: Vec<String> = items
        .iter()
        .map(|(label, value)| {
            format!(
                " {: >width$} {} {}",
                label,
                "•".green(),
                value,
                width = max_label_width
            )
        })
        .collect();
    // Find width of longest formatted line to draw the box
    let max_content_width = formatted_lines
        .iter()
        .map(|s| visual_width(s))
        .max()
        .unwrap_or(0);

    // Build final string with box-drawn characters
    let mut output = String::new();
    let top_border = format!("╭{}╮", "─".repeat(max_content_width + 2));
    let bottom_border = format!("╰{}╯", "─".repeat(max_content_width + 2));

    output.push_str(&format!("{}{}\n", padding_left, top_border));

    for line in formatted_lines {
        let line_padding_right = " ".repeat(max_content_width - visual_width(&line));
        output.push_str(&format!(
            "{}│ {}{} │\n",
            padding_left, line, line_padding_right
        ));
    }

    output.push_str(&format!("{}{}\n", padding_left, bottom_border));
    output
}

pub fn run_challenge_countdown(years: i64, months: i64) {
    let metadata = fs::metadata("/").ok();
    let install_time = metadata
        .and_then(|m| m.modified().ok())
        .unwrap_or(UNIX_EPOCH);

    // Converting SystemTime to Chrono DataTime
    let install_dt: DateTime<Utc> = install_time.into();
    let now_dt: DateTime<Utc> = SystemTime::now().into();

    // Calculate the target date
    let target_days_in_years = Duration::days(365 * years);
    let total_days_in_months = (months as f64 * 30.44).round() as i64;
    let total_challenge_days = total_days_in_years + total_days_in_months;
    let target_dt = install_dt + Duration::days(total_challenge_days);

    let mut goal_string = String::new();
    if years > 0 {
        goal_string.push_str(&format!("{} year(s)", years));
    }
    if months > 0 {
        if !goal_string.is_empty() {
            goal_string.push_str(", ");
        }
        goal_string.push_str(&format!("{} month(s)", months));
    }

    // build output content
    let days_old = now_dt.signed_duration_since(install_dt).num_days();

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
        ))
    }

    println!("{}", format_challenge_box(info_items));
}
