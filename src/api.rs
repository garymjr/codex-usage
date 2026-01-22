use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::path::PathBuf;

use crate::auth::Credentials;

const DEFAULT_CHATGPT_BASE_URL: &str = "https://chatgpt.com/backend-api/";

#[derive(Debug, Clone)]
pub enum PlanType {
    Guest,
    Free,
    Go,
    Plus,
    Pro,
    FreeWorkspace,
    Team,
    Business,
    Education,
    Quorum,
    K12,
    Enterprise,
    Edu,
    Unknown(String),
}

impl<'de> serde::Deserialize<'de> for PlanType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "guest" => PlanType::Guest,
            "free" => PlanType::Free,
            "go" => PlanType::Go,
            "plus" => PlanType::Plus,
            "pro" => PlanType::Pro,
            "free_workspace" => PlanType::FreeWorkspace,
            "team" => PlanType::Team,
            "business" => PlanType::Business,
            "education" => PlanType::Education,
            "quorum" => PlanType::Quorum,
            "k12" => PlanType::K12,
            "enterprise" => PlanType::Enterprise,
            "edu" => PlanType::Edu,
            _ => PlanType::Unknown(s),
        })
    }
}

impl std::fmt::Display for PlanType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanType::Guest => write!(f, "guest"),
            PlanType::Free => write!(f, "free"),
            PlanType::Go => write!(f, "go"),
            PlanType::Plus => write!(f, "plus"),
            PlanType::Pro => write!(f, "pro"),
            PlanType::FreeWorkspace => write!(f, "free_workspace"),
            PlanType::Team => write!(f, "team"),
            PlanType::Business => write!(f, "business"),
            PlanType::Education => write!(f, "education"),
            PlanType::Quorum => write!(f, "quorum"),
            PlanType::K12 => write!(f, "k12"),
            PlanType::Enterprise => write!(f, "enterprise"),
            PlanType::Edu => write!(f, "edu"),
            PlanType::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WindowSnapshot {
    #[serde(rename = "used_percent")]
    pub used_percent: i64,
    #[serde(rename = "reset_at")]
    pub reset_at: i64,
    #[serde(rename = "limit_window_seconds")]
    pub limit_window_seconds: i64,
    #[serde(default)]
    #[serde(rename = "reset_after_seconds")]
    #[allow(dead_code)]
    pub reset_after_seconds: Option<i64>,
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RateLimitDetails {
    #[serde(rename = "primary_window")]
    pub primary_window: Option<WindowSnapshot>,
    #[serde(rename = "secondary_window")]
    pub secondary_window: Option<WindowSnapshot>,
    #[serde(default)]
    #[allow(dead_code)]
    pub allowed: Option<bool>,
    #[serde(default)]
    #[allow(dead_code)]
    pub limit_reached: Option<bool>,
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreditDetails {
    #[serde(rename = "has_credits")]
    pub has_credits: Option<bool>,
    pub unlimited: Option<bool>,
    #[serde(deserialize_with = "deserialize_balance")]
    pub balance: Option<f64>,
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

fn deserialize_balance<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Deserialize;

    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        Some(serde_json::Value::Number(n)) => Ok(n.as_f64()),
        Some(serde_json::Value::String(s)) => Ok(s.parse::<f64>().ok()),
        _ => Ok(None),
    }
}

#[derive(Debug, Deserialize)]
pub struct UsageResponse {
    #[serde(rename = "plan_type")]
    pub plan_type: Option<PlanType>,
    #[serde(rename = "rate_limit")]
    pub rate_limit: Option<RateLimitDetails>,
    pub credits: Option<CreditDetails>,
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

pub struct UsageFetcher {
    client: Client,
    base_url: String,
}

impl UsageFetcher {
    pub fn new() -> Self {
        let base_url = Self::resolve_base_url();
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn fetch_usage(&self, credentials: &Credentials) -> Result<UsageResponse> {
        let url = self.build_usage_url();
        let mut request = self.client.get(&url);

        request = request
            .header(
                "Authorization",
                format!("Bearer {}", credentials.access_token),
            )
            .header("User-Agent", "codex-usage")
            .header("Accept", "application/json")
            .timeout(std::time::Duration::from_secs(30));

        if let Some(ref account_id) = credentials.account_id {
            request = request.header("ChatGPT-Account-Id", account_id);
        }

        let response = request.send().await.context("Request failed")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read response body")?;

        match status.as_u16() {
            200..=299 => serde_json::from_str(&body)
                .with_context(|| format!("Failed to parse response: {}", body)),
            401 | 403 => Err(anyhow!(
                "Unauthorized: Token expired or invalid. Run `codex` to re-authenticate."
            )),
            code => Err(anyhow!("API error {}: {}", code, body)),
        }
    }

    fn build_usage_url(&self) -> String {
        let path = if self.base_url.contains("/backend-api") {
            "/wham/usage"
        } else {
            "/api/codex/usage"
        };
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn resolve_base_url() -> String {
        if let Some(contents) = Self::load_config_contents() {
            if let Some(parsed) = Self::parse_chatgpt_base_url(&contents) {
                return Self::normalize_base_url(&parsed);
            }
        }
        DEFAULT_CHATGPT_BASE_URL.to_string()
    }

    fn load_config_contents() -> Option<String> {
        let config_path = Self::get_config_path()?;
        std::fs::read_to_string(&config_path).ok()
    }

    fn get_config_path() -> Option<PathBuf> {
        let codex_home = std::env::var("CODEX_HOME")
            .ok()
            .filter(|s| !s.trim().is_empty());

        let base = if let Some(home) = codex_home {
            PathBuf::from(home)
        } else {
            dirs::home_dir()?.join(".codex")
        };

        Some(base.join("config.toml"))
    }

    fn parse_chatgpt_base_url(contents: &str) -> Option<String> {
        for line in contents.lines() {
            let line = line.split('#').next().unwrap_or(line).trim();
            if line.is_empty() {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                if key == "chatgpt_base_url" {
                    let mut value = value.trim();
                    if (value.starts_with('"') && value.ends_with('"'))
                        || (value.starts_with('\'') && value.ends_with('\''))
                    {
                        value = &value[1..value.len() - 1];
                    }
                    return Some(value.trim().to_string());
                }
            }
        }
        None
    }

    fn normalize_base_url(url: &str) -> String {
        let mut normalized = url.trim().trim_end_matches('/').to_string();

        if normalized.is_empty() {
            normalized = DEFAULT_CHATGPT_BASE_URL.to_string();
        }

        if (normalized.starts_with("https://chatgpt.com")
            || normalized.starts_with("https://chat.openai.com"))
            && !normalized.contains("/backend-api")
        {
            normalized.push_str("/backend-api");
        }

        normalized
    }
}

impl Default for UsageFetcher {
    fn default() -> Self {
        Self::new()
    }
}

pub fn format_reset_time(timestamp: i64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(timestamp, 0)
        .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());
    let now = Utc::now();
    let duration = dt.signed_duration_since(now);

    if duration.num_hours() > 24 {
        let days = duration.num_days();
        let hours = duration.num_hours() % 24;
        if hours > 0 {
            format!("{}d {}h", days, hours)
        } else {
            format!("{}d", days)
        }
    } else if duration.num_minutes() > 60 {
        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;
        if minutes > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}h", hours)
        }
    } else {
        format!("{}m", duration.num_minutes())
    }
}
