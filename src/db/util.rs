use crate::prelude::*;
use sqlx::{sqlite::SqliteConnectOptions, Executor, Row, SqlitePool};
use std::path::Path;

pub async fn get_db_pool(db_path: &Path) -> Result<SqlitePool> {
    Ok(SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .shared_cache(true)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal),
    )
    .await?)
}

pub async fn create_media_db(pool: &SqlitePool) -> Result<()> {
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

    Ok(())
}

pub async fn url_exists(pool: &SqlitePool, url: &str) -> Result<bool> {
    let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM urls WHERE url = ?)")
        .bind(url)
        .fetch_one(pool)
        .await?;

    match row.try_get::<bool, _>(0) {
        Ok(exists) => Ok(exists),
        Err(e) => Err(e.into()),
    }
}
