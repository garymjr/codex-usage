use crate::api::{UsageResponse, WindowSnapshot, format_reset_time};
use crate::pace::UsagePace;
use colored::*;
use strip_ansi_escapes::strip;
use unicode_width::UnicodeWidthStr;

const WIDTH: usize = 74;
const TITLE_DECOR: &str = "✦";

fn visible_len(s: &str) -> usize {
    let stripped = strip(s.as_bytes());
    UnicodeWidthStr::width(String::from_utf8_lossy(&stripped).as_ref())
}

pub fn display_usage(response: &UsageResponse) {
    let title = format!(
        "{decor} {decor} {decor} CODEX USAGE MONITOR {decor} {decor} {decor}",
        decor = TITLE_DECOR
    );
    let title_colored = title.bold().bright_cyan();
    print_centered(&title_colored.to_string(), WIDTH);
    print_rule("=", WIDTH);

    if let Some(meta_line) = format_meta_line(response) {
        print_centered(&meta_line, WIDTH);
        print_rule("-", WIDTH);
    }

    let section_title = "Session-Based Usage Limits".bold();
    print_line(&section_title.to_string(), WIDTH);
    let section_subtitle = "Based on rate-limit windows from the API".dimmed();
    print_line(&section_subtitle.to_string(), WIDTH);
    print_rule("-", WIDTH);

    if let Some(rate_limit) = &response.rate_limit {
        // 5-hour window (usually primary)
        if let Some(window) = &rate_limit.primary_window {
            let label = format!("5h Window ({})", format_label_minutes(window));
            display_window_line(window, &label, WIDTH);
        }

        print_rule("-", WIDTH);

        // Weekly window (usually secondary)
        if let Some(window) = &rate_limit.secondary_window {
            let label = format!("Weekly Window ({})", format_label_minutes(window));
            display_window_line(window, &label, WIDTH);
        }

        print_rule("-", WIDTH);

        // Pace for weekly window
        if let Some(window) = &rate_limit.secondary_window {
            if let Some(pace) = UsagePace::from_window(window, chrono::Utc::now(), 10080) {
                let reset_label_width = reset_label_width(window);
                display_pace_line(&pace, WIDTH, reset_label_width);
            } else if let Some(primary) = &rate_limit.primary_window {
                if let Some(pace) = UsagePace::from_window(primary, chrono::Utc::now(), 300) {
                    let reset_label_width = reset_label_width(primary);
                    display_pace_line(&pace, WIDTH, reset_label_width);
                }
            }
        }
    } else {
        print_line("No rate-limit data available.", WIDTH);
    }

    print_rule("=", WIDTH);
}

fn print_rule(ch: &str, width: usize) {
    println!("{}", ch.repeat(width));
}

fn format_label_minutes(window: &WindowSnapshot) -> String {
    let minutes = window.limit_window_seconds / 60;
    if minutes >= 60 {
        let hours = minutes / 60;
        let mins = minutes % 60;
        if mins > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}h", hours)
        }
    } else {
        format!("{}m", minutes)
    }
}

fn display_window_line(window: &WindowSnapshot, label: &str, width: usize) {
    let used = window.used_percent.clamp(0, 100);
    let remaining = 100i64.saturating_sub(used);
    let reset = format_reset_time(window.reset_at);
    let status_color = if remaining <= 10 {
        "red"
    } else if remaining <= 30 {
        "yellow"
    } else {
        "green"
    };

    let indicator = apply_color("●", status_color);
    let label_colored = label.bold();
    let percent_str = format!("{:>3}%", remaining);
    let percent_colored = apply_color(&percent_str, status_color);
    let reset_colored = format!("reset {}", reset).dimmed();

    let label_part = format!("{} {}", indicator, label_colored);
    let bar_width = width
        .saturating_sub(visible_len(&label_part))
        .saturating_sub(1)
        .saturating_sub(visible_len(&percent_colored.to_string()))
        .saturating_sub(1)
        .saturating_sub(visible_len(&reset_colored.to_string()))
        .saturating_sub(1)
        .max(10);

    let fill_width = ((remaining as f64 / 100.0) * bar_width as f64) as usize;
    let fill = fill_width.min(bar_width);

    let filled = "█".repeat(fill);
    let empty = "░".repeat(bar_width.saturating_sub(fill));
    let bar = format!(
        "{}{}",
        apply_color(&filled, status_color),
        apply_color(&empty, "white")
    );

    let line = format!(
        "{} {} {} {}",
        label_part, bar, percent_colored, reset_colored
    );
    print_line(&line, width);
}

