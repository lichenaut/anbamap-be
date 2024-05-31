use crate::db::redis::update_db;
use crate::prelude::*;
use crate::scrape::scraper::youtube::scrape_youtube;

pub async fn run_scrapers() -> Result<()> {
    let mut media = Vec::new();
    scrape_youtube(&mut media).await?;
    update_db(media).await?;

    Ok(())
}
