use crate::db::media::update_media_db;
use crate::prelude::*;
use crate::scrape::scraper::youtube::scrape_youtube;

pub async fn run_scrapers(docker_volume: &str) -> Result<()> {
    let mut media = Vec::new();
    scrape_youtube(&mut media).await?;
    update_media_db(docker_volume, media).await?;

    Ok(())
}
