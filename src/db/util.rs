use crate::prelude::*;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
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
