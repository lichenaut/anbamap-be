use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::error::Error;

pub async fn get_region_db_pool() -> Result<SqlitePool, Box<dyn Error>> {
    Ok(SqlitePool::connect_with(SqliteConnectOptions::new()
        .filename("region_db.sqlite")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .shared_cache(true)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)).await?)
}