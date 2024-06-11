use crate::prelude::*;
use anyhow::anyhow;
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

pub async fn is_source_enabled(key: &str) -> Result<bool> {
    match var(key) {
        Ok(enabled) => match enabled.is_empty() {
            true => {
                tracing::info!("{key} is empty");
                Ok(false)
            }
            false => match enabled.parse::<bool>() {
                Ok(enabled) => Ok(enabled),
                Err(e) => {
                    let err = format!("Failed to parse {key}: {e}");
                    tracing::error!(err);
                    Err(anyhow!(err))
                }
            },
        },
        Err(e) => {
            tracing::info!("{key} not found in environment: {e}");
            Ok(false)
        }
    }
}

pub async fn get_substack_urls() -> Result<Option<String>> {
    match var("SUBSTACK_URLS") {
        Ok(urls) => match urls.is_empty() {
            true => {
                tracing::info!("SUBSTACK_URLS is empty");
                Ok(None)
            }
            false => Ok(Some(urls)),
        },
        Err(e) => {
            tracing::info!("SUBSTACK_URLS not found in environment: {e}");
            Ok(None)
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
            tracing::info!("YOUTUBE_API_KEY not found in environment: {e}");
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
            tracing::info!("YOUTUBE_CHANNEL_IDS not found in environment: {e}");
            Ok(None)
        }
    }
}
