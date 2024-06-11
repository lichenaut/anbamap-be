use crate::db::media::update_media_db;
use crate::prelude::*;
use crate::scrape::scraper::misc::accuracy::scrape_accuracy;
use crate::scrape::scraper::misc::amnesty::scrape_amnesty;
use crate::scrape::scraper::{substack::scrape_substack, youtube::scrape_youtube};

pub async fn run_scrapers(docker_volume: &str) -> Result<()> {
    let mut media = Vec::new();
    scrape_accuracy(&mut media).await?;
    scrape_amnesty(&mut media).await?;
    scrape_substack(&mut media).await?;
    scrape_youtube(&mut media).await?;
    update_media_db(docker_volume, media).await?;

    Ok(())
}
