extern crate redis;
use crate::prelude::*;
use crate::service::var_service::get_redis_client;
use itertools::Itertools;
use redis::Commands;
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

pub async fn update_db(media: Vec<(String, String, String, HashSet<String>)>) -> Result<()> {
    let client = get_redis_client().await?;
    let mut connection = client.get_connection()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let keys: Vec<String> = connection.keys("*")?;
    let mut keys_to_delete = Vec::new();
    for key in keys {
        let timestamp: u64 = connection.hget(&key, "timestamp")?;
        if now - timestamp > 604800 {
            keys_to_delete.push(key);
        }
    }

    if !keys_to_delete.is_empty() {
        connection.del(keys_to_delete)?;
    }

    let now = now.to_string();
    let mut pipe = redis::pipe();
    for (url, title, body, regions) in &media {
        for region in regions {
            let key = format!("{}:{}", region, url);
            pipe.cmd("HSET")
                .arg(&key)
                .arg("timestamp")
                .arg(&now)
                .arg("title")
                .arg(title)
                .arg("body")
                .arg(body)
                .arg("regions")
                .arg(regions.iter().join(","))
                .ignore();
        }
    }

    pipe.execute(&mut connection);

    Ok(())
}
