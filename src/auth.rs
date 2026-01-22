use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Credentials {
    pub access_token: String,
    pub account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthJson {
    #[serde(default)]
    tokens: Option<Tokens>,
    #[serde(rename = "OPENAI_API_KEY")]
    openai_api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Tokens {
    access_token: String,
    account_id: Option<String>,
}

pub fn load_credentials() -> Result<Credentials> {
    let auth_path = get_auth_path()?;
    let content = std::fs::read_to_string(&auth_path)
        .with_context(|| format!("Failed to read auth.json from {}", auth_path.display()))?;

    let auth: AuthJson = serde_json::from_str(&content).context("Failed to parse auth.json")?;

    if let Some(api_key) = auth.openai_api_key {
        if !api_key.trim().is_empty() {
            return Ok(Credentials {
                access_token: api_key,
                account_id: None,
            });
        }
    }

    let tokens = auth
        .tokens
        .context("No tokens found in auth.json. Run `codex` to log in.")?;

    Ok(Credentials {
        access_token: tokens.access_token,
        account_id: tokens.account_id,
    })
}

fn get_auth_path() -> Result<PathBuf> {
    let codex_home = std::env::var("CODEX_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty());

    let base = if let Some(home) = codex_home {
        PathBuf::from(home)
    } else {
        dirs::home_dir()
            .context("Could not determine home directory")?
            .join(".codex")
    };

    Ok(base.join("auth.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_path_default() {
        let path = get_auth_path().unwrap();
        assert!(path.ends_with(".codex/auth.json"));
    }
}
