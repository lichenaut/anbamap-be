extern crate redis;
use crate::prelude::*;
use crate::util::var_service::get_redis_client;
use redis::Commands;
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

pub async fn update_db(media: Vec<(String, String, String, Vec<String>)>) -> Result<()> {
    let client = get_redis_client().await?;
    let mut connection = client.get_connection()?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let keys: HashSet<String> = connection.keys("*")?;
    for key in &keys {
        let timestamp: u64 = connection.hget(key, "timestamp")?;
        if now - timestamp > 60 * 60 * 24 * 7 {
            connection.del(key)?;
        }
    }

    for (url, title, description, regions) in media {
        if keys.contains(&url) {
            continue;
        }

        connection.hset(&url, "timestamp", now)?;
        connection.hset(&url, "title", title)?;
        connection.hset(&url, "body", description)?;
        connection.hset(url, "regions", regions.join(","))?;
    }

    Ok(())
}
