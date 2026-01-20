use crate::api::{format_reset_time, UsageResponse, WindowSnapshot};
use crate::pace::UsagePace;
use colored::*;
use strip_ansi_escapes::strip;
use unicode_width::UnicodeWidthStr;

const BOX_H: &str = "─";
const BOX_V: &str = "│";
const BOX_TL: &str = "┌";
const BOX_TR: &str = "┐";
const BOX_BL: &str = "└";
const BOX_BR: &str = "┘";
const BOX_L: &str = "├";
const BOX_R: &str = "┤";

const WIDTH: usize = 54;

fn visible_len(s: &str) -> usize {
    let stripped = strip(s.as_bytes());
    UnicodeWidthStr::width(String::from_utf8_lossy(&stripped).as_ref())
}

pub fn display_usage(response: &UsageResponse) {
    let title = "Codex Usage";
    let title_colored = title.bold().bright_cyan();
    let content_width = visible_len(&title_colored.to_string());
    let padding_left = (WIDTH - content_width) / 2;
    let padding_right = WIDTH - content_width - padding_left;

    println!("{}{}{}", BOX_TL, BOX_H.repeat(WIDTH), BOX_TR);
    println!(
        "{}{}{}{}{}",
        BOX_V,
        " ".repeat(padding_left),
        title_colored,
        " ".repeat(padding_right),
        BOX_V
    );
    println!("{}", format!("{}", format_separator()));

    if let Some(plan) = &response.plan_type {
        let plan_str = plan.to_string().to_uppercase();
        let prefix = " Plan: ";
        let plan_colored = plan_str.green();
        let visible_total = visible_len(&format!("{}{}", prefix, plan_str));
        let padding = WIDTH.saturating_sub(visible_total);
        println!(
            "{}{}{}{}{}",
            BOX_V,
            prefix,
            plan_colored,
            " ".repeat(padding),
            BOX_V
        );
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
        let prefix = " Credits: ";
        let credit_colored = credit_info.green();
        let visible_total = visible_len(&format!("{}{}", prefix, credit_info));
        let padding = WIDTH.saturating_sub(visible_total);
        println!(
            "{}{}{}{}{}",
            BOX_V,
            prefix,
            credit_colored,
            " ".repeat(padding),
            BOX_V
        );
    }

    println!("{}", format_separator());

    if let Some(rate_limit) = &response.rate_limit {
        // 5-hour window (usually primary)
        if let Some(window) = &rate_limit.primary_window {
            let label = format!("5h Window ({})", format_label_minutes(window));
            display_window(window, &label, WIDTH);
        }

        println!("{}", format_separator());

        // Weekly window (usually secondary)
        if let Some(window) = &rate_limit.secondary_window {
            let label = format!("Weekly Window ({})", format_label_minutes(window));
            display_window(window, &label, WIDTH);
        }

        println!("{}", format_separator());

        // Pace for weekly window
        if let Some(window) = &rate_limit.secondary_window {
            if let Some(pace) = UsagePace::from_window(window, chrono::Utc::now(), 10080) {
                display_pace(&pace, WIDTH);
            } else if let Some(pace) = UsagePace::from_window(&rate_limit.primary_window.as_ref().unwrap(), chrono::Utc::now(), 300) {
                display_pace(&pace, WIDTH);
            }
        }
    }

    println!("{}{}{}", BOX_BL, BOX_H.repeat(WIDTH), BOX_BR);
}

fn format_separator() -> String {
    format!("{}{}{}", BOX_L, BOX_H.repeat(WIDTH), BOX_R)
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

fn display_window(window: &WindowSnapshot, label: &str, width: usize) {
    let used = window.used_percent;
    let remaining = 100i64.saturating_sub(used);
    let reset = format_reset_time(window.reset_at);
    let percent_str = format!("{:>3}%", remaining);
    let percent_colored = apply_color(
        &percent_str,
        if remaining <= 10 { "red" } else if remaining <= 30 { "yellow" } else { "green" },
    );

    let label_colored = label.bold();
    let visible_label = visible_len(&label_colored.to_string());
    let visible_percent = visible_len(&percent_colored.to_string());
    let visible_reset = visible_len(&reset);

    let bar_width = width
        .saturating_sub(visible_label)
        .saturating_sub(visible_percent)
        .saturating_sub(visible_reset)
        .saturating_sub(4); // leading space + 3 separators

    let fill_width = ((remaining as f64 / 100.0) * bar_width as f64) as usize;
    let fill = fill_width.min(bar_width);

    let bar_color = if remaining <= 10 { "red" } else if remaining <= 30 { "yellow" } else { "green" };
    let filled = "█".repeat(fill);
    let empty = "░".repeat(bar_width.saturating_sub(fill));

    let bar = format!(
        "{}{}",
        apply_color(&filled, bar_color),
        apply_color(&empty, "white")
    );
    let inner_base = format!(
        " {} {} {} {}",
        label_colored,
        bar,
        percent_colored,
        reset.dimmed()
    );
    let padding = width.saturating_sub(visible_len(&inner_base));
    println!("{}{}{}", BOX_V, inner_base, " ".repeat(padding) + BOX_V);
}

fn display_pace(pace: &UsagePace, width: usize) {
    let stage_color = match pace.stage {
        crate::pace::Stage::OnTrack => "green",
        crate::pace::Stage::SlightlyAhead | crate::pace::Stage::Ahead | crate::pace::Stage::FarAhead => "cyan",
        crate::pace::Stage::SlightlyBehind => "yellow",
        crate::pace::Stage::Behind | crate::pace::Stage::FarBehind => "red",
    };

    let emoji = pace.stage_emoji();
    let stage_desc = pace.stage_description();
    let delta = pace.format_delta();
    let eta_val = pace.format_eta();

    // Build the content string (excluding BOX_V and padding)
    let part1_colored = format!("Pace: {} {} ({})", emoji, apply_color(stage_desc, stage_color), apply_color(&delta, stage_color));
    let part2_colored = apply_color(&format!("ETA: {}", eta_val), "white");

    let visible_part1 = visible_len(&part1_colored);
    let visible_part2 = visible_len(&part2_colored);

    let padding = width
        .saturating_sub(1) // leading space
        .saturating_sub(visible_part1)
        .saturating_sub(1) // single separator space
        .saturating_sub(visible_part2);

    let inner = format!(
        " {} {}{}",
        part1_colored,
        " ".repeat(padding),
        part2_colored
    );
    let right_pad = width.saturating_sub(visible_len(&inner));
    println!("{}{}{}{}", BOX_V, inner, " ".repeat(right_pad), BOX_V);
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
