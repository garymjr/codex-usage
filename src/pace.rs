use crate::api::WindowSnapshot;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    OnTrack,
    SlightlyAhead,
    Ahead,
    FarAhead,
    SlightlyBehind,
    Behind,
    FarBehind,
}

#[derive(Debug, Clone)]
pub struct UsagePace {
    pub stage: Stage,
    pub delta_percent: f64,
    pub expected_used_percent: f64,
    pub actual_used_percent: f64,
    pub eta_seconds: Option<f64>,
    pub will_last_to_reset: bool,
}

impl UsagePace {
    pub fn from_window(
        window: &WindowSnapshot,
        now: DateTime<Utc>,
        default_window_minutes: i64,
    ) -> Option<Self> {
        let reset_time = DateTime::from_timestamp(window.reset_at, 0)?;
        let window_minutes = (window.limit_window_seconds / 60) as i64;
        let window_minutes = if window_minutes > 0 {
            window_minutes
        } else {
            default_window_minutes
        };

        let duration_sec = window_minutes as f64 * 60.0;
        let time_until_reset = (reset_time - now).num_seconds().max(0) as f64;

        if time_until_reset > duration_sec || time_until_reset == 0.0 {
            return None;
        }

        let elapsed = (duration_sec - time_until_reset).clamp(0.0, duration_sec);
        let expected = (elapsed / duration_sec * 100.0).clamp(0.0, 100.0);
        let actual = (window.used_percent as f64).clamp(0.0, 100.0);

        if elapsed == 0.0 && actual > 0.0 {
            return None;
        }

        let delta = actual - expected;
        let stage = Self::stage_from_delta(delta);

        let (eta_seconds, will_last_to_reset) = if elapsed > 0.0 && actual > 0.0 {
            let rate = actual / elapsed;
            if rate > 0.0 {
                let remaining = (100.0 - actual).max(0.0);
                let candidate = remaining / rate;
                if candidate >= time_until_reset {
                    (None, true)
                } else {
                    (Some(candidate), false)
                }
            } else {
                (None, true)
            }
        } else if elapsed > 0.0 && actual == 0.0 {
            (None, true)
        } else {
            (None, false)
        };

        Some(UsagePace {
            stage,
            delta_percent: delta,
            expected_used_percent: expected,
            actual_used_percent: actual,
            eta_seconds,
            will_last_to_reset,
        })
    }

    fn stage_from_delta(delta: f64) -> Stage {
        let abs_delta = delta.abs();
        if abs_delta <= 2.0 {
            Stage::OnTrack
        } else if abs_delta <= 6.0 {
            if delta >= 0.0 {
                Stage::SlightlyAhead
            } else {
                Stage::SlightlyBehind
            }
        } else if abs_delta <= 12.0 {
            if delta >= 0.0 {
                Stage::Ahead
            } else {
                Stage::Behind
            }
        } else if delta >= 0.0 {
            Stage::FarAhead
        } else {
            Stage::FarBehind
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_from_delta() {
        assert_eq!(UsagePace::stage_from_delta(0.0), Stage::OnTrack);
        assert_eq!(UsagePace::stage_from_delta(2.0), Stage::OnTrack);
        assert_eq!(UsagePace::stage_from_delta(-2.0), Stage::OnTrack);
        assert_eq!(UsagePace::stage_from_delta(4.0), Stage::SlightlyAhead);
        assert_eq!(UsagePace::stage_from_delta(-4.0), Stage::SlightlyBehind);
        assert_eq!(UsagePace::stage_from_delta(10.0), Stage::Ahead);
        assert_eq!(UsagePace::stage_from_delta(-10.0), Stage::Behind);
        assert_eq!(UsagePace::stage_from_delta(20.0), Stage::FarAhead);
        assert_eq!(UsagePace::stage_from_delta(-20.0), Stage::FarBehind);
    }
}
