mod api;
mod auth;
mod output;
mod pace;

use anyhow::Result;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    // Load credentials
    let credentials = auth::load_credentials()?;

    // Fetch usage
    let fetcher = api::UsageFetcher::new();
    let response = fetcher.fetch_usage(&credentials).await?;

    let output = output::JsonOutput::from_response(&response);
    println!("{}", serde_json::to_string(&output)?);

    Ok(())
}
