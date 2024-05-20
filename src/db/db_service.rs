use std::error::Error;
use tokio::time::{Duration, interval};

pub async fn schedule_scrapers() -> Result<(), Box<dyn Error>> {
    let mut interval = interval(Duration::from_secs(72000));
    
    loop {
        run_scrapers().await?;
        interval.tick().await;
    }
}

pub async fn run_scrapers() -> Result<(), Box<dyn Error>> {
    // forbes400::scrape().await?;
    // wikidata::scrape().await?;
    // wikipedia::scrape().await?;
    // youtube::scrape().await?;
    Ok(())
}