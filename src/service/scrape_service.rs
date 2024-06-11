use std::path::Path;

use crate::db::media::update_media_db;
use crate::db::util::{create_media_db, get_db_pool};
use crate::prelude::*;
use crate::scrape::scraper::misc::accuracy::scrape_accuracy;
use crate::scrape::scraper::misc::amnesty::scrape_amnesty;
use crate::scrape::scraper::misc::antiwar::scrape_antiwar;
use crate::scrape::scraper::{substack::scrape_substack, youtube::scrape_youtube};

pub async fn run_scrapers(docker_volume: &str) -> Result<()> {
    let db_path = format!("{}/media_db.sqlite", docker_volume);
    let db_path = Path::new(&db_path);
    let pool = get_db_pool(db_path).await?;
    create_media_db(&pool).await?;
    let mut media = Vec::new();
    scrape_accuracy(&pool, &mut media).await?;
    scrape_amnesty(&pool, &mut media).await?;
    scrape_antiwar(&pool, &mut media).await?;
    scrape_substack(&pool, &mut media).await?;
    scrape_youtube(&pool, &mut media).await?;
    update_media_db(&pool, media).await?;

    Ok(())
}
