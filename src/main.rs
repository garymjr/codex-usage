mod api;
mod auth;
mod display;
mod pace;

use anyhow::Result;
use colored::Colorize;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    // Load credentials
    let credentials = auth::load_credentials()?;

    // Fetch usage
    let fetcher = api::UsageFetcher::new();
    let response = fetcher.fetch_usage(&credentials).await?;

    // Display usage
    display::display_usage(&response);

    Ok(())
}
