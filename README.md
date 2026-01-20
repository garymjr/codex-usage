# codex-usage

CLI to display current Codex (OpenAI) usage with 5-hour window, weekly window, and pace.

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

## Display

- **Plan**: Current subscription plan (PRO, PLUS, etc.)
- **Credits**: Account credit balance or "Unlimited"
- **5h Window**: 5-hour rolling usage limit with progress bar
- **Weekly Window**: Weekly usage limit with progress bar
- **Pace**: Usage pace indicator
  - **↑/↓**: Ahead/behind of expected usage
  - **(±X%)**: Delta from expected percentage
  - **ETA**: Time until quota at current rate

## Output

```
┌──────────────────────────────────────────────────────┐
│                     Codex Usage                      
├──────────────────────────────────────────────────────┤
│ Plan: PRO                                            
│ Credits: None                                        
├──────────────────────────────────────────────────────┤
│ 5h Window (5h) ██░░░░░░░░░░░░░░░░░░░░░░░░░░░  7% 2h 5m
├──────────────────────────────────────────────────────┤
│ Weekly Window (168h) █░░░░░░░░░░░░░░░░░░░░░  7% 6d 15h
├──────────────────────────────────────────────────────┤
│ Pace: ↑ slightly ahead (+2.2%)        ETA: 4d 10h
└──────────────────────────────────────────────────────┘
```

## Implementation

- Parses `~/.codex/auth.json` for credentials
- Fetches usage from `https://chatgpt.com/backend-api/wham/usage`
- Calculates pace based on elapsed time vs expected usage
- Runs once and exits (not a long-running CLI)
