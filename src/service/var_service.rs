use crate::prelude::*;
use anyhow::anyhow;
use redis::Client;
use std::env::var;

pub async fn get_docker_volume() -> Result<String> {
    match var("DOCKER_VOLUME") {
        Ok(volume) => match volume.is_empty() {
            true => {
                let err = "DOCKER_VOLUME is empty";
                tracing::error!(err);
                Err(anyhow!(err))
            }
            false => Ok(volume),
        },
        Err(e) => {
            let err = format!("DOCKER_VOLUME not found in environment: {:?}", e);
            tracing::error!(err);
            Err(anyhow!(err))
        }
    }
}

pub async fn get_redis_client() -> Result<Client> {
    let redis_password = match var("REDIS_PASSWORD") {
        Ok(password) => match password.is_empty() {
            true => {
                let err = "REDIS_PASSWORD is empty";
                tracing::error!(err);
                return Err(anyhow!(err));
            }
            false => password,
        },
        Err(e) => {
            let err = format!("REDIS_PASSWORD not found in environment: {:?}", e);
            tracing::error!(err);
            return Err(anyhow!(err));
        }
    };

    let redis_endpoint = match var("REDIS_ENDPOINT") {
        Ok(endpoint) => match endpoint.is_empty() {
            true => {
                let err = "REDIS_ENDPOINT is empty";
                tracing::error!(err);
                return Err(anyhow!(err));
            }
            false => endpoint,
        },
        Err(e) => {
            let err = format!("REDIS_ENDPOINT not found in environment: {:?}", e);
            tracing::error!(err);
            return Err(anyhow!(err));
        }
    };

    Ok(Client::open(format!(
        "rediss://default:{}@{}",
        redis_password, redis_endpoint
    ))?)
}

pub async fn get_youtube_api_key() -> Result<Option<String>> {
    match var("YOUTUBE_API_KEY") {
        Ok(api_key) => match api_key.is_empty() {
            true => {
                tracing::info!("YOUTUBE_API_KEY is empty");
                Ok(None)
            }
            false => Ok(Some(api_key)),
        },
        Err(e) => {
            tracing::info!("YOUTUBE_API_KEY not found in environment: {}", e);
            Ok(None)
        }
    }
}

pub async fn get_youtube_channel_ids() -> Result<Option<String>> {
    match var("YOUTUBE_CHANNEL_IDS") {
        Ok(channel_ids) => match channel_ids.is_empty() {
            true => {
                tracing::info!("YOUTUBE_CHANNEL_IDS is empty");
                Ok(None)
            }
            false => Ok(Some(channel_ids)),
        },
        Err(e) => {
            tracing::info!("YOUTUBE_CHANNEL_IDS not found in environment: {}", e);
            Ok(None)
        }
    }
}
