use chrono::Utc;
use serde::Serialize;

use crate::api::{UsageResponse, WindowSnapshot};
use crate::pace::{Stage, UsagePace};

#[derive(Debug, Serialize)]
pub struct JsonOutput {
    pub fetched_at: String,
    pub fetched_at_unix: i64,
    pub plan: Option<String>,
    pub credits: Option<CreditsOutput>,
    pub windows: WindowsOutput,
    pub pace: Option<PaceOutput>,
}

#[derive(Debug, Serialize)]
pub struct CreditsOutput {
    pub has_credits: bool,
    pub unlimited: bool,
    pub balance: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct WindowsOutput {
    pub five_hour: Option<WindowOutput>,
    pub weekly: Option<WindowOutput>,
}

#[derive(Debug, Serialize)]
pub struct WindowOutput {
    pub used_percent: i64,
    pub remaining_percent: i64,
    pub reset_at: i64,
    pub reset_in_seconds: i64,
    pub window_seconds: i64,
    pub window_minutes: i64,
}

#[derive(Debug, Serialize)]
pub struct PaceOutput {
    pub source_window: String,
    pub stage: String,
    pub delta_percent: f64,
    pub expected_used_percent: f64,
    pub actual_used_percent: f64,
    pub eta_seconds: Option<i64>,
    pub will_last_to_reset: bool,
}

impl JsonOutput {
    pub fn from_response(response: &UsageResponse) -> Self {
        let now = Utc::now();
        let now_ts = now.timestamp();

        let five_hour = response
            .rate_limit
            .as_ref()
            .and_then(|rl| rl.primary_window.as_ref())
            .map(|window| window_output(window, now_ts));

        let weekly = response
            .rate_limit
            .as_ref()
            .and_then(|rl| rl.secondary_window.as_ref())
            .map(|window| window_output(window, now_ts));

        let pace = build_pace(response, now);

        JsonOutput {
            fetched_at: now.to_rfc3339(),
            fetched_at_unix: now_ts,
            plan: response.plan_type.as_ref().map(|plan| plan.to_string()),
            credits: response.credits.as_ref().map(|credits| CreditsOutput {
                has_credits: credits.has_credits.unwrap_or(false),
                unlimited: credits.unlimited.unwrap_or(false),
                balance: credits.balance,
            }),
            windows: WindowsOutput { five_hour, weekly },
            pace,
        }
    }
}

fn window_output(window: &WindowSnapshot, now_ts: i64) -> WindowOutput {
    let used = window.used_percent.clamp(0, 100);
    WindowOutput {
        used_percent: used,
        remaining_percent: 100i64.saturating_sub(used),
        reset_at: window.reset_at,
        reset_in_seconds: (window.reset_at - now_ts).max(0),
        window_seconds: window.limit_window_seconds,
        window_minutes: window.limit_window_seconds / 60,
    }
}

fn build_pace(response: &UsageResponse, now: chrono::DateTime<Utc>) -> Option<PaceOutput> {
    let rate_limit = response.rate_limit.as_ref()?;

    if let Some(weekly) = rate_limit.secondary_window.as_ref() {
        if let Some(pace) = UsagePace::from_window(weekly, now, 10080) {
            return Some(to_pace_output(&pace, "weekly"));
        }
    }

    if let Some(five_hour) = rate_limit.primary_window.as_ref() {
        if let Some(pace) = UsagePace::from_window(five_hour, now, 300) {
            return Some(to_pace_output(&pace, "five_hour"));
        }
    }

    None
}

fn to_pace_output(pace: &UsagePace, source_window: &str) -> PaceOutput {
    PaceOutput {
        source_window: source_window.to_string(),
        stage: stage_name(pace.stage).to_string(),
        delta_percent: pace.delta_percent,
        expected_used_percent: pace.expected_used_percent,
        actual_used_percent: pace.actual_used_percent,
        eta_seconds: pace.eta_seconds.map(|seconds| seconds as i64),
        will_last_to_reset: pace.will_last_to_reset,
    }
}

fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::OnTrack => "on_track",
        Stage::SlightlyAhead => "slightly_ahead",
        Stage::Ahead => "ahead",
        Stage::FarAhead => "far_ahead",
        Stage::SlightlyBehind => "slightly_behind",
        Stage::Behind => "behind",
        Stage::FarBehind => "far_behind",
    }
}

