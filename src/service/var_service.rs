use crate::prelude::*;
use anyhow::anyhow;
use std::env::var;

pub async fn get_age_limit() -> Result<i32> {
    match var("AGE_LIMIT") {
        Ok(age_limit) => match age_limit.parse::<i32>() {
            Ok(age_limit) => Ok(age_limit),
            Err(e) => {
                let err = format!("Failed to parse AGE_LIMIT to i32: {}", e);
                tracing::error!(err);
                Err(anyhow!(err))
            }
        },
        Err(_) => Ok(604800),
    }
}

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
