# codex-usage

CLI that returns current Codex (OpenAI) usage as JSON, including 5-hour and weekly windows plus pace data.

## Usage

```bash
cargo run --release
./target/release/codex-usage
```

## Auth

Reads credentials from `~/.codex/auth.json`. Supports:

- OAuth tokens (`access_token`, `refresh_token`, `account_id`)
- `OPENAI_API_KEY` format

Respects `CODEX_HOME` environment variable for custom codex directory.

## Output

```json
{
  "fetched_at": "2026-03-04T22:00:00+00:00",
  "fetched_at_unix": 1772661600,
  "plan": "pro",
  "credits": {
    "has_credits": false,
    "unlimited": false,
    "balance": null
  },
  "windows": {
    "five_hour": {
      "used_percent": 22,
      "remaining_percent": 78,
      "reset_at": 1772672400,
      "reset_in_seconds": 10800,
      "window_seconds": 18000,
      "window_minutes": 300
    },
    "weekly": {
      "used_percent": 44,
      "remaining_percent": 56,
      "reset_at": 1773266400,
      "reset_in_seconds": 604800,
      "window_seconds": 604800,
      "window_minutes": 10080
    }
  },
  "pace": {
    "source_window": "weekly",
    "stage": "on_track",
    "delta_percent": 1.2,
    "expected_used_percent": 42.8,
    "actual_used_percent": 44.0,
    "eta_seconds": null,
    "will_last_to_reset": true
  }
}
```

## tmux + jq

Use a short formatter in `~/.tmux.conf`:

```tmux
set -g status-interval 60
set -g status-right '#(~/Developer/codex-usage/target/release/codex-usage | jq -r "\"5h:\(.windows.five_hour.remaining_percent // \"?\")% wk:\(.windows.weekly.remaining_percent // \"?\")%\"")'
```

## Implementation

- Parses `~/.codex/auth.json` for credentials
- Fetches usage from `https://chatgpt.com/backend-api/wham/usage`
- Returns JSON with normalized fields for plan, credits, windows, and pace
