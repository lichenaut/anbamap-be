use crate::prelude::*;
use sqlx::SqlitePool;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn update_media_db(
    pool: &SqlitePool,
    media: Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let now: i32 = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .try_into()?;
    sqlx::query("DELETE FROM urls WHERE timestamp < ?")
        .bind(now - 604800)
        .execute(pool)
        .await?;

    for (url, title, body, regions) in &media {
        if url.is_empty() || title.is_empty() || body.is_empty() || regions.is_empty() {
            continue;
        }

        sqlx::query("INSERT OR IGNORE INTO urls (url, timestamp, title, body) VALUES (?, ?, ?, ?)")
            .bind(url)
            .bind(now)
            .bind(title)
            .bind(body)
            .execute(pool)
            .await?;

        for region in regions {
            sqlx::query("INSERT OR IGNORE INTO url_regions (url, region_code) VALUES (?, ?)")
                .bind(url)
                .bind(region)
                .execute(pool)
                .await?;
        }
    }

    Ok(())
}
