use std::error::Error;
use tokio::time::{Duration, interval};
use crate::scrape::scrapers::youtube::scrape_youtube_channel;

pub async fn schedule_scrapers() -> Result<(), Box<dyn Error>> {
    let mut interval = interval(Duration::from_secs(7200));
    
    loop {
        interval.tick().await;
        run_scrapers().await?;
    }
}

pub async fn run_scrapers() -> Result<(), Box<dyn Error>> {
    scrape_youtube_channel("UCNye-wNBqNL5ZzHSJj3l8Bg").await?;
    
    Ok(())
}