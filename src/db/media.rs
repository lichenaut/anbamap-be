use super::util::get_db_pool;
use crate::{prelude::*, service::var_service::get_age_limit};
use sqlx::Executor;
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub async fn update_media_db(
    docker_volume: &str,
    media: Vec<(String, String, String, Vec<String>)>,
) -> Result<()> {
    let db_path = format!("{}/media_db.sqlite", docker_volume);
    let db_path = Path::new(&db_path);

    let pool = get_db_pool(db_path).await?;
    pool.execute(
        "CREATE TABLE IF NOT EXISTS urls (
            url TEXT PRIMARY KEY,
            timestamp INTEGER,
            title TEXT,
            body TEXT
        )",
    )
    .await?;
    pool.execute(
        "CREATE TABLE IF NOT EXISTS url_regions (
            url TEXT,
            region_code TEXT,
            PRIMARY KEY (url, region_code),
            FOREIGN KEY (url) REFERENCES urls (url)
        )",
    )
    .await?;

    let now: i32 = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .try_into()?;
    sqlx::query("DELETE FROM urls WHERE timestamp < ?")
        .bind(now - get_age_limit().await?)
        .execute(&pool)
        .await?;

    for (url, title, body, regions) in &media {
        sqlx::query(
            "INSERT OR REPLACE INTO urls (url, timestamp, title, body) VALUES (?, ?, ?, ?)",
        )
        .bind(url)
        .bind(now)
        .bind(title)
        .bind(body)
        .execute(&pool)
        .await?;

        for region in regions {
            sqlx::query("INSERT OR REPLACE INTO url_regions (url, region_code) VALUES (?, ?)")
                .bind(url)
                .bind(region)
                .execute(&pool)
                .await?;
        }
    }

    Ok(())
}