fn display_pace_line(pace: &UsagePace, width: usize, reset_label_width: usize) {
    let stage_color = match pace.stage {
        crate::pace::Stage::OnTrack => "green",
        crate::pace::Stage::SlightlyAhead
        | crate::pace::Stage::Ahead
        | crate::pace::Stage::FarAhead => "cyan",
        crate::pace::Stage::SlightlyBehind => "yellow",
        crate::pace::Stage::Behind | crate::pace::Stage::FarBehind => "red",
    };

    let emoji = pace.stage_emoji();
    let stage_desc = pace.stage_description();
    let delta = pace.format_delta();
    let eta_val = pace.format_eta();

    // Build the content string (excluding padding)
    let part1_colored = format!(
        "Pace: {} {} ({})",
        emoji,
        apply_color(stage_desc, stage_color),
        apply_color(&delta, stage_color)
    );
    let part2_colored = apply_color(&format!("ETA: {}", eta_val), "white");

    let visible_part1 = visible_len(&part1_colored);
    let visible_part2 = visible_len(&part2_colored);

    let reset_start = width.saturating_sub(reset_label_width);
    let max_start = width.saturating_sub(visible_part2);
    let min_start = visible_part1.saturating_add(1);
    let start = reset_start.min(max_start).max(min_start.min(max_start));
    let padding = start.saturating_sub(visible_part1);

    let inner = format!(
        "{}{}{}",
        part1_colored,
        " ".repeat(padding),
        part2_colored
    );
    print_line(&inner, width);
}

fn reset_label_width(window: &WindowSnapshot) -> usize {
    let reset = format_reset_time(window.reset_at);
    let reset_label = format!("reset {}", reset);
    visible_len(&reset_label)
}

fn apply_color(text: &str, color: &str) -> ColoredString {
    match color {
        "red" => text.red(),
        "green" => text.green(),
        "yellow" => text.yellow(),
        "cyan" => text.cyan(),
        "blue" => text.blue(),
        "magenta" => text.magenta(),
        "white" => text.white(),
        _ => text.normal(),
    }
}

fn print_centered(line: &str, width: usize) {
    let visible = visible_len(line);
    let padding_left = (width.saturating_sub(visible)) / 2;
    let padding_right = width.saturating_sub(visible).saturating_sub(padding_left);
    println!(
        "{}{}{}",
        " ".repeat(padding_left),
        line,
        " ".repeat(padding_right)
    );
}

fn print_line(line: &str, width: usize) {
    let padding = width.saturating_sub(visible_len(line));
    println!("{}{}", line, " ".repeat(padding));
}

fn format_meta_line(response: &UsageResponse) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();

    if let Some(plan) = &response.plan_type {
        let plan_str = plan.to_string().to_uppercase();
        parts.push(format!("plan: {}", plan_str).green().to_string());
    }

    if let Some(credits) = &response.credits {
        let credit_info = if credits.has_credits.unwrap_or(false) {
            if credits.unlimited.unwrap_or(false) {
                "Unlimited".to_string()
            } else if let Some(balance) = &credits.balance {
                format!("${:.2}", balance)
            } else {
                "None".to_string()
            }
        } else {
            "None".to_string()
        };
        parts.push(format!("credits: {}", credit_info).green().to_string());
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!("[ {} ]", parts.join(" | ")))
    }
}
